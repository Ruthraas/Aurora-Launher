use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use super::Account;
use crate::error::AppError;

const ACCOUNTS_FILE: &str = "accounts.json";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountsFile {
    accounts: Vec<Account>,
    active_account_id: Option<Uuid>,
}

/// Lê e escreve `accounts.json` na pasta de dados do app. Cada chamada
/// relê o arquivo do disco — o volume de contas é pequeno o bastante
/// pra isso não ser um problema, e evita ter que sincronizar um cache
/// em memória entre múltiplos comandos.
pub struct AccountStore {
    file_path: PathBuf,
}

impl AccountStore {
    pub fn new(app: &AppHandle) -> Result<Self, AppError> {
        let dir = app.path().app_data_dir()?;
        fs::create_dir_all(&dir)?;
        Ok(Self { file_path: dir.join(ACCOUNTS_FILE) })
    }

    fn read(&self) -> Result<AccountsFile, AppError> {
        if !self.file_path.exists() {
            return Ok(AccountsFile::default());
        }
        let raw = fs::read_to_string(&self.file_path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn write(&self, data: &AccountsFile) -> Result<(), AppError> {
        let raw = serde_json::to_string_pretty(data)?;
        fs::write(&self.file_path, raw)?;
        Ok(())
    }

    pub fn list(&self) -> Result<(Vec<Account>, Option<Uuid>), AppError> {
        let data = self.read()?;
        Ok((data.accounts, data.active_account_id))
    }

    pub fn add(&self, account: Account) -> Result<(), AppError> {
        let mut data = self.read()?;
        if data.active_account_id.is_none() {
            data.active_account_id = Some(account.id());
        }
        data.accounts.push(account);
        self.write(&data)
    }

    pub fn remove(&self, id: Uuid) -> Result<(), AppError> {
        let mut data = self.read()?;
        let existed = data.accounts.iter().any(|a| a.id() == id);
        if !existed {
            return Err(AppError::NotFound(format!("conta {id}")));
        }
        data.accounts.retain(|a| a.id() != id);
        if data.active_account_id == Some(id) {
            data.active_account_id = data.accounts.first().map(Account::id);
        }
        self.write(&data)
    }

    /// Lê, aplica `f` e regrava — usado pra mudanças pontuais (ex.:
    /// setar a skin) sem duplicar a lógica de leitura/escrita de
    /// `accounts.json`.
    pub fn update(&self, id: Uuid, f: impl FnOnce(&mut Account)) -> Result<Account, AppError> {
        let mut data = self.read()?;
        let account = data.accounts.iter_mut().find(|a| a.id() == id).ok_or_else(|| AppError::NotFound(format!("conta {id}")))?;
        f(account);
        let updated = account.clone();
        self.write(&data)?;
        Ok(updated)
    }

    pub fn set_active(&self, id: Uuid) -> Result<(), AppError> {
        let mut data = self.read()?;
        if !data.accounts.iter().any(|a| a.id() == id) {
            return Err(AppError::NotFound(format!("conta {id}")));
        }
        data.active_account_id = Some(id);
        self.write(&data)
    }

    /// Sai da conta ativa (sem remover nenhuma conta) — o launcher volta
    /// pra tela de login, as contas continuam salvas pra escolher de
    /// novo depois.
    pub fn clear_active(&self) -> Result<(), AppError> {
        let mut data = self.read()?;
        data.active_account_id = None;
        self.write(&data)
    }
}
