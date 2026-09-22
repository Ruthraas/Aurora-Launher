use serde::{Deserialize, Serialize};

use crate::core::download::http::{client, with_retry};
use crate::core::minecraft::arguments::ArgumentEntry;
use crate::error::AppError;

const QUILT_META_URL: &str = "https://meta.quiltmc.org";

/// `meta.quiltmc.org/v3/versions/loader/<mc>` já devolve só as
/// combinações loader+hashed+intermediary compatíveis com `mc_version`
/// (igual o endpoint equivalente do Fabric) — não precisamos filtrar
/// nada, só usar `loader.version` de cada entrada. Confirmado contra a
/// API real (`/v3/versions/loader/1.20.1`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuiltLoaderEntry {
    pub loader: QuiltLoaderVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuiltLoaderVersion {
    pub version: String,
}

/// Versões do Quilt Loader compatíveis com `mc_version`, na ordem que
/// a própria API devolve (mais recente primeiro).
pub async fn fetch_loader_versions(mc_version: &str) -> Result<Vec<QuiltLoaderEntry>, AppError> {
    let url = format!("{QUILT_META_URL}/v3/versions/loader/{mc_version}");
    with_retry(|| async {
        let response = client().get(&url).send().await?.error_for_status()?;
        Ok(response.json().await?)
    })
    .await
}

/// O "profile" pronto que o Quilt gera é byte-a-byte no mesmo formato
/// do Fabric (`inheritsFrom` + bibliotecas sem `downloads.artifact`,
/// só `name`+`url`) — o Quilt é um fork do Fabric Loader, mesma
/// convenção de repositório. Confirmado contra
/// `/v3/versions/loader/<mc>/<loader>/profile/json` de verdade.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltProfile {
    pub id: String,
    pub inherits_from: String,
    pub main_class: String,
    pub arguments: QuiltArguments,
    pub libraries: Vec<QuiltLibrary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuiltArguments {
    #[serde(default)]
    pub game: Vec<ArgumentEntry>,
    #[serde(default)]
    pub jvm: Vec<ArgumentEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuiltLibrary {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
}

impl QuiltLibrary {
    pub fn maven_path(&self) -> Option<String> {
        super::maven::maven_coordinate_to_path(&self.name)
    }

    pub fn download_url(&self) -> Option<String> {
        let path = self.maven_path()?;
        Some(format!("{}/{path}", self.url.trim_end_matches('/')))
    }
}

pub async fn fetch_profile(mc_version: &str, loader_version: &str) -> Result<QuiltProfile, AppError> {
    let url = format!("{QUILT_META_URL}/v3/versions/loader/{mc_version}/{loader_version}/profile/json");
    with_retry(|| async {
        let response = client().get(&url).send().await?.error_for_status()?;
        Ok(response.json().await?)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_path_matches_real_repository_layout() {
        let library = QuiltLibrary {
            name: "org.quiltmc:quilt-loader:0.20.0-beta.9".to_string(),
            url: "https://maven.quiltmc.org/repository/release/".to_string(),
            sha1: None,
        };
        assert_eq!(
            library.maven_path().unwrap(),
            "org/quiltmc/quilt-loader/0.20.0-beta.9/quilt-loader-0.20.0-beta.9.jar"
        );
        assert_eq!(
            library.download_url().unwrap(),
            "https://maven.quiltmc.org/repository/release/org/quiltmc/quilt-loader/0.20.0-beta.9/quilt-loader-0.20.0-beta.9.jar"
        );
    }

    #[test]
    fn profile_parses_real_shape() {
        // formato real confirmado em
        // meta.quiltmc.org/v3/versions/loader/1.20.1/0.20.0-beta.9/profile/json
        let json = r#"{
            "id": "quilt-loader-0.20.0-beta.9-1.20.1",
            "inheritsFrom": "1.20.1",
            "type": "release",
            "mainClass": "org.quiltmc.loader.impl.launch.knot.KnotClient",
            "arguments": {"game": []},
            "libraries": [
                {"name": "net.fabricmc:intermediary:1.20.1", "url": "https://maven.fabricmc.net/"}
            ]
        }"#;
        let profile: QuiltProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.inherits_from, "1.20.1");
        assert_eq!(profile.main_class, "org.quiltmc.loader.impl.launch.knot.KnotClient");
        assert_eq!(profile.libraries.len(), 1);
    }
}
