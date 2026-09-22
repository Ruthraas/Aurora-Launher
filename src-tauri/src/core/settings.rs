use std::fs;
use std::path::PathBuf;

use keyring::Entry;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::AppError;

const SETTINGS_FILE: &str = "settings.json";
const KEYRING_SERVICE: &str = "aurora-launcher";
const KEYRING_CURSEFORGE_KEY: &str = "curseforge_api_key";

/// Configurações do app que não são conta nem instância — hoje só a
/// chave da API do CurseForge. Guardada no cofre de credenciais do SO
/// (Windows Credential Manager, via a crate `keyring`) em vez de texto
/// puro em `settings.json` — a chave é pessoal do usuário (obtida em
/// console.curseforge.com) e um `settings.json` em texto plano fica
/// legível por qualquer processo/pessoa com acesso à pasta de dados do
/// app.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub curseforge_api_key: Option<String>,
}

/// Formato antigo de `settings.json` (antes da migração pro keyring) —
/// só usado pra ler uma instalação existente uma vez e apagar a chave
/// de lá.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacySettings {
    curseforge_api_key: Option<String>,
}

pub struct SettingsStore {
    file_path: PathBuf,
}

fn curseforge_entry() -> Result<Entry, AppError> {
    Entry::new(KEYRING_SERVICE, KEYRING_CURSEFORGE_KEY).map_err(|error| AppError::InvalidInput(error.to_string()))
}

impl SettingsStore {
    pub fn new(app: &AppHandle) -> Result<Self, AppError> {
        let dir = app.path().app_data_dir()?;
        fs::create_dir_all(&dir)?;
        Ok(Self { file_path: dir.join(SETTINGS_FILE) })
    }

    pub fn read(&self) -> Result<Settings, AppError> {
        self.migrate_legacy_file()?;

        let entry = curseforge_entry()?;
        let curseforge_api_key = match entry.get_password() {
            Ok(key) => Some(key),
            Err(keyring::Error::NoEntry) => None,
            Err(error) => return Err(AppError::InvalidInput(error.to_string())),
        };
        Ok(Settings { curseforge_api_key })
    }

    pub fn set_curseforge_api_key(&self, api_key: Option<String>) -> Result<(), AppError> {
        let entry = curseforge_entry()?;
        match api_key.filter(|key| !key.trim().is_empty()) {
            Some(key) => entry.set_password(&key).map_err(|error| AppError::InvalidInput(error.to_string())),
            None => match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(error) => Err(AppError::InvalidInput(error.to_string())),
            },
        }
    }

    /// Instalação que já existia antes desta mudança tem a chave em
    /// `settings.json` puro — lê uma vez, move pro keyring, e apaga o
    /// arquivo (nada mais fica gravado nele; se um dia houver outra
    /// configuração não-sensível, volta a escrever, só sem a chave).
    fn migrate_legacy_file(&self) -> Result<(), AppError> {
        if !self.file_path.exists() {
            return Ok(());
        }
        let raw = fs::read_to_string(&self.file_path)?;
        let legacy: LegacySettings = serde_json::from_str(&raw).unwrap_or_default();
        if let Some(key) = legacy.curseforge_api_key.filter(|k| !k.trim().is_empty()) {
            let entry = curseforge_entry()?;
            entry.set_password(&key).map_err(|error| AppError::InvalidInput(error.to_string()))?;
        }
        fs::remove_file(&self.file_path)?;
        Ok(())
    }
}
