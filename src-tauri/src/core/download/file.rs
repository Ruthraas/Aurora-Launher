use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use super::http::{client, with_retry};
use crate::error::AppError;

/// Baixa `url` pra `dest`, verificando o SHA1 esperado quando ele
/// existe (algumas bibliotecas do Fabric não publicam hash — nesse
/// caso só confere se o arquivo já existe). Escreve num arquivo
/// temporário ao lado do destino (`<dest>.part`) e só troca pelo
/// definitivo (rename atômico) depois — nunca deixa um arquivo
/// corrompido/parcial com o nome final.
///
/// Se `dest` já existe e já bate com o hash esperado (ou já existe,
/// quando não há hash pra conferir), não baixa de novo — é o que
/// permite retomar depois de uma falha no meio de um lote grande sem
/// reiniciar do zero.
pub async fn download_file(url: &str, dest: &Path, expected_sha1: Option<&str>) -> Result<(), AppError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).await?;
    }

    let already_present = match expected_sha1 {
        Some(sha1) => file_matches_sha1(dest, sha1).await,
        None => tokio::fs::try_exists(dest).await.unwrap_or(false),
    };
    if already_present {
        return Ok(());
    }

    let bytes = with_retry(|| async {
        let response = client().get(url).send().await?.error_for_status()?;
        Ok(response.bytes().await?)
    })
    .await?;

    if let Some(expected) = expected_sha1 {
        let actual_sha1 = sha1_hex(&bytes);
        if !actual_sha1.eq_ignore_ascii_case(expected) {
            return Err(AppError::HashMismatch { expected: expected.to_string(), actual: actual_sha1 });
        }
    }

    let tmp_path = tmp_path_for(dest);
    {
        let mut file = fs::File::create(&tmp_path).await?;
        file.write_all(&bytes).await?;
        file.flush().await?;
    }
    fs::rename(&tmp_path, dest).await?;
    Ok(())
}

async fn file_matches_sha1(path: &Path, expected_sha1: &str) -> bool {
    let Ok(bytes) = fs::read(path).await else { return false };
    sha1_hex(&bytes).eq_ignore_ascii_case(expected_sha1)
}

pub fn sha1_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn tmp_path_for(dest: &Path) -> PathBuf {
    let mut tmp = dest.as_os_str().to_owned();
    tmp.push(".part");
    PathBuf::from(tmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_hex_matches_known_vector() {
        // sha1("") — vetor padrão, usado em toda implementação de referência.
        assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[tokio::test]
    async fn file_matches_sha1_true_for_correct_hash_false_otherwise() {
        let dir = std::env::temp_dir().join(format!("aurora-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("file.txt");
        tokio::fs::write(&path, b"abc").await.unwrap();

        assert!(file_matches_sha1(&path, "a9993e364706816aba3e25717850c26c9cd0d89d").await);
        assert!(!file_matches_sha1(&path, "0000000000000000000000000000000000000000").await);

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }
}
