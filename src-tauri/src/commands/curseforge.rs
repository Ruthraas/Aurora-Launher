use tauri::AppHandle;

use crate::core::curseforge::{self, CurseForgeSearchResponse, ProjectType};
use crate::core::settings::SettingsStore;
use crate::error::{AppError, AppResult};

/// Busca mods/modpacks no CurseForge pra popular o grid de "Descobrir".
/// Exige a chave da API já configurada (ver `set_curseforge_api_key`)
/// — sem ela, devolve um erro claro em vez de deixar a requisição
/// estourar um 403 confuso lá na frente.
#[tauri::command]
pub async fn search_curseforge(
    app: AppHandle,
    query: String,
    project_type: ProjectType,
    offset: u32,
    limit: u32,
) -> AppResult<CurseForgeSearchResponse> {
    let settings = SettingsStore::new(&app)?.read()?;
    let api_key = settings
        .curseforge_api_key
        .ok_or_else(|| AppError::InvalidInput("chave da API do CurseForge não configurada".to_string()))?;
    Ok(curseforge::search(&api_key, &query, project_type, offset, limit).await?)
}
