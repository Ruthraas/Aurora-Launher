use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::core::accounts::{offline, Account, AccountStore};
use crate::core::skins::{self, TextureKind};
use crate::error::{AppError, AppResult};

fn skins_dir(app: &AppHandle, account_id: Uuid) -> Result<std::path::PathBuf, AppError> {
    let dir = app.path().app_data_dir()?.join("skins").join(account_id.to_string());
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountsResponse {
    pub accounts: Vec<Account>,
    pub active_account_id: Option<Uuid>,
}

#[tauri::command]
pub fn list_accounts(app: AppHandle) -> AppResult<AccountsResponse> {
    let store = AccountStore::new(&app)?;
    let (accounts, active_account_id) = store.list()?;
    Ok(AccountsResponse { accounts, active_account_id })
}

#[tauri::command]
pub fn add_offline_account(app: AppHandle, nickname: String) -> AppResult<Account> {
    let account = offline::create_offline_account(&nickname)?;
    let store = AccountStore::new(&app)?;
    store.add(account.clone())?;
    Ok(account)
}

#[tauri::command]
pub fn remove_account(app: AppHandle, id: Uuid) -> AppResult<()> {
    let store = AccountStore::new(&app)?;
    store.remove(id)?;
    Ok(())
}

#[tauri::command]
pub fn set_active_account(app: AppHandle, id: Uuid) -> AppResult<()> {
    let store = AccountStore::new(&app)?;
    store.set_active(id)?;
    Ok(())
}

#[tauri::command]
pub fn logout(app: AppHandle) -> AppResult<()> {
    let store = AccountStore::new(&app)?;
    store.clear_active()?;
    Ok(())
}

/// Busca a skin (e capa, se tiver) publicada por um jogador de
/// verdade, pelo nome, e aplica na conta offline — pura conveniência
/// visual local, não autentica como aquela conta nem manda nada pra
/// Mojang além da própria consulta pública de perfil.
#[tauri::command]
pub async fn import_account_skin_by_username(app: AppHandle, id: Uuid, username: String) -> AppResult<Account> {
    let textures = skins::fetch_by_username(&username).await?;
    let dir = skins_dir(&app, id)?;

    std::fs::write(dir.join("skin.png"), &textures.skin_png).map_err(AppError::from)?;
    if let Some(cape) = &textures.cape_png {
        std::fs::write(dir.join("cape.png"), cape).map_err(AppError::from)?;
    }

    let store = AccountStore::new(&app)?;
    Ok(store.update(id, |account| {
        let Account::Offline { skin_file, cape_file, .. } = account;
        *skin_file = Some("skin.png".to_string());
        *cape_file = textures.cape_png.is_some().then(|| "cape.png".to_string());
    })?)
}

#[tauri::command]
pub fn upload_account_skin(app: AppHandle, id: Uuid, png_bytes: Vec<u8>) -> AppResult<Account> {
    skins::validate_texture_upload(&png_bytes, TextureKind::Skin)?;
    let dir = skins_dir(&app, id)?;
    std::fs::write(dir.join("skin.png"), &png_bytes).map_err(AppError::from)?;

    let store = AccountStore::new(&app)?;
    Ok(store.update(id, |account| {
        let Account::Offline { skin_file, .. } = account;
        *skin_file = Some("skin.png".to_string());
    })?)
}

#[tauri::command]
pub fn upload_account_cape(app: AppHandle, id: Uuid, png_bytes: Vec<u8>) -> AppResult<Account> {
    skins::validate_texture_upload(&png_bytes, TextureKind::Cape)?;
    let dir = skins_dir(&app, id)?;
    std::fs::write(dir.join("cape.png"), &png_bytes).map_err(AppError::from)?;

    let store = AccountStore::new(&app)?;
    Ok(store.update(id, |account| {
        let Account::Offline { cape_file, .. } = account;
        *cape_file = Some("cape.png".to_string());
    })?)
}

#[tauri::command]
pub fn clear_account_skin(app: AppHandle, id: Uuid) -> AppResult<Account> {
    let store = AccountStore::new(&app)?;
    Ok(store.update(id, |account| {
        let Account::Offline { skin_file, .. } = account;
        *skin_file = None;
    })?)
}

#[tauri::command]
pub fn clear_account_cape(app: AppHandle, id: Uuid) -> AppResult<Account> {
    let store = AccountStore::new(&app)?;
    Ok(store.update(id, |account| {
        let Account::Offline { cape_file, .. } = account;
        *cape_file = None;
    })?)
}

/// Devolve a textura como PNG em base64, pro front renderizar (cara
/// de bloco / preview) sem precisar configurar o protocolo de assets
/// do Tauri pra pasta de dados do app.
#[tauri::command]
pub fn get_account_texture(app: AppHandle, id: Uuid, texture: String) -> AppResult<Option<String>> {
    let file_name = match texture.as_str() {
        "skin" => "skin.png",
        "cape" => "cape.png",
        other => return Err(AppError::InvalidInput(format!("textura desconhecida: {other}")).into()),
    };
    let path = skins_dir(&app, id)?.join(file_name);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(AppError::from)?;
    Ok(Some(base64::engine::general_purpose::STANDARD.encode(bytes)))
}
