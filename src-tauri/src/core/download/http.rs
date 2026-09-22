use std::future::Future;
use std::sync::OnceLock;
use std::time::Duration;

use crate::error::AppError;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Timeout usado por download de arquivo (client.jar, bibliotecas,
/// pacotes de modpack inteiros) — bem mais folgado que o default do
/// cliente porque é timeout de requisição TOTAL (não "parado por N
/// segundos"): um arquivo grande numa conexão lenta pode legitimamente
/// levar minutos ainda transferindo normalmente.
pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// Cliente HTTP compartilhado. O timeout default (30s) é pra chamada
/// de API pequena (busca, metadados, versões) — sem isso, uma conexão
/// que trava deixaria a instalação parada pra sempre sem nenhum erro
/// pra mostrar. Download de arquivo usa `DOWNLOAD_TIMEOUT` por cima
/// deste via `.timeout()` no request (reqwest deixa sobrescrever por
/// chamada) — ver `download::file::download_file`.
pub fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
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
