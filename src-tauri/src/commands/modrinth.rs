use crate::core::modrinth::{self, ModrinthSearchResponse, ProjectType};
use crate::error::AppResult;

/// Busca mods/modpacks no Modrinth pra popular o grid de "Descobrir".
/// `mc_version` opcional filtra pela versão do Minecraft; `offset`/
/// `limit` fazem a paginação.
#[tauri::command]
pub async fn search_modrinth(
    query: String,
    project_type: ProjectType,
    mc_version: Option<String>,
    offset: u32,
    limit: u32,
) -> AppResult<ModrinthSearchResponse> {
    Ok(modrinth::search(&query, project_type, mc_version.as_deref(), offset, limit).await?)
}
