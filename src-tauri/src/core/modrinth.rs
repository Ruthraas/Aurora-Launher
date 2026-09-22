use serde::{Deserialize, Serialize};

use crate::core::download::http::{client, with_retry};
use crate::error::AppError;

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Mod,
    Modpack,
    ResourcePack,
    Shader,
    Datapack,
}

impl ProjectType {
    fn facet_value(self) -> &'static str {
        match self {
            ProjectType::Mod => "mod",
            ProjectType::Modpack => "modpack",
            ProjectType::ResourcePack => "resourcepack",
            ProjectType::Shader => "shader",
            ProjectType::Datapack => "datapack",
        }
    }
}

/// Um resultado de busca — só os campos que a UI de fato usa. A API
/// devolve bem mais coisa (`gallery`, `license`, `organization`, ...)
/// que ignoramos silenciosamente (serde não reclama de campo extra por
/// padrão).
/// A resposta da API do Modrinth vem em `snake_case`; o resto do app
/// (structs em `commands::*`) sempre serializa pro front em
/// `camelCase` — por isso o rename é assimétrico: desserializa como a
/// API manda, serializa como o front espera.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct ModrinthHit {
    pub project_id: String,
    pub slug: String,
    pub project_type: ProjectType,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    #[serde(default)]
    pub categories: Vec<String>,
    /// Versões do Minecraft que o projeto suporta, mais recente
    /// primeiro na resposta real da API.
    #[serde(default)]
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct ModrinthSearchResponse {
    pub hits: Vec<ModrinthHit>,
    pub total_hits: u64,
    pub offset: u32,
    pub limit: u32,
}

/// Busca mods/modpacks no Modrinth — API pública, sem chave/token
/// necessário. `query` vazio devolve os mais relevantes/populares (a
/// própria API já trata isso, não é um caso especial nosso).
pub async fn search(
    query: &str,
    project_type: ProjectType,
    mc_version: Option<&str>,
    offset: u32,
    limit: u32,
) -> Result<ModrinthSearchResponse, AppError> {
    let mut facets = vec![vec![format!("project_type:{}", project_type.facet_value())]];
    if let Some(version) = mc_version {
        facets.push(vec![format!("versions:{version}")]);
    }
    let facets_json = serde_json::to_string(&facets)?;

    let url = format!("{MODRINTH_API_BASE}/search");
    with_retry(|| async {
        let response = client()
            .get(&url)
            .query(&[
                ("query", query),
                ("facets", facets_json.as_str()),
                ("offset", &offset.to_string()),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    })
    .await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct ModrinthVersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub hashes: ModrinthFileHashes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthFileHashes {
    pub sha1: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModrinthDependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

/// Confirmado contra `/v2/project/<id>/version` de verdade (Iris
/// Shaders depende de Sodium): `version_id` pode vir `null` quando a
/// dependência aponta só pro projeto, sem uma versão específica —
/// nesse caso resolvemos a versão compatível na hora de instalar.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct ModrinthDependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub dependency_type: ModrinthDependencyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct ModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    pub files: Vec<ModrinthVersionFile>,
    #[serde(default)]
    pub dependencies: Vec<ModrinthDependency>,
}

/// Versões de um projeto compatíveis com `loader`+`game_version` —
/// mais recente primeiro (ordem que a própria API já devolve).
pub async fn fetch_versions(
    project_id: &str,
    loader: &str,
    game_version: &str,
) -> Result<Vec<ModrinthVersion>, AppError> {
    let url = format!("{MODRINTH_API_BASE}/project/{project_id}/version");
    let loaders_json = serde_json::to_string(&[loader])?;
    let game_versions_json = serde_json::to_string(&[game_version])?;
    with_retry(|| async {
        let response = client()
            .get(&url)
            .query(&[("loaders", loaders_json.as_str()), ("game_versions", game_versions_json.as_str())])
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    })
    .await
}

/// Todas as versões publicadas de um projeto, sem filtro de loader/
/// versão do jogo — usado pro seletor "Versão do modpack" (queremos
/// mostrar todas as opções, não só as compatíveis com algo).
pub async fn fetch_versions_unfiltered(project_id: &str) -> Result<Vec<ModrinthVersion>, AppError> {
    let url = format!("{MODRINTH_API_BASE}/project/{project_id}/version");
    with_retry(|| async { Ok(client().get(&url).send().await?.error_for_status()?.json().await?) }).await
}

/// Busca um projeto por hash do arquivo (SHA1) — usado pra reconhecer
/// mods colocados manualmente na pasta `mods/`. Confirmado contra
/// `/v2/version_file/<hash>` de verdade.
pub async fn fetch_version_by_hash(sha1: &str) -> Result<Option<ModrinthVersion>, AppError> {
    let url = format!("{MODRINTH_API_BASE}/version_file/{sha1}");
    let response = client().get(&url).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    Ok(Some(response.error_for_status()?.json().await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_version_list_with_dependency() {
        // formato real confirmado em /v2/project/YL57xq9U/version (Iris
        // Shaders, que depende do Sodium).
        let json = r#"[{
            "id": "abc123",
            "project_id": "YL57xq9U",
            "name": "Iris 1.7",
            "version_number": "1.7.0",
            "game_versions": ["1.20.1"],
            "loaders": ["fabric"],
            "files": [{
                "url": "https://cdn.modrinth.com/data/YL57xq9U/versions/abc123/iris.jar",
                "filename": "iris.jar",
                "primary": true,
                "size": 1234,
                "hashes": {"sha1": "deadbeef"}
            }],
            "dependencies": [{
                "version_id": "ryOMVRuG",
                "project_id": "AANobbMI",
                "dependency_type": "required"
            }]
        }]"#;
        let versions: Vec<ModrinthVersion> = serde_json::from_str(json).unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].files[0].hashes.sha1, "deadbeef");
        assert_eq!(versions[0].dependencies[0].project_id.as_deref(), Some("AANobbMI"));
        assert_eq!(versions[0].dependencies[0].dependency_type, ModrinthDependencyType::Required);
    }

    #[test]
    fn parses_real_search_response_shape() {
        // formato real confirmado contra api.modrinth.com/v2/search
        let json = r#"{
            "hits": [{
                "project_id": "t1tOiUHZ",
                "project_type": "modpack",
                "slug": "create_plus",
                "author": "Sshmoob",
                "title": "Create+",
                "description": "A simple pack.",
                "categories": ["adventure", "forge"],
                "versions": ["1.19.2", "1.21.1"],
                "downloads": 1308672,
                "icon_url": "https://cdn.modrinth.com/data/t1tOiUHZ/icon.webp",
                "date_created": "2024-05-04T06:47:25.077272+00:00"
            }],
            "offset": 0,
            "limit": 10,
            "total_hits": 1
        }"#;
        let response: ModrinthSearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.hits.len(), 1);
        assert_eq!(response.hits[0].project_type, ProjectType::Modpack);
        assert_eq!(response.hits[0].downloads, 1308672);
        assert_eq!(response.total_hits, 1);
    }

    #[test]
    fn facet_value_matches_api_convention() {
        assert_eq!(ProjectType::Mod.facet_value(), "mod");
        assert_eq!(ProjectType::Modpack.facet_value(), "modpack");
    }
}
