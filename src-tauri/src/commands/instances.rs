use std::path::{Path, PathBuf};

use base64::Engine;
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
    custom_jvm_args: Option<String>,
) -> AppResult<Instance> {
    if ram_min_mb == 0 || ram_max_mb < ram_min_mb {
        return Err(AppError::InvalidInput("intervalo de memória inválido".to_string()).into());
    }
    let store = instance_store(&app)?;
    let mut instance = store.get(id)?;
    instance.ram_min_mb = ram_min_mb;
    instance.ram_max_mb = ram_max_mb;
    instance.jvm_flag_preset = jvm_flag_preset;
    if let Some(custom_jvm_args) = custom_jvm_args {
        instance.custom_jvm_args = custom_jvm_args.trim().to_string();
    }
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

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LogFileInfo {
    file_name: String,
    size_bytes: u64,
    modified_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Diretórios onde arquivo de log pode aparecer numa instância:
/// `logs/` (o launcher sempre grava `launcher-stdout.log`/
/// `launcher-stderr.log` ali, ver `core::instances::launch`; o próprio
/// Minecraft, via log4j, grava `latest.log`/`debug.log` no mesmo
/// lugar) e `crash-reports/` (relatório de crash do jogo, formato
/// vanilla padrão — nome do arquivo tem timestamp, não é fixo).
const LOG_DIRS: &[&str] = &["logs", "crash-reports"];

/// Lista os arquivos de log/crash-report disponíveis pra uma
/// instância, mais recente primeiro — pra popular a aba "Logs" da tela
/// de detalhe sem o usuário precisar navegar manualmente até a pasta.
#[tauri::command]
pub fn list_instance_logs(app: AppHandle, id: Uuid) -> AppResult<Vec<LogFileInfo>> {
    let store = instance_store(&app)?;
    let instance_dir = store.instance_dir(id);

    let mut files = Vec::new();
    for dir_name in LOG_DIRS {
        let dir = instance_dir.join(dir_name);
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else { continue };
            if !metadata.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_string) else { continue };
            let modified_at = metadata.modified().ok().map(chrono::DateTime::<chrono::Utc>::from);
            files.push(LogFileInfo { file_name: format!("{dir_name}/{name}"), size_bytes: metadata.len(), modified_at });
        }
    }
    files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(files)
}

const MAX_LOG_READ_BYTES: u64 = 512 * 1024;

/// Lê o conteúdo de um log listado por `list_instance_logs` — só os
/// últimos `MAX_LOG_READ_BYTES` (log de sessão longa pode ter dezenas
/// de MB; a UI só precisa do final pra diagnosticar um crash recente).
/// `relative_path` tem que ser exatamente um dos valores devolvidos por
/// `list_instance_logs` (`"<pasta>/<arquivo>"`) — validado contra
/// `LOG_DIRS` e sem `..`/separador extra, pra não virar um jeito de ler
/// qualquer arquivo arbitrário do disco a partir do front.
#[tauri::command]
pub fn read_instance_log(app: AppHandle, id: Uuid, relative_path: String) -> AppResult<String> {
    let (dir_name, file_name) = relative_path
        .split_once('/')
        .ok_or_else(|| AppError::InvalidInput("caminho de log inválido".to_string()))?;
    if !LOG_DIRS.contains(&dir_name) || file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err(AppError::InvalidInput("caminho de log inválido".to_string()).into());
    }

    let store = instance_store(&app)?;
    let path = store.instance_dir(id).join(dir_name).join(file_name);
    let bytes = std::fs::read(&path).map_err(AppError::from)?;

    let tail = if bytes.len() as u64 > MAX_LOG_READ_BYTES { &bytes[bytes.len() - MAX_LOG_READ_BYTES as usize..] } else { &bytes[..] };
    Ok(String::from_utf8_lossy(tail).into_owned())
}

/// Exporta a instância pra um `.zip` (config, mods, saves — NÃO as
/// bibliotecas/assets do cache compartilhado, ver
/// `core::instances::transfer::export_instance`) dentro de
/// `<dados do app>/exports/`, e já abre essa pasta no explorador — sem
/// diálogo nativo de "salvar como" (o app não tem esse plugin), então
/// o destino é fixo e previsível em vez de perguntar onde salvar.
#[tauri::command]
pub fn export_instance(app: AppHandle, id: Uuid) -> AppResult<String> {
    let store = instance_store(&app)?;
    let instance = store.get(id)?;

    let exports_dir = app.path().app_data_dir().map_err(AppError::from)?.join("exports");
    std::fs::create_dir_all(&exports_dir).map_err(AppError::from)?;

    let safe_name: String =
        instance.name.trim().chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).collect();
    let dest = exports_dir.join(format!("{safe_name}.aurorapack.zip"));
    crate::core::instances::transfer::export_instance(&store, id, &dest)?;

    // não é crítico se o explorador não abrir (ex.: SO sem GUI) — o
    // caminho de volta pro front já é o suficiente pra UI mostrar
    // "exportado em <caminho>" de qualquer jeito.
    let _ = app.opener().open_path(exports_dir.to_string_lossy(), None::<&str>);

    Ok(dest.to_string_lossy().into_owned())
}

/// Importa um `.zip` exportado por `export_instance` — os bytes vêm
/// inteiros pelo IPC (mesmo padrão já usado pra upload de skin/capa,
/// `commands::accounts`), gravados num arquivo temporário só pra
/// `transfer::import_instance` ler como zip de verdade. Sem diálogo
/// nativo de "abrir arquivo", o front usa um `<input type="file">`
/// normal e manda os bytes.
#[tauri::command]
pub fn import_instance_from_bytes(app: AppHandle, zip_bytes: Vec<u8>) -> AppResult<Instance> {
    let tmp_path = std::env::temp_dir().join(format!("aurora-import-{}.zip", Uuid::new_v4()));
    std::fs::write(&tmp_path, &zip_bytes).map_err(AppError::from)?;

    let store = instance_store(&app)?;
    let result = crate::core::instances::transfer::import_instance(&store, &tmp_path);
    let _ = std::fs::remove_file(&tmp_path);
    let instance = result?;

    // bibliotecas/assets/client.jar/Java não vêm no zip (ficam no
    // cache compartilhado) — dispara a mesma instalação usada em
    // "criar instância"/"tentar de novo", que pula o que já estiver
    // certo no cache local.
    spawn_install(&app, instance.id, instance.mc_version.clone(), instance.loader.clone());

    Ok(instance)
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfo {
    folder_name: String,
    size_bytes: u64,
    last_played: Option<chrono::DateTime<chrono::Utc>>,
    has_icon: bool,
}

fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else { return 0 };
    entries
        .flatten()
        .map(|entry| {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                dir_size(&entry_path)
            } else {
                entry.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

/// Lista os mundos salvos (`saves/<pasta>/`) de uma instância — cada
/// pasta de save vira uma entrada, mais recente primeiro (data de
/// modificação do `level.dat`, que o próprio Minecraft reescreve toda
/// vez que salva o jogo). O nome de exibição é o nome da PASTA — ler o
/// nome "de verdade" exigiria decodificar NBT binário do `level.dat`
/// (formato próprio, comprimido em gzip) só pra isso, e a pasta já é o
/// nome que o jogador escolheu na tela de criar mundo na esmagadora
/// maioria dos casos.
#[tauri::command]
pub fn list_instance_worlds(app: AppHandle, id: Uuid) -> AppResult<Vec<WorldInfo>> {
    let store = instance_store(&app)?;
    let saves_dir = store.instance_dir(id).join("saves");

    let mut worlds = Vec::new();
    let Ok(entries) = std::fs::read_dir(&saves_dir) else { return Ok(worlds) };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(folder_name) = path.file_name().and_then(|n| n.to_str()).map(str::to_string) else { continue };
        let last_played = std::fs::metadata(path.join("level.dat")).ok().and_then(|m| m.modified().ok()).map(chrono::DateTime::<chrono::Utc>::from);
        worlds.push(WorldInfo { folder_name, size_bytes: dir_size(&path), last_played, has_icon: path.join("icon.png").exists() });
    }
    worlds.sort_by(|a, b| b.last_played.cmp(&a.last_played));
    Ok(worlds)
}

fn validate_world_folder_name(folder_name: &str) -> AppResult<()> {
    if folder_name.is_empty() || folder_name.contains('/') || folder_name.contains('\\') || folder_name.contains("..") {
        return Err(AppError::InvalidInput("nome de mundo inválido".to_string()).into());
    }
    Ok(())
}

/// Apaga um mundo — sem confirmação nenhuma aqui, a UI que decide
/// (mesma disciplina de `delete_instance`: irreversível, confirma
/// antes de chamar isso).
#[tauri::command]
pub fn delete_instance_world(app: AppHandle, id: Uuid, folder_name: String) -> AppResult<()> {
    validate_world_folder_name(&folder_name)?;
    let store = instance_store(&app)?;
    let world_dir = store.instance_dir(id).join("saves").join(&folder_name);
    if !world_dir.exists() {
        return Err(AppError::NotFound(format!("mundo \"{folder_name}\"")).into());
    }
    std::fs::remove_dir_all(&world_dir).map_err(AppError::from)?;
    Ok(())
}

/// Abre a pasta de UM mundo específico no explorador — mais direto que
/// "Abrir pasta" da instância inteira quando o que se quer é olhar/
/// mexer nos arquivos de um save só.
#[tauri::command]
pub fn open_instance_world_folder(app: AppHandle, id: Uuid, folder_name: String) -> AppResult<()> {
    validate_world_folder_name(&folder_name)?;
    let store = instance_store(&app)?;
    let world_dir = store.instance_dir(id).join("saves").join(&folder_name);
    std::fs::create_dir_all(&world_dir).map_err(AppError::from)?;
    app.opener().open_path(world_dir.to_string_lossy(), None::<&str>).map_err(|e| AppError::InvalidInput(e.to_string()))?;
    Ok(())
}

/// Ícone do mundo (`icon.png`, PNG que o próprio Minecraft grava
/// quando o jogador tira um) em base64 — mesmo padrão já usado pra
/// textura de skin/capa (`commands::accounts::get_account_texture`).
#[tauri::command]
pub fn get_world_icon(app: AppHandle, id: Uuid, folder_name: String) -> AppResult<Option<String>> {
    validate_world_folder_name(&folder_name)?;
    let store = instance_store(&app)?;
    let icon_path = store.instance_dir(id).join("saves").join(&folder_name).join("icon.png");
    if !icon_path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&icon_path).map_err(AppError::from)?;
    Ok(Some(base64::engine::general_purpose::STANDARD.encode(bytes)))
}
