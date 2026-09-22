pub mod offline;
mod store;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use store::AccountStore;

/// Uma conta pronta pra jogar. Aurora Launcher é offline-only por
/// decisão do produto — sem login Microsoft, sem tentativa de contornar
/// a autenticação oficial da Mojang.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Account {
    Offline {
        id: Uuid,
        uuid: Uuid,
        username: String,
        added_at: DateTime<Utc>,
        /// Nome do arquivo em `<app_data>/skins/<id>/` (não o caminho
        /// inteiro — evita vazar o layout de pastas do usuário pro
        /// front). `None` = skin padrão (Steve/Alex).
        #[serde(default)]
        skin_file: Option<String>,
        #[serde(default)]
        cape_file: Option<String>,
    },
}

impl Account {
    pub fn id(&self) -> Uuid {
        match self {
            Account::Offline { id, .. } => *id,
        }
    }
}
