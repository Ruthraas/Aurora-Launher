use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const MODS_LOCK_FILE: &str = "mods.lock.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModSource {
    Modrinth,
    Curseforge,
}

/// Tipo de conteúdo instalável numa instância — deliberadamente SEM
/// `Modpack` (baixar um modpack cria uma instância nova do zero, é um
/// fluxo completamente diferente, ver `core::modpack`). Cada tipo tem
/// sua própria pasta de destino.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentType {
    Mod,
    ResourcePack,
    Shader,
    Datapack,
}

impl Default for ContentType {
    fn default() -> Self {
        Self::Mod
    }
}

impl ContentType {
    pub fn install_dir(self) -> &'static str {
        match self {
            ContentType::Mod => "mods",
            ContentType::ResourcePack => "resourcepacks",
            ContentType::Shader => "shaderpacks",
            ContentType::Datapack => "datapacks",
        }
    }

    pub fn is_loader_locked(self) -> bool {
        matches!(self, ContentType::Mod)
    }
}

/// Um mod instalado numa instância. Guardado em `mods.lock.json`
/// dentro da pasta da instância — é o único jeito confiável de saber
/// "esse mod já tá aqui" sem ter que reabrir e inspecionar cada jar
/// toda vez (ver `mods_compat::detect_already_installed`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModLockEntry {
    pub source: ModSource,
    pub project_id: String,
    pub version_id: String,
    /// Só pra exibição amigável (ex.: "0.5.13") — a chave de
    /// deduplicação/comparação continua sendo `(source, project_id)`.
    pub version_number: String,
    pub file_name: String,
    pub sha1: String,
    pub installed_at: DateTime<Utc>,
    /// `true` quando o usuário escolheu instalar esse mod diretamente;
    /// `false` quando veio como dependência obrigatória de outro mod.
    pub explicit: bool,
    /// Título/ícone de verdade (ex.: "Sodium", não o nome do arquivo) —
    /// `None` em entradas persistidas antes desse campo existir, ou em
    /// dependências instaladas automaticamente (a gente não busca o
    /// ícone delas, só do que o usuário escolheu de propósito).
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    /// Determina em qual pasta o arquivo mora (`mods/`,
    /// `resourcepacks/`, ...) — sem isso, remover um resourcepack/
    /// shader/datapack só apagava a entrada do lock e deixava o
    /// arquivo de verdade órfão em disco pra sempre (bug real: todo
    /// mundo assumia `mods/` sem checar). Entradas persistidas antes
    /// desse campo existir assumem `Mod` (é o que praticamente tudo
    /// era até essa correção).
    #[serde(default)]
    pub content_type: ContentType,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ModsLockFile {
    mods: Vec<ModLockEntry>,
}

/// De qual modpack a instância veio (se veio de algum) — guardado pra
/// futuras atualizações de versão do pack (fora de escopo por agora,
/// só a referência já fica salva).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackOrigin {
    pub source: ModSource,
    pub project_id: String,
    pub version_id: String,
}

pub struct ModsLockStore {
    file_path: PathBuf,
}

impl ModsLockStore {
    pub fn new(instance_dir: &Path) -> Self {
        Self { file_path: instance_dir.join(MODS_LOCK_FILE) }
    }

    pub fn read(&self) -> Result<Vec<ModLockEntry>, AppError> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(&self.file_path)?;
        Ok(serde_json::from_str::<ModsLockFile>(&raw)?.mods)
    }

    fn write(&self, mods: &[ModLockEntry]) -> Result<(), AppError> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(&ModsLockFile { mods: mods.to_vec() })?;
        fs::write(&self.file_path, raw)?;
        Ok(())
    }

    /// Adiciona/substitui a entrada de `(source, project_id)` — nunca
    /// duplica um registro pro mesmo projeto na mesma instância.
    pub fn upsert(&self, entry: ModLockEntry) -> Result<(), AppError> {
        let mut mods = self.read()?;
        mods.retain(|m| !(m.source == entry.source && m.project_id == entry.project_id));
        mods.push(entry);
        self.write(&mods)
    }

    /// Remove a entrada de `(source, project_id)` do lock — NÃO apaga
    /// o arquivo em si (quem chama decide isso, já que sabe em qual
    /// pasta o arquivo mora: `mods/`, `resourcepacks/`, ...).
    pub fn remove(&self, source: ModSource, project_id: &str) -> Result<Option<ModLockEntry>, AppError> {
        let mut mods = self.read()?;
        let index = mods.iter().position(|m| m.source == source && m.project_id == project_id);
        let removed = index.map(|i| mods.remove(i));
        if removed.is_some() {
            self.write(&mods)?;
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(project_id: &str) -> ModLockEntry {
        ModLockEntry {
            source: ModSource::Modrinth,
            project_id: project_id.to_string(),
            version_id: "v1".to_string(),
            version_number: "1.0".to_string(),
            file_name: "mod.jar".to_string(),
            sha1: "abc".to_string(),
            installed_at: Utc::now(),
            explicit: true,
            title: None,
            icon_url: None,
            content_type: ContentType::Mod,
        }
    }

    #[test]
    fn install_dir_matches_content_type() {
        assert_eq!(ContentType::Mod.install_dir(), "mods");
        assert_eq!(ContentType::ResourcePack.install_dir(), "resourcepacks");
        assert_eq!(ContentType::Shader.install_dir(), "shaderpacks");
        assert_eq!(ContentType::Datapack.install_dir(), "datapacks");
    }

    #[test]
    fn only_mod_is_loader_locked() {
        assert!(ContentType::Mod.is_loader_locked());
        assert!(!ContentType::ResourcePack.is_loader_locked());
        assert!(!ContentType::Shader.is_loader_locked());
        assert!(!ContentType::Datapack.is_loader_locked());
    }

    #[test]
    fn upsert_replaces_entry_for_same_project_instead_of_duplicating() {
        let dir = std::env::temp_dir().join(format!("aurora-mods-lock-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let store = ModsLockStore::new(&dir);

        store.upsert(sample_entry("proj-a")).unwrap();
        let mut updated = sample_entry("proj-a");
        updated.version_id = "v2".to_string();
        store.upsert(updated).unwrap();

        let mods = store.read().unwrap();
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].version_id, "v2");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn remove_deletes_matching_entry_and_keeps_others() {
        let dir = std::env::temp_dir().join(format!("aurora-mods-lock-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let store = ModsLockStore::new(&dir);
        store.upsert(sample_entry("proj-a")).unwrap();
        store.upsert(sample_entry("proj-b")).unwrap();

        let removed = store.remove(ModSource::Modrinth, "proj-a").unwrap();
        assert!(removed.is_some());

        let mods = store.read().unwrap();
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].project_id, "proj-b");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn remove_on_missing_entry_returns_none() {
        let dir = std::env::temp_dir().join(format!("aurora-mods-lock-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let store = ModsLockStore::new(&dir);
        assert!(store.remove(ModSource::Modrinth, "nope").unwrap().is_none());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn read_on_missing_file_returns_empty() {
        let dir = std::env::temp_dir().join(format!("aurora-mods-lock-{}", uuid::Uuid::new_v4()));
        let store = ModsLockStore::new(&dir);
        assert!(store.read().unwrap().is_empty());
    }
}
