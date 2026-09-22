use std::future::Future;
use std::sync::OnceLock;
use std::time::Duration;

use crate::error::AppError;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Cliente HTTP compartilhado, com timeout — sem isso, uma conexão que
/// trava (em vez de falhar rápido) deixaria a instalação parada pra
/// sempre sem nenhum erro pra mostrar.
pub fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("configuração do cliente HTTP é estática e válida")
    })
}

/// Tenta a operação até 3 vezes, com espera crescente entre elas
/// (300ms, depois 900ms) — cobre uma falha de rede pontual sem exigir
/// que o usuário reinicie a instalação inteira à mão.
pub async fn with_retry<T, F, Fut>(mut attempt: F) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    const MAX_ATTEMPTS: u32 = 3;
    let mut last_error = None;

    for try_number in 0..MAX_ATTEMPTS {
        match attempt().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                last_error = Some(error);
                if try_number + 1 < MAX_ATTEMPTS {
                    let backoff = Duration::from_millis(300 * 3u64.pow(try_number));
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    Err(last_error.expect("o loop roda ao menos uma vez, então sempre há um erro aqui"))
}
