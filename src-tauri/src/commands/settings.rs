use tauri::AppHandle;

use crate::core::settings::{Settings, SettingsStore};
use crate::error::AppResult;

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppResult<Settings> {
    Ok(SettingsStore::new(&app)?.read()?)
}

/// Guarda a chave da API do CurseForge em `settings.json` na pasta de
/// dados do app — nunca no código-fonte. `api_key: null`/vazio limpa a
/// chave salva.
#[tauri::command]
pub fn set_curseforge_api_key(app: AppHandle, api_key: Option<String>) -> AppResult<()> {
    SettingsStore::new(&app)?.set_curseforge_api_key(api_key)?;
    Ok(())
}
