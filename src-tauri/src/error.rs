use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("entrada inválida: {0}")]
    InvalidInput(String),
    #[error("não encontrado: {0}")]
    NotFound(String),
    #[error("caminho de dados do app indisponível: {0}")]
    AppPath(#[from] tauri::Error),
    #[error("rede: {0}")]
    Network(#[from] reqwest::Error),
    #[error("hash divergente: esperado {expected}, obtido {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("zip inválido: {0}")]
    Zip(#[from] zip::result::ZipError),
}

/// `AppError` não implementa `Serialize` diretamente (erros de terceiros
/// como `io::Error` não implementam) — os comandos Tauri devolvem essa
/// forma serializável, com a mensagem já pronta para exibição na UI.
#[derive(Serialize)]
pub struct AppErrorPayload {
    pub message: String,
}

impl From<AppError> for AppErrorPayload {
    fn from(error: AppError) -> Self {
        Self { message: error.to_string() }
    }
}

pub type AppResult<T> = Result<T, AppErrorPayload>;
