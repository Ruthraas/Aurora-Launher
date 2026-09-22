use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

use crate::core::accounts::AccountStore;
use crate::core::instances::install::{install, InstallStage, SharedCache};
use crate::core::instances::{launch, options_txt, Instance, InstanceStatus, InstanceStore, JvmFlagPreset, LoaderKind};
use crate::core::java;
use crate::core::loaders::{fabric, forge, neoforge, quilt};
use crate::core::minecraft::manifest;
use crate::error::{AppError, AppResult};

pub(crate) fn instance_store(app: &AppHandle) -> Result<InstanceStore, AppError> {
    Ok(InstanceStore::new(app.path().app_data_dir()?.join("instances")))
}

pub(crate) fn cache_root(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_data_dir()?.join("cache"))
}

fn build_loader_kind(loader_kind: &str, loader_version: Option<String>) -> Result<LoaderKind, AppError> {
    match loader_kind {
        "vanilla" => Ok(LoaderKind::Vanilla),
        "fabric" => {
            let loader_version = loader_version
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| AppError::InvalidInput("versão do Fabric Loader não informada".to_string()))?;
            Ok(LoaderKind::Fabric { loader_version })
        }
        "neoforge" => {
            let neoforge_version = loader_version
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| AppError::InvalidInput("versão do NeoForge não informada".to_string()))?;
            Ok(LoaderKind::NeoForge { neoforge_version })
        }
        "forge" => {
            let forge_version = loader_version
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| AppError::InvalidInput("versão do Forge não informada".to_string()))?;
            Ok(LoaderKind::Forge { forge_version })
        }
        "quilt" => {
            let loader_version = loader_version
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| AppError::InvalidInput("versão do Quilt Loader não informada".to_string()))?;
            Ok(LoaderKind::Quilt { loader_version })
        }
        other => Err(AppError::InvalidInput(format!("loader \"{other}\" não suportado ainda"))),
    }
}

#[tauri::command]
pub fn list_instances(app: AppHandle) -> AppResult<Vec<Instance>> {
    Ok(instance_store(&app)?.list()?)
}

#[tauri::command]
pub fn delete_instance(app: AppHandle, id: Uuid) -> AppResult<()> {
    instance_store(&app)?.delete(id)?;
    Ok(())
}

/// Versões do Fabric Loader compatíveis com `mc_version`, mais recente
/// primeiro — pra popular o seletor na tela de criar instância.
#[tauri::command]
pub async fn fetch_fabric_loader_versions(mc_version: String) -> AppResult<Vec<fabric::FabricLoaderEntry>> {
    Ok(fabric::fetch_loader_versions(&mc_version).await?)
}

/// Versões do NeoForge compatíveis com `mc_version`, mais recente
/// primeiro — pra popular o seletor na tela de criar instância.
#[tauri::command]
pub async fn fetch_neoforge_versions(mc_version: String) -> AppResult<Vec<String>> {
    Ok(neoforge::fetch_versions(&mc_version).await?)
}

/// Versões do Forge clássico compatíveis com `mc_version`, mais
/// recente primeiro — pra popular o seletor na tela de criar instância.
#[tauri::command]
pub async fn fetch_forge_versions(mc_version: String) -> AppResult<Vec<String>> {
    Ok(forge::fetch_versions(&mc_version).await?)
}

/// Versões do Quilt Loader compatíveis com `mc_version`, mais recente
/// primeiro — pra popular o seletor na tela de criar instância.
#[tauri::command]
pub async fn fetch_quilt_loader_versions(mc_version: String) -> AppResult<Vec<quilt::QuiltLoaderEntry>> {
    Ok(quilt::fetch_loader_versions(&mc_version).await?)
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InstallProgressPayload {
    instance_id: Uuid,
    stage: InstallStage,
    completed: usize,
    total: usize,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InstallFinishedPayload {
    instance_id: Uuid,
    ok: bool,
    error: Option<String>,
}

/// Dispara a instalação numa task solta, reportando progresso via
/// eventos `instance://install-progress` e `instance://install-finished`
/// — pode levar minutos (o SharedCache já pula o que já está correto
/// no disco, então repetir depois de uma falha é rápido pro que já
/// tinha baixado).
fn spawn_install(app: &AppHandle, instance_id: Uuid, mc_version: String, loader: LoaderKind) {
    let app_task = app.clone();

    tauri::async_runtime::spawn(async move {
        let Ok(store) = instance_store(&app_task) else { return };
        let Ok(cache_root) = cache_root(&app_task) else { return };
        let cache = SharedCache { root: &cache_root };

        let app_events = app_task.clone();
        let result = install(&cache, &mc_version, &loader, move |stage, completed, total| {
            let _ = app_events.emit(
                "instance://install-progress",
                InstallProgressPayload { instance_id, stage, completed, total },
            );
        })
        .await;

        let result = result.and_then(|version| {
            let _ = app_task.emit(
                "instance://install-progress",
                InstallProgressPayload { instance_id, stage: InstallStage::OptionsFile, completed: 0, total: 1 },
            );
            options_txt::apply(&store.instance_dir(instance_id))?;
            let _ = app_task.emit(
                "instance://install-progress",
                InstallProgressPayload { instance_id, stage: InstallStage::OptionsFile, completed: 1, total: 1 },
            );
            Ok(version)
        });

        let ok = result.is_ok();
        let error_message = result.err().map(|error| error.to_string());

        if let Ok(mut updated) = store.get(instance_id) {
            updated.status = if ok { InstanceStatus::Ready } else { InstanceStatus::Error };
            updated.error_message = error_message.clone();
            let _ = store.save(&updated);
        }
        let _ = app_task.emit(
            "instance://install-finished",
            InstallFinishedPayload { instance_id, ok, error: error_message },
        );
    });
}

/// Cria a instância (Vanilla ou Fabric) e já devolve — a instalação de
/// verdade roda solta, ver `spawn_install`.
#[tauri::command]
pub fn create_instance(
    app: AppHandle,
    name: String,
    mc_version: String,
    loader_kind: String,
    loader_version: Option<String>,
    ram_min_mb: u32,
    ram_max_mb: u32,
) -> AppResult<Instance> {
    if ram_min_mb == 0 || ram_max_mb < ram_min_mb {
        return Err(AppError::InvalidInput("intervalo de memória inválido".to_string()).into());
    }
    let loader = build_loader_kind(&loader_kind, loader_version)?;

    let store = instance_store(&app)?;
    let mut instance = store.create(name, mc_version.clone(), loader.clone(), ram_min_mb, ram_max_mb)?;
    instance.status = InstanceStatus::Installing;
    store.save(&instance)?;

    spawn_install(&app, instance.id, mc_version, loader);

    Ok(instance)
}

/// Tenta de novo a instalação de uma instância que ficou em `error`
/// (normalmente uma falha de rede pontual) — sem recriar nada.
#[tauri::command]
pub fn retry_instance_install(app: AppHandle, id: Uuid) -> AppResult<Instance> {
    let store = instance_store(&app)?;
    let mut instance = store.get(id)?;
    instance.status = InstanceStatus::Installing;
    instance.error_message = None;
    store.save(&instance)?;

    spawn_install(&app, instance.id, instance.mc_version.clone(), instance.loader.clone());

    Ok(instance)
}

/// Lança a instância com a conta ativa no momento. Reconfere o runtime
/// Java (idempotente — não rebaixa nada que já esteja certo) antes de
/// chamar `core::instances::launch::launch`.
#[tauri::command]
pub async fn launch_instance(app: AppHandle, id: Uuid) -> AppResult<()> {
    let store = instance_store(&app)?;
    let instance = store.get(id)?;

    let accounts_store = AccountStore::new(&app)?;
    let (accounts, active_account_id) = accounts_store.list()?;
    let active_account_id =
        active_account_id.ok_or_else(|| AppError::InvalidInput("nenhuma conta ativa".to_string()))?;
    let account = accounts
        .into_iter()
        .find(|account| account.id() == active_account_id)
        .ok_or_else(|| AppError::NotFound("conta ativa".to_string()))?;

    let manifest = manifest::fetch_version_manifest().await?;
    let entry = manifest
        .versions
        .iter()
        .find(|entry| entry.id == instance.mc_version)
        .ok_or_else(|| AppError::NotFound(format!("versão do Minecraft \"{}\"", instance.mc_version)))?;
    let version = manifest::fetch_version_details(&entry.url).await?;

    let cache_root = cache_root(&app)?;
    let cache = SharedCache { root: &cache_root };
    let java_path = java::ensure_runtime(&cache.runtimes_dir(), version.required_java_component(), |_, _| {}).await?;

    let instance_dir = store.instance_dir(id);
    launch::launch(&instance, &version, &account, &java_path, &instance_dir, &cache).await?;

    let mut updated = instance;
    updated.last_played = Some(chrono::Utc::now());
    store.save(&updated)?;

    Ok(())
}

/// Ajustes editáveis depois de criada — RAM e o preset de flags de
/// JVM (ver "Visão geral" da tela de detalhe da instância).
#[tauri::command]
pub fn update_instance_settings(
    app: AppHandle,
    id: Uuid,
    ram_min_mb: u32,
    ram_max_mb: u32,
    jvm_flag_preset: JvmFlagPreset,
) -> AppResult<Instance> {
    if ram_min_mb == 0 || ram_max_mb < ram_min_mb {
        return Err(AppError::InvalidInput("intervalo de memória inválido".to_string()).into());
    }
    let store = instance_store(&app)?;
    let mut instance = store.get(id)?;
    instance.ram_min_mb = ram_min_mb;
    instance.ram_max_mb = ram_max_mb;
    instance.jvm_flag_preset = jvm_flag_preset;
    store.save(&instance)?;
    Ok(instance)
}

/// Abre a pasta da instância no explorador de arquivos do SO.
#[tauri::command]
pub fn open_instance_folder(app: AppHandle, id: Uuid) -> AppResult<()> {
    let store = instance_store(&app)?;
    let dir = store.instance_dir(id);
    std::fs::create_dir_all(&dir).map_err(AppError::from)?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| AppError::InvalidInput(e.to_string()))?;
    Ok(())
}
