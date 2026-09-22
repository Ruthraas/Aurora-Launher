use crate::core::minecraft::manifest::{self, VersionManifest};
use crate::core::minecraft::version::VersionDetails;
use crate::error::AppResult;

/// Lista de versões oficiais do Minecraft, direto do `piston-meta` da
/// Mojang. Ainda sem cache — a Fase 4 decide onde guardar isso
/// localmente pra não bater na rede toda vez que a tela de criar
/// instância abrir.
#[tauri::command]
pub async fn fetch_version_manifest() -> AppResult<VersionManifest> {
    Ok(manifest::fetch_version_manifest().await?)
}

/// JSON completo de uma versão específica (o `url` vem de um item de
/// `fetch_version_manifest`). Ainda não baixa nenhum arquivo — só
/// resolve o que precisaria ser baixado.
#[tauri::command]
pub async fn fetch_version_details(url: String) -> AppResult<VersionDetails> {
    Ok(manifest::fetch_version_details(&url).await?)
}
