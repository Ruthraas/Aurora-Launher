use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use chrono::Utc;

use crate::core::curseforge;
use crate::core::download::{download_file, extract_prefixed, read_zip_entry};
use crate::core::instances::mods::{ContentType, ModLockEntry, ModSource, ModsLockStore};
use crate::core::instances::LoaderKind;
use crate::core::modrinth;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackVersionOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackPreview {
    pub version_id: String,
    pub mc_version: String,
    pub loader: LoaderKind,
    pub mod_count: usize,
    pub download_size_bytes: u64,
    pub suggested_ram_mb: u32,
    pub versions: Vec<ModpackVersionOption>,
}

/// Sem número oficial pra isso — heurística: base de 1.5GB + ~24MB por
/// mod (cobre a pegada média de um jar de mod na memória), com piso e
/// teto razoáveis pra não sugerir algo absurdo num pack minúsculo ou
/// gigante.
fn suggest_ram_mb(mod_count: usize) -> u32 {
    (1536 + mod_count as u32 * 24).clamp(2048, 10240)
}

fn cache_dir(cache_root: &Path, source: ModSource) -> PathBuf {
    let name = match source {
        ModSource::Modrinth => "modrinth",
        ModSource::Curseforge => "curseforge",
    };
    cache_root.join("modpack-cache").join(name)
}

// ---------------------------------------------------------------------
// Modrinth (.mrpack)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct MrpackIndex {
    dependencies: HashMap<String, String>,
    files: Vec<MrpackFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct MrpackFile {
    path: String,
    hashes: MrpackHashes,
    downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    file_size: u64,
    env: Option<MrpackEnv>,
}

#[derive(Debug, Clone, Deserialize)]
struct MrpackHashes {
    sha1: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MrpackEnv {
    client: String,
}

/// O `.mrpack` NÃO traz `project_id`/`version_id` por arquivo (só
/// `path`/`hashes`/`downloads`/`fileSize`/`env` — confirmado no
/// formato real) — é por isso que, sem isso, os mods de um modpack do
/// Modrinth nunca entravam em `mods.lock.json`, quebrando toda a
/// detecção de "já instalado" pra eles. Mas a URL de download de
/// arquivos hospedados no próprio CDN do Modrinth segue um padrão
/// fixo e documentado: `cdn.modrinth.com/data/<project_id>/versions/
/// <version_id>/<nome>` (confirmado baixando a versão real do
/// Sodium) — então dá pra extrair os dois dali. Arquivos hospedados em
/// outro lugar (raro, mas o formato permite) simplesmente não entram
/// no lock — continuam instalados, só não ficam rastreados por
/// `(source, project_id)`.
fn parse_modrinth_cdn_ids(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("https://cdn.modrinth.com/data/")?;
    let mut parts = rest.splitn(4, '/');
    let project_id = parts.next()?.to_string();
    if parts.next()? != "versions" {
        return None;
    }
    let version_id = parts.next()?.to_string();
    Some((project_id, version_id))
}

/// As chaves de `dependencies` do `modrinth.index.json` pro loader são
/// fixas por especificação do formato `.mrpack` (`fabric-loader`,
/// `forge`, `neoforge`, `quilt-loader`) — confirmado baixando um pack
/// real (Create+, dependencies: {"minecraft":"1.21.1","neoforge":"21.1.233"}).
fn mrpack_loader(deps: &HashMap<String, String>) -> Result<LoaderKind, AppError> {
    if let Some(v) = deps.get("neoforge") {
        return Ok(LoaderKind::NeoForge { neoforge_version: v.clone() });
    }
    if let Some(v) = deps.get("forge") {
        // A chave "forge" do modrinth.index.json só traz o número do
        // build (ex.: "47.4.13"), mas o instalador real do Forge
        // exige a versão combinada "<mc_version>-<build>" (é assim que
        // o Maven do Forge organiza os artefatos — confirmado contra
        // `installer_url` em core/loaders/forge/mod.rs). Sem isso a
        // URL do instalador vem 404: bug real, reportado pelo usuário
        // instalando um modpack de verdade.
        let mc_version = deps
            .get("minecraft")
            .ok_or_else(|| AppError::InvalidInput("modrinth.index.json sem versão do Minecraft".to_string()))?;
        return Ok(LoaderKind::Forge { forge_version: format!("{mc_version}-{v}") });
    }
    if let Some(v) = deps.get("fabric-loader") {
        return Ok(LoaderKind::Fabric { loader_version: v.clone() });
    }
    if let Some(v) = deps.get("quilt-loader") {
        return Ok(LoaderKind::Quilt { loader_version: v.clone() });
    }
    Err(AppError::InvalidInput("modrinth.index.json não declara nenhum loader conhecido".to_string()))
}

async fn ensure_mrpack_cached(cache_root: &Path, version: &modrinth::ModrinthVersion) -> Result<PathBuf, AppError> {
    let file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| AppError::InvalidInput("versão do modpack sem arquivo".to_string()))?;
    let dest = cache_dir(cache_root, ModSource::Modrinth).join(format!("{}.mrpack", version.id));
    download_file(&file.url, &dest, Some(&file.hashes.sha1)).await?;
    Ok(dest)
}

fn read_mrpack_index(mrpack_path: &Path) -> Result<MrpackIndex, AppError> {
    let bytes = read_zip_entry(mrpack_path, "modrinth.index.json")?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub async fn preview_modrinth(
    cache_root: &Path,
    project_id: &str,
    version_id: Option<&str>,
) -> Result<ModpackPreview, AppError> {
    let all_versions = modrinth::fetch_versions_unfiltered(project_id).await?;
    let versions = all_versions
        .iter()
        .map(|v| ModpackVersionOption { id: v.id.clone(), label: v.version_number.clone() })
        .collect();

    let chosen = match version_id {
        Some(id) => all_versions.iter().find(|v| v.id == id),
        None => all_versions.first(),
    }
    .ok_or_else(|| AppError::NotFound("versão do modpack".to_string()))?;

    let mrpack_path = ensure_mrpack_cached(cache_root, chosen).await?;
    let index = read_mrpack_index(&mrpack_path)?;
    let loader = mrpack_loader(&index.dependencies)?;
    let mc_version = index
        .dependencies
        .get("minecraft")
        .cloned()
        .ok_or_else(|| AppError::InvalidInput("modrinth.index.json sem versão do Minecraft".to_string()))?;

    let download_size_bytes: u64 = index.files.iter().map(|f| f.file_size).sum();

    Ok(ModpackPreview {
        version_id: chosen.id.clone(),
        mc_version,
        loader,
        mod_count: index.files.len(),
        download_size_bytes,
        suggested_ram_mb: suggest_ram_mb(index.files.len()),
        versions,
    })
}

/// Instala um modpack do Modrinth já baixado (mrpack em cache) na
/// pasta da instância: baixa os `files[]` (respeitando `env.client` —
/// arquivos marcados `unsupported` no client não fazem sentido baixar)
/// e extrai `overrides/` por cima.
pub async fn install_modrinth_files(
    cache_root: &Path,
    mrpack_path: &Path,
    instance_dir: &Path,
    mut on_progress: impl FnMut(usize, usize) + Send,
) -> Result<(), AppError> {
    let index = read_mrpack_index(mrpack_path)?;
    let relevant: Vec<&MrpackFile> =
        index.files.iter().filter(|f| f.env.as_ref().map(|e| e.client != "unsupported").unwrap_or(true)).collect();

    let lock_store = ModsLockStore::new(instance_dir);
    let total = relevant.len();
    for (i, file) in relevant.iter().enumerate() {
        let Some(url) = file.downloads.first() else { continue };
        let dest = instance_dir.join(&file.path);
        download_file(url, &dest, Some(&file.hashes.sha1)).await?;

        if let Some((project_id, version_id)) = parse_modrinth_cdn_ids(url) {
            let file_name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            lock_store.upsert(ModLockEntry {
                source: ModSource::Modrinth,
                project_id,
                version_id,
                version_number: String::new(),
                file_name,
                sha1: file.hashes.sha1.clone(),
                installed_at: Utc::now(),
                explicit: true,
                title: None,
                icon_url: None,
                content_type: ContentType::Mod,
            })?;
        }

        on_progress(i + 1, total.max(1));
    }
    let _ = cache_root; // reservado pra eventual cache de arquivo individual no futuro

    extract_prefixed(mrpack_path, "overrides/", instance_dir)?;
    // client-overrides/ (opcional no formato .mrpack) tem prioridade
    // sobre overrides/ quando os dois existem pro mesmo arquivo — extrai
    // por cima, então.
    extract_prefixed(mrpack_path, "client-overrides/", instance_dir)?;

    Ok(())
}

// ---------------------------------------------------------------------
// CurseForge (manifest.json)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct CfManifest {
    minecraft: CfManifestMinecraft,
    overrides: String,
    files: Vec<CfManifestFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct CfManifestMinecraft {
    version: String,
    #[serde(rename = "modLoaders")]
    mod_loaders: Vec<CfManifestLoader>,
}

#[derive(Debug, Clone, Deserialize)]
struct CfManifestLoader {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CfManifestFile {
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
}

/// `modLoaders[].id` vem como `"<loader>-<versão>"` (ex.:
/// `"neoforge-21.1.248"`) — confirmado baixando um pack real
/// (Turbopack Up, 1.21.1).
fn cf_manifest_loader(mod_loaders: &[CfManifestLoader], mc_version: &str) -> Result<LoaderKind, AppError> {
    let id = &mod_loaders.first().ok_or_else(|| AppError::InvalidInput("manifest.json sem modLoaders".to_string()))?.id;
    let (kind, version) = id
        .split_once('-')
        .ok_or_else(|| AppError::InvalidInput(format!("modLoader id inesperado: {id}")))?;
    match kind {
        "neoforge" => Ok(LoaderKind::NeoForge { neoforge_version: version.to_string() }),
        // Mesmo caso do mrpack: o Maven do Forge organiza os
        // artefatos como "<mc_version>-<build>", mas o manifest.json
        // do CurseForge só traz o build ("47.4.13"). Sem combinar, a
        // URL do instalador vem 404.
        "forge" => Ok(LoaderKind::Forge { forge_version: format!("{mc_version}-{version}") }),
        "fabric" => Ok(LoaderKind::Fabric { loader_version: version.to_string() }),
        "quilt" => Ok(LoaderKind::Quilt { loader_version: version.to_string() }),
        other => Err(AppError::InvalidInput(format!("loader do modpack não suportado: {other}"))),
    }
}

async fn ensure_cf_pack_cached(cache_root: &Path, api_key: &str, mod_id: u32, file_id: u32) -> Result<PathBuf, AppError> {
    let files = curseforge::fetch_files_by_ids(api_key, &[file_id]).await?;
    let file = files.into_iter().next().ok_or_else(|| AppError::NotFound("versão do modpack".to_string()))?;
    let download_url = file
        .download_url
        .clone()
        .ok_or_else(|| AppError::InvalidInput("o arquivo do modpack não pode ser baixado automaticamente".to_string()))?;
    let sha1 = file.sha1().map(str::to_string);
    let dest = cache_dir(cache_root, ModSource::Curseforge).join(format!("{mod_id}-{file_id}.zip"));
    download_file(&download_url, &dest, sha1.as_deref()).await?;
    Ok(dest)
}

pub async fn preview_curseforge(
    cache_root: &Path,
    api_key: &str,
    mod_id: u32,
    file_id: Option<u32>,
) -> Result<ModpackPreview, AppError> {
    let files = curseforge::fetch_files(api_key, mod_id, "", "").await;
    // `fetch_files` exige loader/versão pra filtrar — pro seletor de
    // versão do modpack queremos TODAS, então usamos a listagem crua
    // (mesma paginação simples, sem os filtros de compatibilidade que
    // fazem sentido pra mod, não pra modpack inteiro).
    let all_files = match files {
        Ok(f) if !f.is_empty() => f,
        _ => curseforge::fetch_all_files(api_key, mod_id).await?,
    };
    let versions = all_files.iter().map(|f| ModpackVersionOption { id: f.id.to_string(), label: f.display_name.clone() }).collect();

    let chosen_id = file_id.unwrap_or_else(|| all_files.first().map(|f| f.id).unwrap_or_default());
    let pack_path = ensure_cf_pack_cached(cache_root, api_key, mod_id, chosen_id).await?;
    let manifest_bytes = read_zip_entry(&pack_path, "manifest.json")?;
    let manifest: CfManifest = serde_json::from_slice(&manifest_bytes)?;
    let loader = cf_manifest_loader(&manifest.minecraft.mod_loaders, &manifest.minecraft.version)?;

    let file_ids: Vec<u32> = manifest.files.iter().map(|f| f.file_id).collect();
    let resolved = curseforge::fetch_files_by_ids(api_key, &file_ids).await?;
    let download_size_bytes: u64 = resolved.iter().map(|f| f.file_length).sum();

    Ok(ModpackPreview {
        version_id: chosen_id.to_string(),
        mc_version: manifest.minecraft.version,
        loader,
        mod_count: manifest.files.len(),
        download_size_bytes,
        suggested_ram_mb: suggest_ram_mb(manifest.files.len()),
        versions,
    })
}

pub struct CfInstallOutcome {
    pub missing: Vec<crate::core::instances::ManualDownload>,
}

/// Instala um modpack do CurseForge já baixado (zip em cache) na pasta
/// da instância. Mods com `downloadUrl: null` (autor bloqueou download
/// automático) são pulados e devolvidos em `missing` — a instância
/// ainda fica utilizável, só marcada como incompleta pelo chamador.
pub async fn install_curseforge_files(
    api_key: &str,
    pack_path: &Path,
    instance_dir: &Path,
    mut on_progress: impl FnMut(usize, usize) + Send,
) -> Result<CfInstallOutcome, AppError> {
    let manifest_bytes = read_zip_entry(pack_path, "manifest.json")?;
    let manifest: CfManifest = serde_json::from_slice(&manifest_bytes)?;

    let file_ids: Vec<u32> = manifest.files.iter().map(|f| f.file_id).collect();
    let resolved = curseforge::fetch_files_by_ids(api_key, &file_ids).await?;

    let lock_store = ModsLockStore::new(instance_dir);
    let total = resolved.len();
    let mut missing = Vec::new();
    for (i, file) in resolved.iter().enumerate() {
        match &file.download_url {
            Some(url) => {
                let dest = instance_dir.join("mods").join(&file.file_name);
                download_file(url, &dest, file.sha1()).await?;
                lock_store.upsert(ModLockEntry {
                    source: ModSource::Curseforge,
                    project_id: file.mod_id.to_string(),
                    version_id: file.id.to_string(),
                    version_number: file.display_name.clone(),
                    file_name: file.file_name.clone(),
                    sha1: file.sha1().unwrap_or_default().to_string(),
                    installed_at: Utc::now(),
                    explicit: true,
                    title: None,
                    icon_url: None,
                    content_type: ContentType::Mod,
                })?;
            }
            None => missing.push(crate::core::instances::ManualDownload {
                file_name: file.file_name.clone(),
                project_url: format!("https://www.curseforge.com/minecraft/mc-mods/{}", file.mod_id),
            }),
        }
        on_progress(i + 1, total.max(1));
    }

    let overrides_prefix = format!("{}/", manifest.overrides.trim_matches('/'));
    extract_prefixed(pack_path, &overrides_prefix, instance_dir)?;

    Ok(CfInstallOutcome { missing })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_modrinth_cdn_url() {
        // URL real confirmada baixando a versão do Sodium de verdade.
        let url = "https://cdn.modrinth.com/data/AANobbMI/versions/SMxNOGZ6/sodium-fabric-0.8.13%2Bmc1.21.1.jar";
        assert_eq!(parse_modrinth_cdn_ids(url), Some(("AANobbMI".to_string(), "SMxNOGZ6".to_string())));
    }

    #[test]
    fn ignores_non_modrinth_cdn_url() {
        // arquivo hospedado em outro lugar (o formato .mrpack permite) —
        // não dá pra saber o project_id, então não tenta adivinhar.
        assert_eq!(parse_modrinth_cdn_ids("https://github.com/someone/mod/releases/download/v1/mod.jar"), None);
    }

    #[test]
    fn mrpack_loader_reads_neoforge_key() {
        // formato real confirmado no modrinth.index.json do Create+.
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.21.1".to_string());
        deps.insert("neoforge".to_string(), "21.1.233".to_string());
        assert_eq!(mrpack_loader(&deps).unwrap(), LoaderKind::NeoForge { neoforge_version: "21.1.233".to_string() });
    }

    #[test]
    fn mrpack_loader_combines_mc_version_for_forge() {
        // mesmo bug do CurseForge: modrinth.index.json só traz o build
        // do forge ("47.4.13"), tem que combinar com a versão do MC.
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.20.2".to_string());
        deps.insert("forge".to_string(), "47.4.13".to_string());
        assert_eq!(mrpack_loader(&deps).unwrap(), LoaderKind::Forge { forge_version: "1.20.2-47.4.13".to_string() });
    }

    #[test]
    fn mrpack_loader_reads_fabric_key() {
        let mut deps = HashMap::new();
        deps.insert("fabric-loader".to_string(), "0.15.0".to_string());
        assert_eq!(mrpack_loader(&deps).unwrap(), LoaderKind::Fabric { loader_version: "0.15.0".to_string() });
    }

    #[test]
    fn mrpack_loader_rejects_unknown() {
        let deps = HashMap::new();
        assert!(mrpack_loader(&deps).is_err());
    }

    #[test]
    fn cf_manifest_loader_parses_real_shape() {
        // formato real confirmado no manifest.json de "1.21.1 Turbopack Up".
        let loaders = vec![CfManifestLoader { id: "neoforge-21.1.248".to_string() }];
        assert_eq!(
            cf_manifest_loader(&loaders, "1.21.1").unwrap(),
            LoaderKind::NeoForge { neoforge_version: "21.1.248".to_string() }
        );
    }

    #[test]
    fn cf_manifest_loader_combines_mc_version_for_forge() {
        // bug real: manifest.json só traz o build do forge ("47.4.13"),
        // mas o Maven do Forge exige "<mc_version>-<build>" na URL do
        // instalador.
        let loaders = vec![CfManifestLoader { id: "forge-47.4.13".to_string() }];
        assert_eq!(
            cf_manifest_loader(&loaders, "1.20.2").unwrap(),
            LoaderKind::Forge { forge_version: "1.20.2-47.4.13".to_string() }
        );
    }

    #[test]
    fn suggest_ram_respects_bounds() {
        assert_eq!(suggest_ram_mb(0), 2048);
        assert_eq!(suggest_ram_mb(400), 10240);
        assert!(suggest_ram_mb(50) > 2048);
    }
}
