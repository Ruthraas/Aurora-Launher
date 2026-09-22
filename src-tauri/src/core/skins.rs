use base64::Engine;
use serde::Deserialize;

use crate::core::download::http::{client, with_retry};
use crate::error::AppError;

const MOJANG_API_BASE: &str = "https://api.mojang.com";
const SESSION_SERVER_BASE: &str = "https://sessionserver.mojang.com";

#[derive(Debug, Deserialize)]
struct ProfileLookup {
    id: String,
}

#[derive(Debug, Deserialize)]
struct SessionProfile {
    properties: Vec<SessionProperty>,
}

#[derive(Debug, Deserialize)]
struct SessionProperty {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct TexturesPayload {
    textures: Textures,
}

#[derive(Debug, Default, Deserialize)]
struct Textures {
    #[serde(rename = "SKIN")]
    skin: Option<Texture>,
    #[serde(rename = "CAPE")]
    cape: Option<Texture>,
}

#[derive(Debug, Deserialize)]
struct Texture {
    url: String,
}

pub struct FetchedTextures {
    pub skin_png: Vec<u8>,
    pub cape_png: Option<Vec<u8>>,
}

/// Busca a skin (e capa, se tiver) publicada por um jogador de
/// verdade, pelo nome — mesma técnica que o SKlauncher e outros
/// launchers de terceiros usam pra "pegar emprestada" uma skin pública
/// sem precisar logar com aquela conta: `api.mojang.com` resolve
/// nome→UUID, `sessionserver` devolve o perfil com uma propriedade
/// `textures` em base64 contendo a URL real da textura (confirmado
/// contra a API de verdade, perfil do "Notch").
pub async fn fetch_by_username(username: &str) -> Result<FetchedTextures, AppError> {
    let lookup_url = format!("{MOJANG_API_BASE}/users/profiles/minecraft/{username}");
    let lookup: ProfileLookup = with_retry(|| async {
        let response = client().get(&lookup_url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(format!("jogador \"{username}\"")));
        }
        Ok(response.error_for_status()?.json().await?)
    })
    .await?;

    let profile_url = format!("{SESSION_SERVER_BASE}/session/minecraft/profile/{}", lookup.id);
    let profile: SessionProfile = with_retry(|| async {
        let response = client().get(&profile_url).send().await?.error_for_status()?;
        Ok(response.json().await?)
    })
    .await?;

    let textures_property = profile
        .properties
        .iter()
        .find(|p| p.name == "textures")
        .ok_or_else(|| AppError::NotFound(format!("\"{username}\" não tem textures publicadas")))?;

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&textures_property.value)
        .map_err(|_| AppError::InvalidInput("propriedade textures não é base64 válido".to_string()))?;
    let payload: TexturesPayload = serde_json::from_slice(&decoded)?;

    let skin_url =
        payload.textures.skin.map(|t| t.url).ok_or_else(|| AppError::NotFound(format!("\"{username}\" não tem skin")))?;
    let skin_png = download_png(&skin_url).await?;
    let cape_png = match payload.textures.cape {
        Some(cape) => Some(download_png(&cape.url).await?),
        None => None,
    };

    Ok(FetchedTextures { skin_png, cape_png })
}

async fn download_png(url: &str) -> Result<Vec<u8>, AppError> {
    with_retry(|| async {
        let response = client().get(url).send().await?.error_for_status()?;
        Ok(response.bytes().await?.to_vec())
    })
    .await
}
