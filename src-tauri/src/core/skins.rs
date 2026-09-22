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

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const MAX_TEXTURE_BYTES: usize = 2 * 1024 * 1024;

/// Dimensões válidas de skin do Minecraft: 64x64 (formato atual) ou
/// 64x32 (formato legado, ainda aceito). Capa é sempre 64x32.
/// Confirmado contra o próprio uploader oficial do Mojang (minecraft.net),
/// que rejeita qualquer outra dimensão.
fn is_valid_skin_size(width: u32, height: u32) -> bool {
    (width, height) == (64, 64) || (width, height) == (64, 32)
}

fn is_valid_cape_size(width: u32, height: u32) -> bool {
    (width, height) == (64, 32)
}

/// Lê só a assinatura + o chunk `IHDR` (largura/altura) de um PNG —
/// não precisa de uma crate de imagem inteira pra validar upload, o
/// formato do cabeçalho é fixo e documentado (assinatura de 8 bytes +
/// chunk IHDR de 13 bytes logo em seguida: 4 bytes de tamanho, "IHDR",
/// 4 bytes de largura, 4 bytes de altura, big-endian).
fn read_png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 8 + 8 + 8 || bytes[0..8] != PNG_SIGNATURE {
        return None;
    }
    if &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

/// Valida um upload de textura antes de gravar em disco — sem isso, um
/// arquivo que não é PNG de verdade (ou tem a dimensão errada) só
/// falhava depois, de forma confusa, quando o jogo tentava carregar a
/// textura. `kind` é só pro texto do erro.
pub fn validate_texture_upload(bytes: &[u8], kind: TextureKind) -> Result<(), AppError> {
    if bytes.len() > MAX_TEXTURE_BYTES {
        return Err(AppError::InvalidInput(format!("arquivo maior que {}MB", MAX_TEXTURE_BYTES / 1024 / 1024)));
    }
    let (width, height) = read_png_dimensions(bytes).ok_or_else(|| AppError::InvalidInput("arquivo não é um PNG válido".to_string()))?;
    let valid = match kind {
        TextureKind::Skin => is_valid_skin_size(width, height),
        TextureKind::Cape => is_valid_cape_size(width, height),
    };
    if !valid {
        let expected = match kind {
            TextureKind::Skin => "64x64 ou 64x32",
            TextureKind::Cape => "64x32",
        };
        return Err(AppError::InvalidInput(format!(
            "dimensão inválida ({width}x{height}) — {} do Minecraft precisa ser {expected}",
            kind.label()
        )));
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub enum TextureKind {
    Skin,
    Cape,
}

impl TextureKind {
    fn label(self) -> &'static str {
        match self {
            TextureKind::Skin => "skin",
            TextureKind::Cape => "capa",
        }
    }
}

#[cfg(test)]
mod texture_validation_tests {
    use super::*;

    fn png_with_dimensions(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[0u8; 5]);
        bytes
    }

    #[test]
    fn rejects_non_png_bytes() {
        assert!(validate_texture_upload(b"not a png", TextureKind::Skin).is_err());
    }

    #[test]
    fn accepts_64x64_skin() {
        assert!(validate_texture_upload(&png_with_dimensions(64, 64), TextureKind::Skin).is_ok());
    }

    #[test]
    fn accepts_legacy_64x32_skin() {
        assert!(validate_texture_upload(&png_with_dimensions(64, 32), TextureKind::Skin).is_ok());
    }

    #[test]
    fn rejects_wrong_skin_dimensions() {
        assert!(validate_texture_upload(&png_with_dimensions(100, 100), TextureKind::Skin).is_err());
    }

    #[test]
    fn accepts_64x32_cape() {
        assert!(validate_texture_upload(&png_with_dimensions(64, 32), TextureKind::Cape).is_ok());
    }

    #[test]
    fn rejects_64x64_cape() {
        assert!(validate_texture_upload(&png_with_dimensions(64, 64), TextureKind::Cape).is_err());
    }

    #[test]
    fn rejects_oversized_file() {
        let mut big = png_with_dimensions(64, 64);
        big.resize(MAX_TEXTURE_BYTES + 1, 0);
        assert!(validate_texture_upload(&big, TextureKind::Skin).is_err());
    }
}
