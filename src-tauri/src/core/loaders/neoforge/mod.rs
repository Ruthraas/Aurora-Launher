pub mod install;

use serde::Deserialize;

use crate::core::download::http::{client, with_retry};
use crate::error::AppError;

const MAVEN_API_BASE: &str = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
const MAVEN_RELEASES_BASE: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";

#[derive(Debug, Clone, Deserialize)]
struct VersionsResponse {
    versions: Vec<String>,
}

/// Versões do NeoForge compatíveis com `mc_version` (ex.: "1.21.11" →
/// prefixo "21.11." — é assim que o NeoForge nomeia as próprias
/// versões, sem o "1." inicial; confirmado contra a API real). Mais
/// recente primeiro.
pub async fn fetch_versions(mc_version: &str) -> Result<Vec<String>, AppError> {
    let prefix = neoforge_version_prefix(mc_version);
    let response: VersionsResponse = with_retry(|| async {
        let response = client().get(MAVEN_API_BASE).send().await?.error_for_status()?;
        Ok(response.json().await?)
    })
    .await?;

    let mut matching: Vec<String> = response.versions.into_iter().filter(|v| v.starts_with(&prefix)).collect();
    matching.reverse(); // a API devolve em ordem crescente
    Ok(matching)
}

fn neoforge_version_prefix(mc_version: &str) -> String {
    let stripped = mc_version.strip_prefix("1.").unwrap_or(mc_version);
    format!("{stripped}.")
}

pub fn installer_url(neoforge_version: &str) -> String {
    format!("{MAVEN_RELEASES_BASE}/{neoforge_version}/neoforge-{neoforge_version}-installer.jar")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_prefix_strips_leading_1_dot() {
        assert_eq!(neoforge_version_prefix("1.21.11"), "21.11.");
        assert_eq!(neoforge_version_prefix("1.20.1"), "20.1.");
    }
}
