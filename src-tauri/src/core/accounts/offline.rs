use chrono::Utc;
use uuid::Uuid;

use super::Account;
use crate::error::AppError;

const MAX_NICKNAME_LEN: usize = 16;

/// UUID "offline" do Minecraft: MD5 direto de `"OfflinePlayer:<nick>"`
/// com os bits de versão/variante ajustados — é literalmente o que
/// `UUID.nameUUIDFromBytes` faz em Java, SEM prefixo de namespace (por
/// isso não dá pra usar `Uuid::new_v3` de uma lib genérica, que
/// concatenaria um namespace antes do hash e daria um UUID diferente).
pub fn offline_uuid(nickname: &str) -> Uuid {
    let name = format!("OfflinePlayer:{nickname}");
    let digest = md5::compute(name.as_bytes());
    let mut bytes = *digest;
    bytes[6] = (bytes[6] & 0x0f) | 0x30; // versão 3
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variante RFC4122
    Uuid::from_bytes(bytes)
}

pub fn validate_nickname(nickname: &str) -> Result<(), AppError> {
    let trimmed = nickname.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("nome de usuário não pode ser vazio".into()));
    }
    if trimmed.chars().count() > MAX_NICKNAME_LEN {
        return Err(AppError::InvalidInput(format!(
            "nome de usuário deve ter no máximo {MAX_NICKNAME_LEN} caracteres"
        )));
    }
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::InvalidInput(
            "nome de usuário só pode ter letras, números e _".into(),
        ));
    }
    Ok(())
}

pub fn create_offline_account(nickname: &str) -> Result<Account, AppError> {
    validate_nickname(nickname)?;
    let trimmed = nickname.trim();
    Ok(Account::Offline {
        id: Uuid::new_v4(),
        uuid: offline_uuid(trimmed),
        username: trimmed.to_string(),
        added_at: Utc::now(),
        skin_file: None,
        cape_file: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_matches_known_vector() {
        // vetor documentado publicamente para o algoritmo do Minecraft
        // (idêntico ao UUID.nameUUIDFromBytes do Java)
        assert_eq!(offline_uuid("Notch").to_string(), "b50ad385-829d-3141-a216-7e7d7539ba7f");
    }

    #[test]
    fn offline_uuid_is_case_sensitive() {
        assert_ne!(offline_uuid("Steve"), offline_uuid("steve"));
    }

    #[test]
    fn offline_uuid_is_deterministic() {
        assert_eq!(offline_uuid("BDarkBr"), offline_uuid("BDarkBr"));
    }

    #[test]
    fn validate_nickname_rejects_empty_or_blank() {
        assert!(validate_nickname("").is_err());
        assert!(validate_nickname("   ").is_err());
    }

    #[test]
    fn validate_nickname_rejects_invalid_chars() {
        assert!(validate_nickname("bad name!").is_err());
    }

    #[test]
    fn validate_nickname_rejects_too_long() {
        assert!(validate_nickname("a_name_way_too_long_for_minecraft").is_err());
    }

    #[test]
    fn validate_nickname_accepts_valid() {
        assert!(validate_nickname("BDarkBr_123").is_ok());
    }

    #[test]
    fn create_offline_account_trims_nickname() {
        let account = create_offline_account("  BDarkBr  ").unwrap();
        let Account::Offline { username, .. } = account else {
            panic!("esperava conta offline");
        };
        assert_eq!(username, "BDarkBr");
    }
}
