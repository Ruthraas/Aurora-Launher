use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::download::http::{client, with_retry};
use crate::core::download::DownloadTask;
use crate::error::AppError;

const RESOURCES_BASE_URL: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

pub async fn fetch_asset_index(url: &str) -> Result<AssetIndex, AppError> {
    with_retry(|| async {
        let response = client().get(url).send().await?.error_for_status()?;
        Ok(response.json().await?)
    })
    .await
}

/// Monta as tasks de download de todos os objetos de um asset index,
/// gravando no layout "hashed" (`objects/<hash[0:2]>/<hash>`) que o
/// próprio jogo espera dentro da pasta de assets compartilhada — o
/// mesmo objeto serve pra qualquer versão que referencie o mesmo hash,
/// então isso funciona como cache global automaticamente.
pub fn build_asset_download_tasks(index: &AssetIndex, assets_dir: &Path) -> Vec<DownloadTask> {
    index
        .objects
        .values()
        .map(|object| {
            let prefix = &object.hash[0..2];
            DownloadTask {
                url: format!("{RESOURCES_BASE_URL}/{prefix}/{}", object.hash),
                dest: assets_dir.join("objects").join(prefix).join(&object.hash),
                sha1: Some(object.hash.clone()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_asset_download_tasks_uses_hashed_layout() {
        let index: AssetIndex = serde_json::from_str(
            r#"{"objects": {"icons/icon_16x16.png": {"hash": "5ff04807c356f1beed0b86ccf659b44b9983e3fa", "size": 781}}}"#,
        )
        .unwrap();

        let tasks = build_asset_download_tasks(&index, &PathBuf::from("C:/aurora/assets"));
        assert_eq!(tasks.len(), 1);
        assert_eq!(
            tasks[0].url,
            "https://resources.download.minecraft.net/5f/5ff04807c356f1beed0b86ccf659b44b9983e3fa"
        );
        assert_eq!(tasks[0].dest, PathBuf::from("C:/aurora/assets/objects/5f/5ff04807c356f1beed0b86ccf659b44b9983e3fa"));
        assert_eq!(tasks[0].sha1.as_deref(), Some("5ff04807c356f1beed0b86ccf659b44b9983e3fa"));
    }
}
