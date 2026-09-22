use std::path::{Path, PathBuf};

use super::runtime_manifest::{fetch_runtime_entry, fetch_runtime_files_manifest, RuntimeFileEntry};
use crate::core::download::{download_all, DownloadTask};
use crate::core::minecraft::rules::TargetOs;
use crate::error::AppError;

/// Garante que o runtime Java pedido está presente em
/// `runtimes_dir/<component>/`, baixando o que faltar. Idempotente:
/// como `download_file` escreve atomicamente e pula o que já bate com
/// o hash esperado, chamar de novo só completa o que faltou.
///
/// Devolve o caminho do executável `java`/`javaw` pronto pra uso.
pub async fn ensure_runtime<F>(
    runtimes_dir: &Path,
    component: &str,
    on_progress: F,
) -> Result<PathBuf, AppError>
where
    F: FnMut(usize, usize),
{
    let os = TargetOs::current();
    let entry = fetch_runtime_entry(os, component)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("runtime Java \"{component}\" para este sistema")))?;

    let files_manifest = fetch_runtime_files_manifest(&entry.manifest.url).await?;
    let runtime_dir = runtimes_dir.join(component);

    let mut tasks = Vec::new();
    for (relative_path, file_entry) in &files_manifest.files {
        let dest = runtime_dir.join(relative_path);
        match file_entry {
            RuntimeFileEntry::Directory => {
                tokio::fs::create_dir_all(&dest).await?;
            }
            RuntimeFileEntry::File { downloads, .. } => {
                tasks.push(DownloadTask {
                    url: downloads.raw.url.clone(),
                    dest,
                    sha1: Some(downloads.raw.sha1.clone()),
                });
            }
            RuntimeFileEntry::Link { .. } => {
                // symlinks só existem em builds Unix — pendente pra
                // quando o suporte a Linux/macOS for priorizado.
            }
        }
    }

    download_all(tasks, 8, on_progress).await?;

    Ok(java_executable_path(&runtime_dir))
}

fn java_executable_path(runtime_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        runtime_dir.join("bin").join("javaw.exe")
    } else {
        runtime_dir.join("bin").join("java")
    }
}
