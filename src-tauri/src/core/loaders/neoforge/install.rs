use std::path::{Path, PathBuf};

use super::installer_url;
use crate::core::download::{download_all, download_file, read_zip_entry, DownloadTask};
use crate::core::loaders::installer::{build_datamap, run_processor, InstallProfile, LoaderVersionProfile};
use crate::error::AppError;

pub fn neoforge_root(cache_root: &Path, neoforge_version: &str) -> PathBuf {
    cache_root.join("neoforge").join(neoforge_version)
}

pub fn installer_path(cache_root: &Path, neoforge_version: &str) -> PathBuf {
    neoforge_root(cache_root, neoforge_version).join("installer.jar")
}

fn read_install_profile(installer: &Path) -> Result<InstallProfile, AppError> {
    let bytes = read_zip_entry(installer, "install_profile.json")?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn read_version_profile(installer: &Path, profile: &InstallProfile) -> Result<LoaderVersionProfile, AppError> {
    let entry = profile.json.trim_start_matches('/');
    let bytes = read_zip_entry(installer, entry)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Lê o `version.json` de um instalador do NeoForge já baixado (usado
/// no lançamento — não precisa de rede de novo, só do que a instalação
/// já deixou em cache).
pub fn read_cached_version_profile(cache_root: &Path, neoforge_version: &str) -> Result<LoaderVersionProfile, AppError> {
    let installer = installer_path(cache_root, neoforge_version);
    let profile = read_install_profile(&installer)?;
    read_version_profile(&installer, &profile)
}

/// Caminho local do jar "patchado" (client.jar + hooks do NeoForge),
/// gerado pelos processors na instalação — precisa ir no classpath do
/// lançamento, além (não no lugar) do client.jar vanilla original.
///
/// Isso sozinho NÃO é o bastante: o jar patchado só tem o Minecraft
/// com os pontos de extensão do NeoForge, não as classes do mod loader
/// em si (`NeoForgeMod`, `ClientNeoForgeMod`, ...) — essas vêm do jar
/// "universal" (`resolve_universal_jar_path`), que também precisa
/// entrar no classpath. Sem ele: "The patched Minecraft jar is
/// missing" (mensagem enganosa do FML — o jar existe, só falta o
/// universal ao lado dele).
pub fn resolve_patched_client_path(cache_root: &Path, neoforge_version: &str) -> Result<PathBuf, AppError> {
    let installer = installer_path(cache_root, neoforge_version);
    let profile = read_install_profile(&installer)?;
    let raw = &profile
        .data
        .get("PATCHED")
        .ok_or_else(|| AppError::NotFound("entrada PATCHED no install_profile.json".to_string()))?
        .client;
    let coordinate = raw
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| AppError::InvalidInput(format!("valor de PATCHED inesperado: {raw}")))?;
    let path = crate::core::loaders::maven::maven_coordinate_to_path(coordinate)
        .ok_or_else(|| AppError::InvalidInput(format!("coordenada maven inválida: {coordinate}")))?;
    Ok(cache_root.join("libraries").join(path))
}

/// Caminho local do jar "universal" do NeoForge — as classes do mod
/// loader em si, separadas do jar patchado (ver doc acima).
pub fn resolve_universal_jar_path(cache_root: &Path, neoforge_version: &str) -> Result<PathBuf, AppError> {
    let installer = installer_path(cache_root, neoforge_version);
    let profile = read_install_profile(&installer)?;
    let library = profile
        .libraries
        .iter()
        .find(|library| library.name.ends_with(":universal"))
        .ok_or_else(|| AppError::NotFound("biblioteca \"universal\" do NeoForge no install_profile.json".to_string()))?;
    let path = crate::core::loaders::maven::maven_coordinate_to_path(&library.name)
        .ok_or_else(|| AppError::InvalidInput(format!("coordenada maven inválida: {}", library.name)))?;
    Ok(cache_root.join("libraries").join(path))
}

/// Instala o NeoForge pra `mc_version`: baixa o instalador, as
/// bibliotecas (do instalador e do `version.json`) e roda os
/// processors client (deobfuscação + patch binário) em sequência.
/// Precisa do `client.jar` vanilla e de um runtime Java já prontos
/// (ver `core::instances::install`, que orquestra tudo isso junto).
pub async fn install(
    cache_root: &Path,
    mc_version: &str,
    neoforge_version: &str,
    java_path: &Path,
    client_jar_path: &Path,
    mut on_progress: impl FnMut(usize, usize) + Send,
) -> Result<LoaderVersionProfile, AppError> {
    let root = neoforge_root(cache_root, neoforge_version);
    let installer = installer_path(cache_root, neoforge_version);
    let libraries_dir = cache_root.join("libraries");

    download_file(&installer_url(neoforge_version), &installer, None).await?;

    let profile = read_install_profile(&installer)?;
    let version_profile = read_version_profile(&installer, &profile)?;

    // Algumas bibliotecas do install_profile.json são só um
    // PLACEHOLDER de onde os processors vão escrever depois (ex.: a
    // entrada "client" do jar patchado) — vêm com
    // `downloads.artifact.url` vazio. Baixar isso literalmente manda
    // uma URL vazia pro reqwest, que recusa com "builder error" (bug
    // real confirmado no Forge clássico, mesmo install_profile format).
    let mut tasks = Vec::new();
    for library in profile.libraries.iter().chain(version_profile.libraries.iter()) {
        if let Some(artifact) = library.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            if artifact.url.is_empty() {
                continue;
            }
            tasks.push(DownloadTask {
                url: artifact.url.clone(),
                dest: libraries_dir.join(&artifact.path),
                sha1: Some(artifact.sha1.clone()),
            });
        }
    }
    download_all(tasks, 8, &mut on_progress).await?;

    let datamap = build_datamap(&profile, &root, &installer, &libraries_dir, mc_version, client_jar_path)?;

    let client_processors: Vec<_> = profile.processors.iter().filter(|p| p.runs_for_client()).collect();
    let total = client_processors.len();
    for (i, processor) in client_processors.into_iter().enumerate() {
        run_processor(processor, &datamap, &installer, &libraries_dir, &root, java_path).await?;
        on_progress(i + 1, total.max(1));
    }

    // O jar que o PROCESS_MINECRAFT_JAR gera não tem o atributo
    // `Minecraft-Dists` que o FancyModLoader passa a exigir a partir
    // da Minecraft 1.21.7 — normalmente só o Gradle do NeoForge
    // adiciona isso, não o instalador. Sem ele: "Fatal Startup Error"
    // mesmo com o jar correto.
    let patched_path = resolve_patched_client_path(cache_root, neoforge_version)?;
    crate::core::download::ensure_manifest_attribute(&patched_path, "Minecraft-Dists", "client")?;

    Ok(version_profile)
}
