use serde::{Deserialize, Serialize};

use super::version::VersionDetails;
use crate::core::download::http::{client, with_retry};
use crate::error::AppError;

const VERSION_MANIFEST_URL: &str = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub sha1: String,
    pub release_time: String,
}

pub async fn fetch_version_manifest() -> Result<VersionManifest, AppError> {
    with_retry(|| async {
        let response = client().get(VERSION_MANIFEST_URL).send().await?.error_for_status()?;
        Ok(response.json::<VersionManifest>().await?)
    })
    .await
}

pub async fn fetch_version_details(url: &str) -> Result<VersionDetails, AppError> {
    with_retry(|| async {
        let response = client().get(url).send().await?.error_for_status()?;
        Ok(response.json::<VersionDetails>().await?)
    })
    .await
}
