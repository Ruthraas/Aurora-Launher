use serde::{Deserialize, Serialize};

use crate::core::download::http::{client, with_retry};
use crate::error::AppError;

const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
/// ID fixo do jogo "Minecraft" na API do CurseForge — confirmado
/// contra a API real, é assim que toda ferramenta de terceiros
/// (launchers, docker-minecraft-server, etc.) referencia o jogo.
const MINECRAFT_GAME_ID: u32 = 432;

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
    /// `classId` da API do CurseForge — confirmado contra `GET /v1/categories?gameId=432`
    /// de verdade (`isClass: true`): 6=Mods, 4471=Modpacks, 12=Resource
    /// Packs, 6552=Shaders, 6945=Data Packs.
    fn class_id(self) -> u32 {
        match self {
            ProjectType::Mod => 6,
            ProjectType::Modpack => 4471,
            ProjectType::ResourcePack => 12,
            ProjectType::Shader => 6552,
            ProjectType::Datapack => 6945,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeLogo {
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeCategory {
    pub name: String,
}

/// Só os campos que a UI usa — a API devolve bem mais (`authors`,
/// `screenshots`, `latestFiles`, ...) que ignoramos.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeHit {
    pub id: u32,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub download_count: u64,
    pub logo: Option<CurseForgeLogo>,
    #[serde(default)]
    pub categories: Vec<CurseForgeCategory>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgePagination {
    total_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct RawSearchResponse {
    data: Vec<CurseForgeHit>,
    pagination: CurseForgePagination,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeSearchResponse {
    pub hits: Vec<CurseForgeHit>,
    pub total_hits: u64,
}

/// Busca mods/modpacks no CurseForge. Diferente do Modrinth, precisa
/// de uma API key (header `x-api-key`) — pessoal, obtida em
/// console.curseforge.com, nunca embutida no código. Sem chave
/// configurada, nem tentamos a requisição (devolve erro claro em vez
/// de um 403 confuso).
pub async fn search(
    api_key: &str,
    query: &str,
    project_type: ProjectType,
    offset: u32,
    limit: u32,
) -> Result<CurseForgeSearchResponse, AppError> {
    let url = format!("{CURSEFORGE_API_BASE}/mods/search");
    let response: RawSearchResponse = with_retry(|| async {
        let response = client()
            .get(&url)
            .header("x-api-key", api_key)
            .header("Accept", "application/json")
            .query(&[
                ("gameId", MINECRAFT_GAME_ID.to_string()),
                ("classId", project_type.class_id().to_string()),
                ("searchFilter", query.to_string()),
                ("index", offset.to_string()),
                ("pageSize", limit.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    })
    .await?;

    Ok(CurseForgeSearchResponse { hits: response.data, total_hits: response.pagination.total_count })
}

/// Nomes por engenharia reversa não valem aqui — o modLoaderType é um
/// enum numérico documentado oficialmente pela CurseForge
/// (docs.curseforge.com/rest-api): 1=Forge, 4=Fabric, 5=Quilt,
/// 6=NeoForge.
fn mod_loader_type(loader: &str) -> Option<u32> {
    match loader {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileHash {
    pub value: String,
    /// 1 = sha1, 2 = md5 (documentado pela API; só usamos sha1).
    pub algo: u32,
}

/// `relationType` também é documentado oficialmente (não confirmado
/// contra um exemplo real com dependência não-vazia — vários mods
/// testados na pesquisa não tinham nenhuma pra essa combinação de
/// versão/loader): 1=EmbeddedLibrary, 2=OptionalDependency,
/// 3=RequiredDependency, 4=Tool, 5=Incompatible, 6=Include.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileDependency {
    pub mod_id: u32,
    pub relation_type: u32,
}

pub const RELATION_TYPE_REQUIRED: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFile {
    pub id: u32,
    pub mod_id: u32,
    pub display_name: String,
    pub file_name: String,
    pub download_url: Option<String>,
    pub file_length: u64,
    #[serde(default)]
    pub hashes: Vec<CurseForgeFileHash>,
    #[serde(default)]
    pub dependencies: Vec<CurseForgeFileDependency>,
}

impl CurseForgeFile {
    pub fn sha1(&self) -> Option<&str> {
        self.hashes.iter().find(|h| h.algo == 1).map(|h| h.value.as_str())
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawFilesResponse {
    data: Vec<CurseForgeFile>,
}

/// Arquivos de um mod compatíveis com `game_version`+`loader`, mais
/// recente primeiro (ordem que a própria API já devolve). `loader`
/// desconhecido (ex.: instância vanilla) devolve lista vazia sem
/// chamar a API.
pub async fn fetch_files(
    api_key: &str,
    mod_id: u32,
    game_version: &str,
    loader: &str,
) -> Result<Vec<CurseForgeFile>, AppError> {
    let Some(loader_type) = mod_loader_type(loader) else { return Ok(Vec::new()) };

    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files");
    let response: RawFilesResponse = with_retry(|| async {
        let response = client()
            .get(&url)
            .header("x-api-key", api_key)
            .header("Accept", "application/json")
            .query(&[("gameVersion", game_version), ("modLoaderType", &loader_type.to_string())])
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    })
    .await?;

    Ok(response.data)
}

/// Arquivos compatíveis com `game_version`, SEM filtro de loader — pra
/// resourcepack/shader/datapack, que no CurseForge não têm
/// `modLoaderType` (esse filtro só faz sentido pra mod).
pub async fn fetch_files_by_version(api_key: &str, mod_id: u32, game_version: &str) -> Result<Vec<CurseForgeFile>, AppError> {
    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files");
    let response: RawFilesResponse = with_retry(|| async {
        let response = client()
            .get(&url)
            .header("x-api-key", api_key)
            .header("Accept", "application/json")
            .query(&[("gameVersion", game_version)])
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    })
    .await?;
    Ok(response.data)
}

/// Todos os arquivos de um mod/modpack, sem filtro de loader/versão do
/// jogo — usado pro seletor "Versão do modpack".
pub async fn fetch_all_files(api_key: &str, mod_id: u32) -> Result<Vec<CurseForgeFile>, AppError> {
    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files");
    let response: RawFilesResponse = with_retry(|| async {
        let response = client()
            .get(&url)
            .header("x-api-key", api_key)
            .header("Accept", "application/json")
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    })
    .await?;
    Ok(response.data)
}

/// Resolve vários `fileId` de uma vez (`POST /v1/mods/files`) — usado
/// pra instalar um modpack do CurseForge, cujo `manifest.json` só traz
/// `projectID`/`fileID`, sem URL nem hash. Uma chamada só em vez de
/// uma por mod (um pack grande facilmente tem 100+ mods).
pub async fn fetch_files_by_ids(api_key: &str, file_ids: &[u32]) -> Result<Vec<CurseForgeFile>, AppError> {
    if file_ids.is_empty() {
        return Ok(Vec::new());
    }
    let url = format!("{CURSEFORGE_API_BASE}/mods/files");
    let response: RawFilesResponse = with_retry(|| async {
        let response = client()
            .post(&url)
            .header("x-api-key", api_key)
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "fileIds": file_ids }))
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    })
    .await?;
    Ok(response.data)
}

/// Busca um mod por hash do arquivo (SHA1 em `algo: 1`, murmur2 em
/// `algo: 2`) — usado pra reconhecer mods colocados manualmente na
/// pasta `mods/`. Confirmado contra `/v1/fingerprints` de verdade
/// (esse endpoint usa o fingerprint murmur2 do CurseForge, não SHA1
/// puro — diferente do Modrinth).
pub async fn fetch_mod_by_fingerprint(api_key: &str, fingerprint: u32) -> Result<Option<CurseForgeFile>, AppError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FingerprintMatch {
        file: CurseForgeFile,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FingerprintData {
        exact_matches: Vec<FingerprintMatch>,
    }
    #[derive(Deserialize)]
    struct FingerprintResponse {
        data: FingerprintData,
    }

    let url = format!("{CURSEFORGE_API_BASE}/fingerprints");
    let response: FingerprintResponse = with_retry(|| async {
        let response = client()
            .post(&url)
            .header("x-api-key", api_key)
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "fingerprints": [fingerprint] }))
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    })
    .await?;

    Ok(response.data.exact_matches.into_iter().next().map(|m| m.file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_loader_type_matches_official_enum() {
        assert_eq!(mod_loader_type("forge"), Some(1));
        assert_eq!(mod_loader_type("fabric"), Some(4));
        assert_eq!(mod_loader_type("quilt"), Some(5));
        assert_eq!(mod_loader_type("neoforge"), Some(6));
        assert_eq!(mod_loader_type("vanilla"), None);
    }

    #[test]
    fn parses_real_files_response_shape() {
        // formato real confirmado em /v1/mods/238222/files (JEI, Fabric 1.20.1)
        let json = r#"{
            "data": [{
                "id": 8895263,
                "modId": 238222,
                "displayName": "15.59.0.212 for Fabric 1.20.1",
                "fileName": "jei-1.20.1-fabric-15.59.0.212.jar",
                "downloadUrl": "https://edge.forgecdn.net/files/8895/263/jei-1.20.1-fabric-15.59.0.212.jar",
                "fileLength": 1849781,
                "hashes": [
                    {"value": "ef251602fcf21b70b8453a93d737333f078ed5ef", "algo": 1},
                    {"value": "25b77c6319ffb9496dd92600ab37daaa", "algo": 2}
                ],
                "dependencies": []
            }]
        }"#;
        let response: RawFilesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data[0].sha1(), Some("ef251602fcf21b70b8453a93d737333f078ed5ef"));
        assert_eq!(
            response.data[0].download_url.as_deref(),
            Some("https://edge.forgecdn.net/files/8895/263/jei-1.20.1-fabric-15.59.0.212.jar")
        );
    }

    #[test]
    fn class_id_matches_api_convention() {
        // confirmado contra api.curseforge.com/v1/mods/search de verdade.
        assert_eq!(ProjectType::Mod.class_id(), 6);
        assert_eq!(ProjectType::Modpack.class_id(), 4471);
    }

    #[test]
    fn parses_real_search_response_shape() {
        // formato real confirmado contra api.curseforge.com/v1/mods/search
        let json = r#"{
            "data": [{
                "id": 1661892,
                "gameId": 432,
                "name": "!  Norwood Pack  !",
                "slug": "norwood-pack",
                "summary": "Endless Late Game Progression",
                "downloadCount": 91,
                "categories": [{"name": "Adventure and RPG"}],
                "logo": {
                    "id": 1997437,
                    "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/1997/437/256/256/x.jpg"
                }
            }],
            "pagination": {"index": 0, "pageSize": 1, "resultCount": 1, "totalCount": 3870}
        }"#;
        let response: RawSearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].download_count, 91);
        assert_eq!(response.pagination.total_count, 3870);
    }
}
