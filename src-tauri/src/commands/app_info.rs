use serde::Serialize;
use tauri::AppHandle;

/// Metadados estáticos do app, usados pela tela "Sobre" e para
/// validar de ponta a ponta o canal de IPC front↔Rust.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub identifier: String,
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppInfo {
    let config = app.config();
    AppInfo {
        name: config.product_name.clone().unwrap_or_default(),
        version: config.version.clone().unwrap_or_default(),
        identifier: config.identifier.clone(),
    }
}
