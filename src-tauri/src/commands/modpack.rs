use tauri::{AppHandle, State};
use uuid::Uuid;

use super::instances::{cache_root, instance_store};
use crate::core::instances::install::{install as install_base, SharedCache};
use crate::core::instances::mods::{ModSource, ModpackOrigin};
use crate::core::instances::options_txt;
use crate::core::instances::InstanceStatus;
use crate::core::jobs::{emit_failed, emit_progress, JobPhase, JobRegistry};
use crate::core::modpack::{self, ModpackPreview};
use crate::core::mods_compat::ProjectRef;
use crate::core::settings::SettingsStore;
use crate::error::{AppError, AppResult};

/// Prévia de um modpack pro modal "Baixar modpack" — versão do
/// Minecraft, loader, contagem de mods, tamanho estimado do download,
/// RAM sugerida. Baixa o próprio pacote (pequeno: só metadados +
/// overrides, os mods em si são baixados depois na instalação) porque
/// é o único jeito de saber a lista de arquivos real.
#[tauri::command]
pub async fn get_modpack_preview(app: AppHandle, project: ProjectRef, version: Option<String>) -> AppResult<ModpackPreview> {
    let cache_root = cache_root(&app)?;
    match project.source {
        ModSource::Modrinth => Ok(modpack::preview_modrinth(&cache_root, &project.project_id, version.as_deref()).await?),
        ModSource::Curseforge => {
            let api_key = SettingsStore::new(&app)?
                .read()?
                .curseforge_api_key
                .ok_or_else(|| AppError::InvalidInput("chave da API do CurseForge não configurada".to_string()))?;
            let mod_id: u32 = project
                .project_id
                .parse()
                .map_err(|_| AppError::InvalidInput(format!("id de modpack do CurseForge inválido: {}", project.project_id)))?;
            let file_id = version.and_then(|v| v.parse().ok());
            Ok(modpack::preview_curseforge(&cache_root, &api_key, mod_id, file_id).await?)
        }
    }
}

/// Cria a instância e baixa o modpack inteiro em background —
/// progresso via `job://progress` (mesmo evento que `install_mod`).
#[tauri::command]
pub async fn install_modpack(
    app: AppHandle,
    registry: State<'_, JobRegistry>,
    project: ProjectRef,
    version_id: String,
    name: String,
) -> AppResult<Uuid> {
    let handle = registry.start();
    let job_id = handle.id;
    let registry = registry.inner().clone();
    let app_task = app.clone();

    tauri::async_runtime::spawn(async move {
        let result = run_install_modpack(&app_task, &handle, &project, &version_id, &name).await;
        if let Err(error) = result {
            emit_failed(&app_task, job_id, error.to_string());
        }
        registry.finish(job_id);
    });

    Ok(job_id)
}

async fn run_install_modpack(
    app: &AppHandle,
    handle: &crate::core::jobs::JobHandle,
    project: &ProjectRef,
    version_id: &str,
    name: &str,
) -> Result<(), AppError> {
    emit_progress(app, handle.id, JobPhase::FetchingMetadata, 0, 1);

    let cache_root_path = cache_root(app)?;
    let store = instance_store(app)?;
    let api_key = SettingsStore::new(app)?.read()?.curseforge_api_key;

    let preview = match project.source {
        ModSource::Modrinth => modpack::preview_modrinth(&cache_root_path, &project.project_id, Some(version_id)).await?,
        ModSource::Curseforge => {
            let api_key =
                api_key.clone().ok_or_else(|| AppError::InvalidInput("chave da API do CurseForge não configurada".to_string()))?;
            let mod_id: u32 = project
                .project_id
                .parse()
                .map_err(|_| AppError::InvalidInput(format!("id de modpack do CurseForge inválido: {}", project.project_id)))?;
            modpack::preview_curseforge(&cache_root_path, &api_key, mod_id, version_id.parse().ok()).await?
        }
    };

    emit_progress(app, handle.id, JobPhase::CreatingInstance, 0, 1);
    let unique_name = store.unique_name(name)?;
    let mut instance = store.create(unique_name, preview.mc_version.clone(), preview.loader.clone(), 1024, preview.suggested_ram_mb)?;
    instance.status = InstanceStatus::Installing;
    instance.modpack_origin = Some(ModpackOrigin { source: project.source, project_id: project.project_id.clone(), version_id: version_id.to_string() });
    store.save(&instance)?;

    if handle.is_cancelled() {
        store.delete(instance.id)?;
        emit_progress(app, handle.id, JobPhase::Cancelled, 0, 1);
        return Ok(());
    }

    let instance_dir = store.instance_dir(instance.id);
    let cache = SharedCache { root: &cache_root_path };

    let install_result = install_base(&cache, &preview.mc_version, &preview.loader, |_stage, done, total| {
        emit_progress(app, handle.id, JobPhase::Downloading, done, total.max(1));
    })
    .await;

    if let Err(error) = install_result {
        instance.status = InstanceStatus::Error;
        instance.error_message = Some(error.to_string());
        store.save(&instance)?;
        return Err(error);
    }

    if handle.is_cancelled() {
        emit_progress(app, handle.id, JobPhase::Cancelled, 0, 1);
        return Ok(());
    }

    let mut missing = Vec::new();
    match project.source {
        ModSource::Modrinth => {
            let mrpack_path = cache_root_path.join("modpack-cache").join("modrinth").join(format!("{}.mrpack", preview.version_id));
            modpack::install_modrinth_files(&cache_root_path, &mrpack_path, &instance_dir, |done, total| {
                emit_progress(app, handle.id, JobPhase::Downloading, done, total);
            })
            .await?;
        }
        ModSource::Curseforge => {
            let api_key =
                api_key.ok_or_else(|| AppError::InvalidInput("chave da API do CurseForge não configurada".to_string()))?;
            let pack_path =
                cache_root_path.join("modpack-cache").join("curseforge").join(format!("{}-{}.zip", project.project_id, preview.version_id));
            let outcome = modpack::install_curseforge_files(&api_key, &pack_path, &instance_dir, |done, total| {
                emit_progress(app, handle.id, JobPhase::Downloading, done, total);
            })
            .await?;
            missing = outcome.missing;
        }
    }

    emit_progress(app, handle.id, JobPhase::Extracting, 1, 1);

    emit_progress(app, handle.id, JobPhase::ConfiguringOptions, 0, 1);
    // Aplicado por cima de tudo, depois dos overrides do modpack —
    // se o autor do pack já trouxe um options.txt próprio, isso só
    // sobrescreve as chaves de desempenho, preserva o resto (ver
    // core::instances::options_txt::apply).
    options_txt::apply(&instance_dir)?;
    emit_progress(app, handle.id, JobPhase::ConfiguringOptions, 1, 1);

    instance.status = if missing.is_empty() { InstanceStatus::Ready } else { InstanceStatus::Incomplete };
    instance.missing_manual_downloads = missing;
    store.save(&instance)?;

    emit_progress(app, handle.id, JobPhase::Finished, 1, 1);
    Ok(())
}
