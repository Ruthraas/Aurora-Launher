use std::path::{Path, PathBuf};

use super::installer_url;
use crate::core::download::{download_all, download_file, read_zip_entry, DownloadTask};
use crate::core::loaders::installer::{build_datamap, run_processor, InstallProfile, LoaderVersionProfile};
use crate::error::AppError;

pub fn forge_root(cache_root: &Path, forge_version: &str) -> PathBuf {
    cache_root.join("forge").join(forge_version)
}

pub fn installer_path(cache_root: &Path, forge_version: &str) -> PathBuf {
    forge_root(cache_root, forge_version).join("installer.jar")
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

/// Lê o `version.json` de um instalador do Forge já baixado (usado no
/// lançamento — não precisa de rede de novo, só do que a instalação já
/// deixou em cache).
pub fn read_cached_version_profile(cache_root: &Path, forge_version: &str) -> Result<LoaderVersionProfile, AppError> {
    let installer = installer_path(cache_root, forge_version);
    let profile = read_install_profile(&installer)?;
    read_version_profile(&installer, &profile)
}

/// Resolve uma entrada de `data` do tipo `[coordenada:maven]` pro
/// caminho local do jar em `LIBRARY_DIR` — usado pros três jars extras
/// que o Forge (diferente do NeoForge) precisa no classpath do
/// lançamento (ver `install()`).
fn resolve_data_jar_path(cache_root: &Path, profile: &InstallProfile, key: &str) -> Result<PathBuf, AppError> {
    let raw = &profile
        .data
        .get(key)
        .ok_or_else(|| AppError::NotFound(format!("entrada {key} no install_profile.json")))?
        .client;
    let coordinate = raw
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| AppError::InvalidInput(format!("valor de {key} inesperado: {raw}")))?;
    let path = crate::core::loaders::maven::maven_coordinate_to_path(coordinate)
        .ok_or_else(|| AppError::InvalidInput(format!("coordenada maven inválida: {coordinate}")))?;
    Ok(cache_root.join("libraries").join(path))
}

/// Caminho local do jar "patchado" (client.jar + hooks do Forge, saído
/// dos processors) — precisa ir no classpath do lançamento.
pub fn resolve_patched_client_path(cache_root: &Path, forge_version: &str) -> Result<PathBuf, AppError> {
    let installer = installer_path(cache_root, forge_version);
    let profile = read_install_profile(&installer)?;
    resolve_data_jar_path(cache_root, &profile, "PATCHED")
}

/// Caminho local do jar "universal" — as classes do mod loader em si
/// (fmlcore/javafmllanguage entram via `libraries` do `version.json`
/// normalmente, mas as classes principais do Forge ficam aqui,
/// separadas do jar patchado — mesmo esquema do NeoForge).
pub fn resolve_universal_jar_path(cache_root: &Path, forge_version: &str) -> Result<PathBuf, AppError> {
    let installer = installer_path(cache_root, forge_version);
    let profile = read_install_profile(&installer)?;
    let library = profile
        .libraries
        .iter()
        .find(|library| library.name.ends_with(":universal"))
        .ok_or_else(|| AppError::NotFound("biblioteca \"universal\" do Forge no install_profile.json".to_string()))?;
    let path = crate::core::loaders::maven::maven_coordinate_to_path(&library.name)
        .ok_or_else(|| AppError::InvalidInput(format!("coordenada maven inválida: {}", library.name)))?;
    Ok(cache_root.join("libraries").join(path))
}

/// Caminho local do jar "client-extra" (recursos/assets do client.jar
/// vanilla reempacotados, gerados pelo processor `MC_EXTRA`) — sem
/// ele o BootstrapLauncher/ModLauncher não acha texturas/idiomas
/// embutidos no jar. Diferente do NeoForge (que só precisa de
/// patched+universal), o Forge clássico precisa desse terceiro jar —
/// confirmado comparando `install_profile.json` reais (1.16.5 e
/// 1.20.1) contra o NeoForge: só o Forge publica `MC_EXTRA` e o
/// referencia via `-DignoreList=...,client-extra,...` no `version.json`.
pub fn resolve_client_extra_jar_path(cache_root: &Path, forge_version: &str) -> Result<PathBuf, AppError> {
    let installer = installer_path(cache_root, forge_version);
    let profile = read_install_profile(&installer)?;
    resolve_data_jar_path(cache_root, &profile, "MC_EXTRA")
}

/// Detecta se o `install_profile.json` é da arquitetura "clássica" que
/// este código suporta (ModLauncher/BootstrapLauncher clássico, lendo
/// bibliotecas soltas via `-cp`/`-p`) ou da arquitetura nova ("shim" +
/// `ForgeBootstrap` lendo um client.jar em formato "bundler" que
/// contém as próprias bibliotecas embutidas) que ainda não foi
/// implementada.
///
/// `MC_EXTRA` NÃO é um sinal confiável — testado contra instaladores
/// reais e ele já sumia antes mesmo da arquitetura mudar (ex.: ausente
/// em 1.20.4, que ainda assim usa o launch clássico). O sinal real e
/// consistente (confirmado em 1.16.5/1.20.1/1.20.2 sem shim vs.
/// 1.20.4/1.21/1.21.1/26.3 com shim) é a presença de uma biblioteca
/// `:shim` na lista `libraries` do `install_profile.json`.
///
/// Detectar isso ANTES de baixar as bibliotecas evita queimar banda
/// numa instalação que vai falhar de qualquer jeito, e é o que
/// permite tentar automaticamente vários loaders/versões (ex.:
/// modpacks) sem travar uma lista de versões suportadas na mão — se a
/// arquitetura mudar de novo, tenta e devolve um erro claro, sem
/// inventar limites.
fn require_supported_architecture(profile: &InstallProfile) -> Result<(), AppError> {
    let uses_shim_architecture = profile.libraries.iter().any(|library| library.name.ends_with(":shim"));
    if uses_shim_architecture {
        return Err(AppError::InvalidInput(
            "esta versão do Forge usa uma arquitetura de lançamento nova (\"shim\"/bundler) que o Aurora Launcher ainda não suporta — tenta uma versão do Minecraft até 1.20.2"
                .to_string(),
        ));
    }
    Ok(())
}

/// Instala o Forge clássico pra `mc_version`: baixa o instalador, as
/// bibliotecas (do instalador e do `version.json`) e roda os
/// processors client em sequência — mesmo pipeline do NeoForge (ver
/// `core::loaders::installer`), só muda de onde vêm os artefatos e
/// quais jars extras entram no classpath do lançamento.
pub async fn install(
    cache_root: &Path,
    mc_version: &str,
    forge_version: &str,
    java_path: &Path,
    client_jar_path: &Path,
    mut on_progress: impl FnMut(usize, usize) + Send,
) -> Result<LoaderVersionProfile, AppError> {
    let root = forge_root(cache_root, forge_version);
    let installer = installer_path(cache_root, forge_version);
    let libraries_dir = cache_root.join("libraries");

    download_file(&installer_url(forge_version), &installer, None).await?;

    let profile = read_install_profile(&installer)?;
    require_supported_architecture(&profile)?;
    let version_profile = read_version_profile(&installer, &profile)?;

    // Algumas bibliotecas do próprio `install_profile.json` (ex.: a
    // entrada "client" do jar patchado, `net.minecraftforge:forge:<v>:client`)
    // são só um PLACEHOLDER de onde os processors vão escrever depois —
    // vêm com `downloads.artifact.url` vazio, porque não existem pra
    // baixar ainda. Confirmado num install_profile real (Forge
    // 26.3-66.0.2): baixar isso literalmente manda uma URL vazia pro
    // reqwest, que recusa com "builder error".
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

    Ok(version_profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile_with_libraries(names: &[&str]) -> InstallProfile {
        let libraries_json: Vec<String> = names.iter().map(|name| format!(r#"{{"name": "{name}"}}"#)).collect();
        let json = format!(
            r#"{{"version": "test", "json": "/version.json", "libraries": [{}], "processors": [], "data": {{}}}}"#,
            libraries_json.join(",")
        );
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn rejects_profile_with_shim_library() {
        // formato real confirmado em forge-26.3-66.0.2 e
        // forge-1.21.1-52.1.16 (arquitetura nova, sem suporte).
        let profile = profile_with_libraries(&["net.minecraftforge:forge:26.3-66.0.2:shim"]);
        assert!(require_supported_architecture(&profile).is_err());
    }

    #[test]
    fn accepts_profile_without_shim_library() {
        // formato real confirmado em forge-1.20.2-48.1.0 e mais
        // antigas (arquitetura clássica, suportada).
        let profile = profile_with_libraries(&["net.minecraftforge:forge:1.20.2-48.1.0:universal"]);
        assert!(require_supported_architecture(&profile).is_ok());
    }
}
