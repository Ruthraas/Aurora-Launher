use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::core::download::http::{client, with_retry};
use crate::core::minecraft::rules::TargetOs;
use crate::error::AppError;

const ALL_RUNTIMES_URL: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeManifestEntry {
    pub manifest: ManifestRef,
    pub version: RuntimeVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestRef {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeVersion {
    pub name: String,
}

/// `all.json`: SO → componente → builds disponíveis (o primeiro da
/// lista é sempre o atual).
type AllRuntimesResponse = HashMap<String, HashMap<String, Vec<RuntimeManifestEntry>>>;

/// Chave usada pelo manifesto de runtimes Java pra identificar o SO —
/// é um schema diferente do `rules[].os.name` das bibliotecas (aqui já
/// inclui a arquitetura).
fn java_runtime_os_key(os: TargetOs) -> &'static str {
    match os {
        TargetOs::Windows => "windows-x64",
        TargetOs::Linux => "linux",
        TargetOs::MacOs => "mac-os",
    }
}

pub async fn fetch_runtime_entry(
    os: TargetOs,
    component: &str,
) -> Result<Option<RuntimeManifestEntry>, AppError> {
    let all: AllRuntimesResponse = with_retry(|| async {
        let response = client().get(ALL_RUNTIMES_URL).send().await?.error_for_status()?;
        Ok(response.json().await?)
    })
    .await?;
    Ok(all
        .get(java_runtime_os_key(os))
        .and_then(|components| components.get(component))
        .and_then(|entries| entries.first())
        .cloned())
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeFilesManifest {
    pub files: HashMap<String, RuntimeFileEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RuntimeFileEntry {
    File {
        downloads: RuntimeFileDownloads,
        #[serde(default)]
        executable: bool,
    },
    Directory,
    /// Só aparece em builds Unix — sem uso hoje (Windows é o alvo
    /// atual do projeto).
    Link {
        #[serde(default)]
        target: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeFileDownloads {
    pub raw: ManifestRef,
}

pub async fn fetch_runtime_files_manifest(url: &str) -> Result<RuntimeFilesManifest, AppError> {
    with_retry(|| async {
        let response = client().get(url).send().await?.error_for_status()?;
        Ok(response.json().await?)
    })
    .await
}
