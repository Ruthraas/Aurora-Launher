use serde::{Deserialize, Serialize};

use crate::core::download::http::{client, with_retry};
use crate::core::minecraft::arguments::ArgumentEntry;
use crate::error::AppError;

const FABRIC_META_URL: &str = "https://meta.fabricmc.net";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLoaderEntry {
    pub loader: FabricLoaderVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLoaderVersion {
    pub version: String,
    pub stable: bool,
}

/// Versões do Fabric Loader compatíveis com `mc_version`, mais recente
/// primeiro — é como a API já devolve.
pub async fn fetch_loader_versions(mc_version: &str) -> Result<Vec<FabricLoaderEntry>, AppError> {
    let url = format!("{FABRIC_META_URL}/v2/versions/loader/{mc_version}");
    with_retry(|| async {
        let response = client().get(&url).send().await?.error_for_status()?;
        Ok(response.json().await?)
    })
    .await
}

/// O "profile" pronto que o próprio Fabric gera no formato de launcher
/// padrão: `inheritsFrom` aponta pra versão vanilla base — client.jar,
/// assets, assetIndex e javaVersion continuam vindo de lá. Isto aqui só
/// acrescenta: main class diferente, bibliotecas extras do loader, e
/// alguns argumentos extras de jogo/JVM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricProfile {
    pub id: String,
    pub inherits_from: String,
    pub main_class: String,
    pub arguments: FabricArguments,
    pub libraries: Vec<FabricLibrary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FabricArguments {
    #[serde(default)]
    pub game: Vec<ArgumentEntry>,
    #[serde(default)]
    pub jvm: Vec<ArgumentEntry>,
}

/// Schema de biblioteca do Fabric é mais simples que o da Mojang: sem
/// `downloads.artifact`, só a coordenada Maven + a URL base do
/// repositório — o caminho do jar segue o layout padrão de repositório
/// Maven (confirmado contra `maven.fabricmc.net` de verdade).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLibrary {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
}

impl FabricLibrary {
    pub fn maven_path(&self) -> Option<String> {
        super::maven::maven_coordinate_to_path(&self.name)
    }

    pub fn download_url(&self) -> Option<String> {
        let path = self.maven_path()?;
        Some(format!("{}/{path}", self.url.trim_end_matches('/')))
    }
}

pub async fn fetch_profile(mc_version: &str, loader_version: &str) -> Result<FabricProfile, AppError> {
    let url = format!("{FABRIC_META_URL}/v2/versions/loader/{mc_version}/{loader_version}/profile/json");
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
        let library = FabricLibrary {
            name: "org.ow2.asm:asm:9.10.1".to_string(),
            url: "https://maven.fabricmc.net/".to_string(),
            sha1: None,
        };
        assert_eq!(library.maven_path().unwrap(), "org/ow2/asm/asm/9.10.1/asm-9.10.1.jar");
        assert_eq!(
            library.download_url().unwrap(),
            "https://maven.fabricmc.net/org/ow2/asm/asm/9.10.1/asm-9.10.1.jar"
        );
    }

    #[test]
    fn maven_path_handles_classifier() {
        let library = FabricLibrary {
            name: "net.fabricmc:intermediary:1.21.11:v2".to_string(),
            url: "https://maven.fabricmc.net/".to_string(),
            sha1: None,
        };
        assert_eq!(
            library.maven_path().unwrap(),
            "net/fabricmc/intermediary/1.21.11/intermediary-1.21.11-v2.jar"
        );
    }

    #[test]
    fn maven_path_rejects_malformed_coordinate() {
        let library = FabricLibrary { name: "not-a-coordinate".to_string(), url: String::new(), sha1: None };
        assert!(library.maven_path().is_none());
    }
}
