use tauri::{AppHandle, Manager};

use crate::core::java;
use crate::error::{AppError, AppResult};

/// Garante que o runtime Java pedido (ex.: `"java-runtime-delta"`,
/// vindo de `VersionDetails::required_java_component()`) está baixado
/// em `<app_data>/runtimes/<component>/`. Devolve o caminho do `java`
/// pronto pra uso. Baixa de verdade — só chamar quando o usuário
/// realmente for lançar uma instância que precise dele (Fase 4).
#[tauri::command]
pub async fn ensure_java_runtime(app: AppHandle, component: String) -> AppResult<String> {
    let runtimes_dir = app.path().app_data_dir().map_err(AppError::from)?.join("runtimes");
    let java_path = java::ensure_runtime(&runtimes_dir, &component, |_, _| {}).await?;
    Ok(java_path.to_string_lossy().into_owned())
}
