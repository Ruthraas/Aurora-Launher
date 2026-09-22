pub mod install;
pub mod launch;
pub mod mods;
pub mod options_txt;
pub mod transfer;

use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
pub use mods::ModpackOrigin;

const INSTANCE_FILE: &str = "instance.json";
const SCHEMA_VERSION: u32 = 1;
const MAX_INSTANCE_NAME_LEN: usize = 60;

/// Diferente do nickname de conta offline (`accounts::offline::validate_nickname`,
/// restrito a `[a-zA-Z0-9_]` porque vira o nome de usuário real do
/// Minecraft), o nome de instância é só um rótulo de exibição — a
/// pasta em disco usa o UUID (`InstanceStore::instance_dir`), não o
/// nome. Então aqui só barra vazio/espaço-só e tamanho absurdo, sem
/// restringir caracteres.
fn validate_instance_name(name: &str) -> Result<(), AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("nome da instância não pode ser vazio".to_string()));
    }
    if trimmed.chars().count() > MAX_INSTANCE_NAME_LEN {
        return Err(AppError::InvalidInput(format!("nome da instância deve ter no máximo {MAX_INSTANCE_NAME_LEN} caracteres")));
    }
    Ok(())
}

/// Forge clássico cobre MC 1.16-1.20.2 (ver `core::loaders::forge`) —
/// versões anteriores usam um instalador GUI que patcheava o jar
/// vanilla direto, formato completamente diferente, não suportado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LoaderKind {
    Vanilla,
    // `alias` aceita instâncias já persistidas em disco antes desta
    // struct ganhar `rename_all_fields` (o campo ia em snake_case).
    Fabric {
        #[serde(alias = "loader_version")]
        loader_version: String,
    },
    #[serde(rename = "neoforge")]
    NeoForge {
        #[serde(alias = "neoforge_version")]
        neoforge_version: String,
    },
    Forge {
        #[serde(alias = "forge_version")]
        forge_version: String,
    },
    Quilt {
        #[serde(alias = "loader_version")]
        loader_version: String,
    },
}

impl LoaderKind {
    pub fn label(&self) -> &'static str {
        match self {
            LoaderKind::Vanilla => "Vanilla",
            LoaderKind::Fabric { .. } => "Fabric",
            LoaderKind::NeoForge { .. } => "NeoForge",
            LoaderKind::Forge { .. } => "Forge",
            LoaderKind::Quilt { .. } => "Quilt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstanceStatus {
    NotInstalled,
    Installing,
    Ready,
    /// Só acontece com modpacks do CurseForge: alguns mods têm
    /// `downloadUrl: null` (autor bloqueou download automático) — o
    /// resto do pack instala normal, esses ficam faltando e o usuário
    /// precisa baixar na mão (ver `Instance::missing_manual_downloads`).
    Incomplete,
    Error,
}

/// Conjunto de flags de JVM aplicado no lançamento, além de
/// `-Xms`/`-Xmx` (que já vêm de `ram_min_mb`/`ram_max_mb`). Ver
/// `core::jvm_flags` pros valores reais de cada preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JvmFlagPreset {
    None,
    G1gcOptimized,
}

impl Default for JvmFlagPreset {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub schema_version: u32,
    pub id: Uuid,
    pub name: String,
    pub mc_version: String,
    pub loader: LoaderKind,
    pub ram_min_mb: u32,
    pub ram_max_mb: u32,
    #[serde(default)]
    pub jvm_flag_preset: JvmFlagPreset,
    /// Flags extras que o usuário digitou na mão (separadas por
    /// espaço), acrescentadas DEPOIS das do preset — pra quem sabe o
    /// que está fazendo e quer algo que nenhum preset cobre. Recurso
    /// padrão em qualquer launcher concorrente (MultiMC, Prism, app do
    /// Modrinth).
    #[serde(default)]
    pub custom_jvm_args: String,
    pub status: InstanceStatus,
    /// Preenchido quando `status == Error`, pra mostrar na UI o motivo
    /// real em vez de um "falhou" genérico.
    #[serde(default)]
    pub error_message: Option<String>,
    /// Preenchido só quando a instância veio de "Baixar modpack".
    #[serde(default)]
    pub modpack_origin: Option<ModpackOrigin>,
    /// Nome dos arquivos que o CurseForge bloqueou pra download
    /// automático (`status == Incomplete`) — cada um com o link da
    /// página do mod pro usuário baixar na mão.
    #[serde(default)]
    pub missing_manual_downloads: Vec<ManualDownload>,
    pub created_at: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualDownload {
    pub file_name: String,
    pub project_url: String,
}

/// Cada instância mora em `instances_dir/<id>/instance.json`. Sem cache
/// em memória — o volume de instâncias é pequeno o bastante pra reler
/// do disco a cada chamada, evitando ter que sincronizar estado.
pub struct InstanceStore {
    instances_dir: PathBuf,
}

impl InstanceStore {
    pub fn new(instances_dir: PathBuf) -> Self {
        Self { instances_dir }
    }

    pub fn instance_dir(&self, id: Uuid) -> PathBuf {
        self.instances_dir.join(id.to_string())
    }

    pub fn list(&self) -> Result<Vec<Instance>, AppError> {
        if !self.instances_dir.exists() {
            return Ok(Vec::new());
        }
        let mut instances = Vec::new();
        for entry in fs::read_dir(&self.instances_dir)? {
            let path = entry?.path().join(INSTANCE_FILE);
            if !path.exists() {
                continue;
            }
            let raw = fs::read_to_string(&path)?;
            instances.push(serde_json::from_str(&raw)?);
        }
        instances.sort_by(|a: &Instance, b: &Instance| b.created_at.cmp(&a.created_at));
        Ok(instances)
    }

    pub fn get(&self, id: Uuid) -> Result<Instance, AppError> {
        let path = self.instance_dir(id).join(INSTANCE_FILE);
        let raw = fs::read_to_string(&path).map_err(|_| AppError::NotFound(format!("instância {id}")))?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn create(
        &self,
        name: String,
        mc_version: String,
        loader: LoaderKind,
        ram_min_mb: u32,
        ram_max_mb: u32,
    ) -> Result<Instance, AppError> {
        validate_instance_name(&name)?;
        let name = name.trim().to_string();
        let instance = Instance {
            schema_version: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            name,
            mc_version,
            loader,
            ram_min_mb,
            ram_max_mb,
            jvm_flag_preset: JvmFlagPreset::default(),
            custom_jvm_args: String::new(),
            status: InstanceStatus::NotInstalled,
            error_message: None,
            modpack_origin: None,
            missing_manual_downloads: Vec::new(),
            created_at: Utc::now(),
            last_played: None,
        };
        self.save(&instance)?;
        Ok(instance)
    }

    /// Gera um nome de instância que não colide com nenhuma já
    /// existente — "Nome", depois "Nome (2)", "Nome (3)"... (usado pra
    /// criar instância a partir de um modpack, quando o nome padrão já
    /// existe).
    pub fn unique_name(&self, base: &str) -> Result<String, AppError> {
        let existing: std::collections::HashSet<String> =
            self.list()?.into_iter().map(|i| i.name).collect();
        if !existing.contains(base) {
            return Ok(base.to_string());
        }
        let mut n = 2;
        loop {
            let candidate = format!("{base} ({n})");
            if !existing.contains(&candidate) {
                return Ok(candidate);
            }
            n += 1;
        }
    }

    pub fn save(&self, instance: &Instance) -> Result<(), AppError> {
        let dir = self.instance_dir(instance.id);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join(INSTANCE_FILE), serde_json::to_string_pretty(instance)?)?;
        Ok(())
    }

    pub fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let dir = self.instance_dir(id);
        if !dir.exists() {
            return Err(AppError::NotFound(format!("instância {id}")));
        }
        fs::remove_dir_all(&dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> InstanceStore {
        InstanceStore::new(std::env::temp_dir().join(format!("aurora-instances-{}", Uuid::new_v4())))
    }

    #[test]
    fn create_rejects_empty_or_blank_name() {
        let store = temp_store();
        assert!(store.create("   ".to_string(), "1.21".to_string(), LoaderKind::Vanilla, 1024, 2048).is_err());
    }

    #[test]
    fn create_rejects_name_too_long() {
        let store = temp_store();
        let long_name = "a".repeat(MAX_INSTANCE_NAME_LEN + 1);
        assert!(store.create(long_name, "1.21".to_string(), LoaderKind::Vanilla, 1024, 2048).is_err());
    }

    #[test]
    fn create_trims_name() {
        let store = temp_store();
        let instance = store.create("  Minha Instância  ".to_string(), "1.21".to_string(), LoaderKind::Vanilla, 1024, 2048).unwrap();
        assert_eq!(instance.name, "Minha Instância");
    }

    #[test]
    fn unique_name_keeps_base_when_free() {
        let store = temp_store();
        assert_eq!(store.unique_name("Aventura Clássica").unwrap(), "Aventura Clássica");
    }

    #[test]
    fn unique_name_appends_counter_on_collision() {
        let store = temp_store();
        store.create("Aventura Clássica".to_string(), "1.20.1".to_string(), LoaderKind::Vanilla, 1024, 2048).unwrap();
        assert_eq!(store.unique_name("Aventura Clássica").unwrap(), "Aventura Clássica (2)");

        store.create("Aventura Clássica (2)".to_string(), "1.20.1".to_string(), LoaderKind::Vanilla, 1024, 2048).unwrap();
        assert_eq!(store.unique_name("Aventura Clássica").unwrap(), "Aventura Clássica (3)");

        fs::remove_dir_all(&store.instances_dir).ok();
    }
}
