use std::path::{Path, PathBuf};

use serde::Serialize;

use super::LoaderKind;
use crate::core::download::{download_all, download_file, extract_zip, DownloadTask};
use crate::core::java;
use crate::core::loaders::{fabric, forge, neoforge, quilt};
use crate::core::minecraft::{assets, library, manifest, rules::TargetOs, version::VersionDetails};
use crate::error::AppError;

/// Cada etapa da instalação — usado só pra rotular o evento de
/// progresso emitido pro front, nada aqui muda o que é baixado.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallStage {
    FetchingMetadata,
    Client,
    Libraries,
    Natives,
    Assets,
    JavaRuntime,
    /// Só existe pra Forge/NeoForge: as ferramentas de deobfuscação e
    /// patch binário rodando de verdade, uma atrás da outra.
    Processors,
    /// Aplicando o preset de otimização no `options.txt` da instância
    /// — ver `core::instances::options_txt`.
    OptionsFile,
}

/// Layout do cache global compartilhado — bibliotecas, assets e
/// runtimes Java ficam aqui, não dentro de cada instância (ver
/// `InstanceStore`), pra nunca duplicar arquivo pesado entre
/// instâncias que usem a mesma versão (ou o mesmo loader).
pub struct SharedCache<'a> {
    pub root: &'a Path,
}

impl<'a> SharedCache<'a> {
    pub fn libraries_dir(&self) -> PathBuf {
        self.root.join("libraries")
    }
    pub fn client_jar_path(&self, version_id: &str) -> PathBuf {
        self.root.join("versions").join(version_id).join(format!("{version_id}.jar"))
    }
    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }
    pub fn natives_dir(&self, version_id: &str) -> PathBuf {
        self.root.join("natives").join(version_id)
    }
    pub fn runtimes_dir(&self) -> PathBuf {
        self.root.join("runtimes")
    }
}

/// Baixa tudo que uma instância precisa pra rodar. Pra Fabric/NeoForge,
/// isso é a base vanilla (client.jar, bibliotecas, natives, assets,
/// Java) mais o que o loader precisar em cima — `launch.rs` é quem
/// funde tudo na hora de montar o comando final, então aqui não
/// precisamos gerar um `VersionDetails` "fundido" artificial.
pub async fn install(
    cache: &SharedCache<'_>,
    mc_version: &str,
    loader: &LoaderKind,
    mut on_progress: impl FnMut(InstallStage, usize, usize) + Send,
) -> Result<VersionDetails, AppError> {
    let (version, java_path) = install_vanilla_base(cache, mc_version, &mut on_progress).await?;

    match loader {
        LoaderKind::Vanilla => {}
        LoaderKind::Fabric { loader_version } => {
            install_fabric_libraries(cache, mc_version, loader_version, &mut on_progress).await?;
        }
        LoaderKind::NeoForge { neoforge_version } => {
            let client_jar = cache.client_jar_path(&version.id);
            on_progress(InstallStage::Processors, 0, 1);
            neoforge::install::install(cache.root, mc_version, neoforge_version, &java_path, &client_jar, |done, total| {
                on_progress(InstallStage::Processors, done, total)
            })
            .await?;
        }
        LoaderKind::Forge { forge_version } => {
            let client_jar = cache.client_jar_path(&version.id);
            on_progress(InstallStage::Processors, 0, 1);
            forge::install::install(cache.root, mc_version, forge_version, &java_path, &client_jar, |done, total| {
                on_progress(InstallStage::Processors, done, total)
            })
            .await?;
        }
        LoaderKind::Quilt { loader_version } => {
            install_quilt_libraries(cache, mc_version, loader_version, &mut on_progress).await?;
        }
    }

    Ok(version)
}

async fn install_vanilla_base(
    cache: &SharedCache<'_>,
    mc_version: &str,
    on_progress: &mut impl FnMut(InstallStage, usize, usize),
) -> Result<(VersionDetails, PathBuf), AppError> {
    on_progress(InstallStage::FetchingMetadata, 0, 1);
    let manifest = manifest::fetch_version_manifest().await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == mc_version)
        .ok_or_else(|| AppError::NotFound(format!("versão do Minecraft \"{mc_version}\"")))?;
    let version = manifest::fetch_version_details(&entry.url).await?;
    on_progress(InstallStage::FetchingMetadata, 1, 1);

    download_file(
        &version.downloads.client.url,
        &cache.client_jar_path(&version.id),
        Some(&version.downloads.client.sha1),
    )
    .await?;
    on_progress(InstallStage::Client, 1, 1);

    let os = TargetOs::current();
    let resolved = library::resolve_libraries(&version.libraries, os);

    let library_tasks: Vec<DownloadTask> = resolved
        .classpath
        .iter()
        .map(|artifact| DownloadTask {
            url: artifact.url.clone(),
            dest: cache.libraries_dir().join(&artifact.path),
            sha1: Some(artifact.sha1.clone()),
        })
        .collect();
    download_all(library_tasks, 8, |done, total| on_progress(InstallStage::Libraries, done, total)).await?;

    let natives_root = cache.natives_dir(&version.id);
    let natives_dest = if version.is_legacy() { natives_root.clone() } else { natives_root.join("java") };
    let natives_cache = cache.root.join("natives-cache");
    let native_total = resolved.natives.len();
    for (i, native) in resolved.natives.iter().enumerate() {
        let tmp_dest = natives_cache.join(&native.artifact.sha1);
        download_file(&native.artifact.url, &tmp_dest, Some(&native.artifact.sha1)).await?;
        extract_zip(&tmp_dest, &natives_dest, &native.exclude)?;
        on_progress(InstallStage::Natives, i + 1, native_total.max(1));
    }
    if native_total == 0 {
        on_progress(InstallStage::Natives, 1, 1);
    }

    let asset_index = assets::fetch_asset_index(&version.asset_index.url).await?;
    let assets_dir = cache.assets_dir();
    let index_dest = assets_dir.join("indexes").join(format!("{}.json", version.assets));
    if let Some(parent) = index_dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&index_dest, serde_json::to_vec(&asset_index)?).await?;

    let asset_tasks = assets::build_asset_download_tasks(&asset_index, &assets_dir);
    download_all(asset_tasks, 16, |done, total| on_progress(InstallStage::Assets, done, total)).await?;

    let java_path =
        java::ensure_runtime(&cache.runtimes_dir(), version.required_java_component(), |done, total| {
            on_progress(InstallStage::JavaRuntime, done, total)
        })
        .await?;
    on_progress(InstallStage::JavaRuntime, 1, 1);

    Ok((version, java_path))
}

/// Baixa as bibliotecas próprias do Fabric Loader (schema diferente
/// das bibliotecas vanilla — ver `core::loaders::fabric`). O
/// `client.jar`/assets/Java já foram cuidados por `install_vanilla_base`.
async fn install_fabric_libraries(
    cache: &SharedCache<'_>,
    mc_version: &str,
    loader_version: &str,
    on_progress: &mut impl FnMut(InstallStage, usize, usize),
) -> Result<(), AppError> {
    let profile = fabric::fetch_profile(mc_version, loader_version).await?;

    let tasks: Vec<DownloadTask> = profile
        .libraries
        .iter()
        .filter_map(|library| {
            let path = library.maven_path()?;
            let url = library.download_url()?;
            Some(DownloadTask { url, dest: cache.libraries_dir().join(path), sha1: library.sha1.clone() })
        })
        .collect();

    download_all(tasks, 8, |done, total| on_progress(InstallStage::Libraries, done, total)).await
}

/// Baixa as bibliotecas próprias do Quilt Loader — mesmo esquema do
/// Fabric (`core::loaders::quilt`, formato de profile idêntico, é um
/// fork).
async fn install_quilt_libraries(
    cache: &SharedCache<'_>,
    mc_version: &str,
    loader_version: &str,
    on_progress: &mut impl FnMut(InstallStage, usize, usize),
) -> Result<(), AppError> {
    let profile = quilt::fetch_profile(mc_version, loader_version).await?;

    let tasks: Vec<DownloadTask> = profile
        .libraries
        .iter()
        .filter_map(|library| {
            let path = library.maven_path()?;
            let url = library.download_url()?;
            Some(DownloadTask { url, dest: cache.libraries_dir().join(path), sha1: library.sha1.clone() })
        })
        .collect();

    download_all(tasks, 8, |done, total| on_progress(InstallStage::Libraries, done, total)).await
}
