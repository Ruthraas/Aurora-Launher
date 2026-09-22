use std::path::PathBuf;
use std::sync::Arc;

use futures::stream::{FuturesUnordered, StreamExt};
use tokio::sync::Semaphore;

use super::file::download_file;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    /// Algumas bibliotecas (ex.: o próprio jar do Fabric Loader) não
    /// publicam hash — nesse caso a verificação é pulada.
    pub sha1: Option<String>,
}

/// Baixa todas as tasks em paralelo, respeitando `concurrency` downloads
/// simultâneos, chamando `on_progress(concluídos, total)` a cada uma
/// que termina — é o que alimenta a barra de progresso na UI.
///
/// Para na primeira falha e devolve o erro — como `download_file`
/// escreve atomicamente, o que já tinha concluído continua no disco,
/// então repetir a chamada não rebaixa esses.
pub async fn download_all<F>(tasks: Vec<DownloadTask>, concurrency: usize, mut on_progress: F) -> Result<(), AppError>
where
    F: FnMut(usize, usize),
{
    let total = tasks.len();
    if total == 0 {
        return Ok(());
    }

    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut in_flight = FuturesUnordered::new();

    for task in tasks {
        let semaphore = Arc::clone(&semaphore);
        in_flight.push(async move {
            let _permit = semaphore.acquire_owned().await.expect("semáforo nunca é fechado");
            download_file(&task.url, &task.dest, task.sha1.as_deref()).await
        });
    }

    let mut completed = 0;
    while let Some(result) = in_flight.next().await {
        result?;
        completed += 1;
        on_progress(completed, total);
    }

    Ok(())
}
