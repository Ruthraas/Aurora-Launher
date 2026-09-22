use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::AppError;

const SETTINGS_FILE: &str = "settings.json";

/// Configurações do app que não são conta nem instância — hoje só a
/// chave da API do CurseForge, guardada em `settings.json` na pasta de
/// dados do app (NUNCA no código-fonte nem em variável de ambiente do
/// repo). A chave é pessoal do usuário, obtida em console.curseforge.com.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub curseforge_api_key: Option<String>,
}

pub struct SettingsStore {
    file_path: PathBuf,
}

impl SettingsStore {
    pub fn new(app: &AppHandle) -> Result<Self, AppError> {
        let dir = app.path().app_data_dir()?;
        fs::create_dir_all(&dir)?;
        Ok(Self { file_path: dir.join(SETTINGS_FILE) })
    }

    pub fn read(&self) -> Result<Settings, AppError> {
        if !self.file_path.exists() {
            return Ok(Settings::default());
        }
        let raw = fs::read_to_string(&self.file_path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn write(&self, settings: &Settings) -> Result<(), AppError> {
        let raw = serde_json::to_string_pretty(settings)?;
        fs::write(&self.file_path, raw)?;
        Ok(())
    }

    pub fn set_curseforge_api_key(&self, api_key: Option<String>) -> Result<(), AppError> {
        let mut settings = self.read()?;
        settings.curseforge_api_key = api_key.filter(|key| !key.trim().is_empty());
        self.write(&settings)
    }
}
