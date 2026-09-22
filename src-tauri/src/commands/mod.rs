//! Camada fina exposta ao front via IPC. Cada submódulo agrupa os
//! comandos de uma área (app_info, accounts, instances, ...) e delega
//! a lógica pesada para `core::*` — nenhum comando deve conter regra
//! de negócio além de validar entrada e mapear o resultado.

pub mod accounts;
pub mod app_info;
pub mod curseforge;
pub mod instances;
pub mod java;
pub mod minecraft;
pub mod modpack;
pub mod modrinth;
pub mod mods;
pub mod settings;
