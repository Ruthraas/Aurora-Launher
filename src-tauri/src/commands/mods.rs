use serde::Serialize;
use tauri::{AppHandle, State};
use uuid::Uuid;

use super::instances::instance_store;
use crate::core::instances::mods::{ModLockEntry, ModSource, ModsLockStore};
use crate::core::instances::options_txt;
use crate::core::jobs::{emit_failed, emit_progress, JobPhase, JobRegistry};
use crate::core::mods_compat::{self, InstanceCompat, ModRef, ProjectRef};
use crate::core::optimization::{self, OptimizationMod};
use crate::core::settings::SettingsStore;
use crate::error::{AppError, AppResult};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceCompatEntry {
    instance_id: Uuid,
    compat: InstanceCompat,
}

/// Calcula, pra cada instância existente, se `project` pode ser
/// instalado nela — ver `InstanceCompat`. Consulta as APIs dos
/// provedores (uma chamada por instância, mais uma por dependência
/// obrigatória resolvida) — pode demorar um pouco com muitas
/// instâncias, mas não há como saber a resposta sem perguntar pra API.
#[tauri::command]
pub async fn get_mod_compatibility(app: AppHandle, project: ProjectRef) -> AppResult<Vec<InstanceCompatEntry>> {
    let store = instance_store(&app)?;
    let instances = store.list()?;
    let cf_api_key = SettingsStore::new(&app)?.read()?.curseforge_api_key;

    let mut out = Vec::with_capacity(instances.len());
    for instance in instances {
        let instance_dir = store.instance_dir(instance.id);
        let compat = mods_compat::compute_compatibility(&instance, &instance_dir, &project, cf_api_key.as_deref()).await?;
        out.push(InstanceCompatEntry { instance_id: instance.id, compat });
    }
    Ok(out)
}

/// Instala `project` nas instâncias de `instance_ids` (mod principal +
/// dependências obrigatórias resolvidas) — roda solto, progresso via
/// `job://progress`. Recalcula a compatibilidade de cada instância na
/// hora (não confia num resultado antigo vindo do front) e pula
/// silenciosamente qualquer instância que não estiver mais compatível.
#[tauri::command]
pub async fn install_mod(
    app: AppHandle,
    registry: State<'_, JobRegistry>,
    project: ProjectRef,
    instance_ids: Vec<Uuid>,
) -> AppResult<Uuid> {
    let handle = registry.start();
    let job_id = handle.id;
    let registry = registry.inner().clone();
    let app_task = app.clone();

    tauri::async_runtime::spawn(async move {
        let result = run_install_mod(&app_task, &handle, &project, &instance_ids).await;
        if let Err(error) = result {
            emit_failed(&app_task, job_id, error.to_string());
        }
        registry.finish(job_id);
    });

    Ok(job_id)
}

async fn run_install_mod(
    app: &AppHandle,
    handle: &crate::core::jobs::JobHandle,
    project: &ProjectRef,
    instance_ids: &[Uuid],
) -> Result<(), AppError> {
    let store = instance_store(app)?;
    let cf_api_key = SettingsStore::new(app)?.read()?.curseforge_api_key;

    let total = instance_ids.len();
    emit_progress(app, handle.id, JobPhase::FetchingMetadata, 0, total);

    for (i, instance_id) in instance_ids.iter().enumerate() {
        if handle.is_cancelled() {
            emit_progress(app, handle.id, JobPhase::Cancelled, i, total);
            return Ok(());
        }

        let instance = store.get(*instance_id)?;
        let instance_dir = store.instance_dir(*instance_id);
        let compat = mods_compat::compute_compatibility(&instance, &instance_dir, project, cf_api_key.as_deref()).await?;

        if let InstanceCompat::Compatible { mod_ref, deps_to_install } = compat {
            install_with_deps(&instance_dir, &mod_ref, &deps_to_install).await?;
        }
        // instância deixou de ser compatível (ou já tinha o mod) entre o
        // clique do usuário e o job rodar — pula em silêncio, o
        // resultado final (mods.lock.json) reflete o estado real.

        emit_progress(app, handle.id, JobPhase::Downloading, i + 1, total);
    }

    emit_progress(app, handle.id, JobPhase::Finished, total, total);
    Ok(())
}

async fn install_with_deps(instance_dir: &std::path::Path, mod_ref: &ModRef, deps: &[ModRef]) -> Result<(), AppError> {
    for dep in deps {
        mods_compat::install_into_instance(instance_dir, dep, false).await?;
    }
    mods_compat::install_into_instance(instance_dir, mod_ref, true).await
}

#[tauri::command]
pub fn cancel_job(registry: State<'_, JobRegistry>, job_id: Uuid) {
    registry.cancel(job_id);
}

/// Lista o que está instalado numa instância — direto do
/// `mods.lock.json`, sem reconsultar API nenhuma (é o "como eu sei o
/// que tem aqui dentro" da tela de detalhe da instância).
#[tauri::command]
pub fn list_instance_mods(app: AppHandle, instance_id: Uuid) -> AppResult<Vec<ModLockEntry>> {
    let store = instance_store(&app)?;
    let instance_dir = store.instance_dir(instance_id);
    Ok(ModsLockStore::new(&instance_dir).read()?)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationEntry {
    #[serde(flatten)]
    info: OptimizationMod,
    compat: InstanceCompat,
}

/// Lista curada de mods de otimização pro loader da instância (Fabric/
/// Quilt e Forge/NeoForge têm ecossistemas diferentes — ver
/// `core::optimization`), cada um já com o status de compatibilidade
/// calculado pra essa instância específica.
#[tauri::command]
pub async fn get_recommended_optimizations(app: AppHandle, instance_id: Uuid) -> AppResult<Vec<OptimizationEntry>> {
    let store = instance_store(&app)?;
    let instance = store.get(instance_id)?;
    let instance_dir = store.instance_dir(instance_id);
    let cf_api_key = SettingsStore::new(&app)?.read()?.curseforge_api_key;

    let mut out = Vec::new();
    for info in optimization::recommended_for(&instance.loader) {
        let project = ProjectRef {
            source: ModSource::Modrinth,
            project_id: info.project_id.to_string(),
            content_type: mods_compat::ContentType::Mod,
            title: Some(info.name.to_string()),
            icon_url: None,
        };
        let compat = mods_compat::compute_compatibility(&instance, &instance_dir, &project, cf_api_key.as_deref()).await?;
        out.push(OptimizationEntry { info: *info, compat });
    }
    Ok(out)
}

/// Instala os mods de otimização escolhidos (por `project_id`, slug do
/// Modrinth) numa instância só — mesmo job/evento de progresso que
/// `install_mod`, só que iterando mods em vez de instâncias.
#[tauri::command]
pub async fn install_optimizations(
    app: AppHandle,
    registry: State<'_, JobRegistry>,
    instance_id: Uuid,
    project_ids: Vec<String>,
) -> AppResult<Uuid> {
    let handle = registry.start();
    let job_id = handle.id;
    let registry_task = registry.inner().clone();
    let app_task = app.clone();

    tauri::async_runtime::spawn(async move {
        let result = run_install_optimizations(&app_task, &handle, instance_id, &project_ids).await;
        if let Err(error) = result {
            emit_failed(&app_task, job_id, error.to_string());
        }
        registry_task.finish(job_id);
    });

    Ok(job_id)
}

async fn run_install_optimizations(
    app: &AppHandle,
    handle: &crate::core::jobs::JobHandle,
    instance_id: Uuid,
    project_ids: &[String],
) -> Result<(), AppError> {
    let store = instance_store(app)?;
    let instance = store.get(instance_id)?;
    let instance_dir = store.instance_dir(instance_id);
    let cf_api_key = SettingsStore::new(app)?.read()?.curseforge_api_key;

    let total = project_ids.len();
    emit_progress(app, handle.id, JobPhase::FetchingMetadata, 0, total);

    for (i, project_id) in project_ids.iter().enumerate() {
        if handle.is_cancelled() {
            emit_progress(app, handle.id, JobPhase::Cancelled, i, total);
            return Ok(());
        }

        let title = optimization::recommended_for(&instance.loader)
            .iter()
            .find(|m| m.project_id == project_id)
            .map(|m| m.name.to_string());
        let project = ProjectRef {
            source: ModSource::Modrinth,
            project_id: project_id.clone(),
            content_type: mods_compat::ContentType::Mod,
            title,
            icon_url: None,
        };
        let compat = mods_compat::compute_compatibility(&instance, &instance_dir, &project, cf_api_key.as_deref()).await?;
        if let InstanceCompat::Compatible { mod_ref, deps_to_install } = compat {
            install_with_deps(&instance_dir, &mod_ref, &deps_to_install).await?;
        }

        emit_progress(app, handle.id, JobPhase::Downloading, i + 1, total);
    }

    emit_progress(app, handle.id, JobPhase::ConfiguringOptions, 0, 1);
    // Clicar em "Otimizar desempenho" numa instância já existente
    // também aplica o preset de options.txt — não só instâncias
    // novas ficam com o ajuste (pedido explícito do usuário: "ter em
    // todos").
    options_txt::apply(&instance_dir)?;
    emit_progress(app, handle.id, JobPhase::ConfiguringOptions, 1, 1);

    emit_progress(app, handle.id, JobPhase::Finished, total, total);
    Ok(())
}

/// Remove um item instalado: apaga o arquivo (na pasta certa pro tipo
/// — `mods/`, `resourcepacks/`, `shaderpacks/`, `datapacks/`) E a
/// entrada do lock — os dois, nunca só um (senão ou sobra arquivo
/// órfão, ou o lock mente sobre o que existe de verdade). Bug real já
/// corrigido aqui: antes sempre olhava só em `mods/`, então remover um
/// resourcepack/shader/datapack apagava a entrada do lock mas deixava
/// o arquivo de verdade órfão em disco pra sempre.
#[tauri::command]
pub fn remove_instance_mod(app: AppHandle, instance_id: Uuid, source: ModSource, project_id: String) -> AppResult<()> {
    let store = instance_store(&app)?;
    let instance_dir = store.instance_dir(instance_id);
    let lock_store = ModsLockStore::new(&instance_dir);

    if let Some(entry) = lock_store.remove(source, &project_id)? {
        let path = instance_dir.join(entry.content_type.install_dir()).join(&entry.file_name);
        if path.exists() {
            std::fs::remove_file(path).map_err(AppError::from)?;
        }
    }
    Ok(())
}
