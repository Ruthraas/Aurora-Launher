use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::core::curseforge;
use crate::core::download::download_file;
// re-exportado pra quem já importava `ContentType` daqui (o tipo
// mora em `instances::mods`, faz mais sentido ali).
pub use crate::core::instances::mods::ContentType;
use crate::core::instances::mods::{ModLockEntry, ModSource, ModsLockStore};
use crate::core::instances::{Instance, LoaderKind};
use crate::core::modrinth;
use crate::error::AppError;

// `ContentType` mora em `core::instances::mods` (ver lá) — regra de
// compatibilidade que só importa aqui: só `Mod` depende do loader da
// instância, os outros três só dependem da versão do Minecraft
// (confirmado contra a API real: resourcepack/shader/datapack do
// Modrinth publicam `loaders: ["minecraft"]`, sem amarra de loader
// nenhuma; datapack, por sinal, deveria morar dentro de um mundo
// específico em `saves/<mundo>/datapacks/`, não na instância inteira —
// instalamos na raiz da instância como simplificação consciente,
// documentada no README).

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRef {
    pub source: ModSource,
    pub project_id: String,
    /// Ignorado pelo fluxo de modpack (`commands::modpack`, que
    /// reaproveita esse tipo só por `source`+`project_id`) — por isso
    /// tem um default, em vez de obrigar quem só quer instalar um
    /// modpack a mandar um valor sem sentido pra esse contexto.
    #[serde(default)]
    pub content_type: ContentType,
    /// Título e ícone já vêm do resultado de busca no front (a gente
    /// não tem outra chamada de API que devolva isso de graça na hora
    /// de calcular compatibilidade) — passados de volta só pra virar
    /// exibição bonita em `mods.lock.json`, sem precisar consultar a
    /// API de novo só pra mostrar um ícone na lista de instalados.
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
}

/// Referência resolvida a um arquivo específico — o suficiente pra
/// baixar sem precisar consultar a API de novo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModRef {
    pub source: ModSource,
    pub project_id: String,
    pub content_type: ContentType,
    pub version_id: String,
    pub version_number: String,
    pub file_name: String,
    pub download_url: String,
    pub sha1: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum InstanceCompat {
    Compatible { mod_ref: ModRef, deps_to_install: Vec<ModRef> },
    AlreadyInstalled { installed_version: String },
    Incompatible { reason: String },
}

/// Nome de loader que as APIs de mod entendem — coincide com o `type`
/// do `LoaderKind` pro Modrinth (confirmado: "fabric"/"forge"/
/// "neoforge"/"quilt" são exatamente os valores aceitos no facet
/// `loaders`); o CurseForge usa um enum numérico próprio, traduzido
/// dentro de `curseforge::fetch_files`.
fn loader_name(loader: &LoaderKind) -> Option<&'static str> {
    match loader {
        LoaderKind::Vanilla => None,
        LoaderKind::Fabric { .. } => Some("fabric"),
        LoaderKind::Forge { .. } => Some("forge"),
        LoaderKind::NeoForge { .. } => Some("neoforge"),
        LoaderKind::Quilt { .. } => Some("quilt"),
    }
}

fn loader_label(loader: &LoaderKind) -> &'static str {
    match loader {
        LoaderKind::Vanilla => "Vanilla",
        LoaderKind::Fabric { .. } => "Fabric",
        LoaderKind::Forge { .. } => "Forge",
        LoaderKind::NeoForge { .. } => "NeoForge",
        LoaderKind::Quilt { .. } => "Quilt",
    }
}

/// Calcula o status de compatibilidade de um item (`project`) pra uma
/// instância específica — ver `InstanceCompat`. `cf_api_key` só é
/// necessário quando `project.source == Curseforge`.
pub async fn compute_compatibility(
    instance: &Instance,
    instance_dir: &std::path::Path,
    project: &ProjectRef,
    cf_api_key: Option<&str>,
) -> Result<InstanceCompat, AppError> {
    let lock = ModsLockStore::new(instance_dir).read()?;
    if let Some(installed) = lock.iter().find(|m| m.source == project.source && m.project_id == project.project_id) {
        return Ok(InstanceCompat::AlreadyInstalled { installed_version: installed.version_number.clone() });
    }

    // resourcepack/shader/datapack não têm amarra de loader — rodam em
    // qualquer instância (inclusive vanilla), só precisam bater a
    // versão do Minecraft.
    let loader = if project.content_type.is_loader_locked() {
        match loader_name(&instance.loader) {
            Some(loader) => Some(loader),
            None => return Ok(InstanceCompat::Incompatible { reason: "Vanilla não suporta mods".to_string() }),
        }
    } else {
        None
    };

    let already_installed_ids: std::collections::HashSet<String> = lock.iter().map(|m| m.project_id.clone()).collect();

    match project.source {
        ModSource::Modrinth => {
            // no Modrinth, quase nenhum mod de utilidade/otimização
            // publica uma versão com a tag "quilt" — só "fabric" —
            // mesmo rodando perfeitamente em Quilt via QFAPI (é assim
            // que o próprio site do Modrinth trata isso). Sem esse
            // fallback, toda instância Quilt veria "Sem versão" pra
            // praticamente qualquer mod.
            let modrinth_loader = match loader {
                Some("quilt") => "fabric",
                Some(other) => other,
                // resourcepack/shader/datapack: confirmado que a API
                // publica essas versões com `loaders: ["minecraft"]`.
                None => "minecraft",
            };
            let versions = modrinth::fetch_versions(&project.project_id, modrinth_loader, &instance.mc_version).await?;
            let Some(version) = versions.into_iter().next() else {
                return Ok(InstanceCompat::Incompatible {
                    reason: format!(
                        "Sem versão para {}",
                        if project.content_type.is_loader_locked() {
                            format!("{} + {}", loader_label(&instance.loader), instance.mc_version)
                        } else {
                            instance.mc_version.clone()
                        }
                    ),
                });
            };
            let Some(file) = version.files.iter().find(|f| f.primary).or_else(|| version.files.first()) else {
                return Ok(InstanceCompat::Incompatible { reason: "Versão sem arquivo pra baixar".to_string() });
            };
            let mod_ref = ModRef {
                source: ModSource::Modrinth,
                // ATENÇÃO: `version.project_id` é o id interno base62
                // que a API resolve (ex.: "AANobbMI"), NÃO o slug que a
                // gente pediu (ex.: "sodium") — bug real encontrado:
                // gravar o id resolvido no lock e depois comparar contra
                // o slug (`project.project_id`, usado em toda consulta
                // futura de compatibilidade) nunca batia, então "já
                // instalado" nunca era detectado e o mod voltava a
                // aparecer como pendente pra sempre. Tem que gravar o
                // MESMO valor que será usado pra comparar depois.
                project_id: project.project_id.clone(),
                content_type: project.content_type,
                version_id: version.id.clone(),
                version_number: version.version_number.clone(),
                file_name: file.filename.clone(),
                download_url: file.url.clone(),
                sha1: file.hashes.sha1.clone(),
                title: project.title.clone(),
                icon_url: project.icon_url.clone(),
            };

            // dependências obrigatórias só fazem sentido pra mod — um
            // resourcepack/shader/datapack no Modrinth não declara
            // `dependencies` que a gente deva resolver aqui.
            let mut deps_to_install = Vec::new();
            if project.content_type == ContentType::Mod {
                for dep in &version.dependencies {
                    if dep.dependency_type != modrinth::ModrinthDependencyType::Required {
                        continue;
                    }
                    let Some(dep_project_id) = &dep.project_id else { continue };
                    if already_installed_ids.contains(dep_project_id) {
                        continue;
                    }
                    let dep_versions = modrinth::fetch_versions(dep_project_id, modrinth_loader, &instance.mc_version).await?;
                    let Some(dep_version) = dep_versions.into_iter().next() else { continue };
                    let Some(dep_file) =
                        dep_version.files.iter().find(|f| f.primary).or_else(|| dep_version.files.first())
                    else {
                        continue;
                    };
                    deps_to_install.push(ModRef {
                        source: ModSource::Modrinth,
                        project_id: dep_version.project_id.clone(),
                        content_type: ContentType::Mod,
                        version_id: dep_version.id.clone(),
                        version_number: dep_version.version_number.clone(),
                        file_name: dep_file.filename.clone(),
                        download_url: dep_file.url.clone(),
                        sha1: dep_file.hashes.sha1.clone(),
                        title: None,
                        icon_url: None,
                    });
                }
            }

            Ok(InstanceCompat::Compatible { mod_ref, deps_to_install })
        }
        ModSource::Curseforge => {
            let Some(api_key) = cf_api_key else {
                return Ok(InstanceCompat::Incompatible { reason: "chave da API do CurseForge não configurada".to_string() });
            };
            let mod_id: u32 = project
                .project_id
                .parse()
                .map_err(|_| AppError::InvalidInput(format!("id inválido no CurseForge: {}", project.project_id)))?;

            let files = match loader {
                Some(loader) => curseforge::fetch_files(api_key, mod_id, &instance.mc_version, loader).await?,
                None => curseforge::fetch_files_by_version(api_key, mod_id, &instance.mc_version).await?,
            };
            let Some(file) = files.into_iter().next() else {
                return Ok(InstanceCompat::Incompatible {
                    reason: format!(
                        "Sem versão para {}",
                        if project.content_type.is_loader_locked() {
                            format!("{} + {}", loader_label(&instance.loader), instance.mc_version)
                        } else {
                            instance.mc_version.clone()
                        }
                    ),
                });
            };
            let Some(download_url) = file.download_url.clone() else {
                return Ok(InstanceCompat::Incompatible {
                    reason: "Este arquivo não pode ser baixado automaticamente (bloqueado pelo autor no CurseForge)"
                        .to_string(),
                });
            };
            let Some(sha1) = file.sha1().map(str::to_string) else {
                return Ok(InstanceCompat::Incompatible { reason: "Arquivo sem hash SHA1 publicado".to_string() });
            };
            let mod_ref = ModRef {
                source: ModSource::Curseforge,
                project_id: file.mod_id.to_string(),
                content_type: project.content_type,
                version_id: file.id.to_string(),
                version_number: file.display_name.clone(),
                file_name: file.file_name.clone(),
                download_url,
                sha1,
                title: project.title.clone(),
                icon_url: project.icon_url.clone(),
            };

            let mut deps_to_install = Vec::new();
            if project.content_type == ContentType::Mod {
                for dep in &file.dependencies {
                    if dep.relation_type != curseforge::RELATION_TYPE_REQUIRED {
                        continue;
                    }
                    let dep_project_id = dep.mod_id.to_string();
                    if already_installed_ids.contains(&dep_project_id) {
                        continue;
                    }
                    let dep_files = curseforge::fetch_files(api_key, dep.mod_id, &instance.mc_version, loader.unwrap_or("")).await?;
                    let Some(dep_file) = dep_files.into_iter().next() else { continue };
                    let (Some(dep_url), Some(dep_sha1)) =
                        (dep_file.download_url.clone(), dep_file.sha1().map(str::to_string))
                    else {
                        continue;
                    };
                    deps_to_install.push(ModRef {
                        source: ModSource::Curseforge,
                        project_id: dep_file.mod_id.to_string(),
                        content_type: ContentType::Mod,
                        version_id: dep_file.id.to_string(),
                        version_number: dep_file.display_name.clone(),
                        file_name: dep_file.file_name.clone(),
                        download_url: dep_url,
                        sha1: dep_sha1,
                        title: None,
                        icon_url: None,
                    });
                }
            }

            Ok(InstanceCompat::Compatible { mod_ref, deps_to_install })
        }
    }
}

/// Baixa `mod_ref` pra dentro da instância (pasta certa pro tipo de
/// conteúdo — `mods/`, `resourcepacks/`, `shaderpacks/`, `datapacks/`
/// — com verificação de SHA1 e escrita atômica, via `download_file`) e
/// registra em `mods.lock.json`.
pub async fn install_into_instance(instance_dir: &Path, mod_ref: &ModRef, explicit: bool) -> Result<(), AppError> {
    let dest = instance_dir.join(mod_ref.content_type.install_dir()).join(&mod_ref.file_name);
    download_file(&mod_ref.download_url, &dest, Some(&mod_ref.sha1)).await?;

    ModsLockStore::new(instance_dir).upsert(ModLockEntry {
        source: mod_ref.source,
        project_id: mod_ref.project_id.clone(),
        version_id: mod_ref.version_id.clone(),
        version_number: mod_ref.version_number.clone(),
        file_name: mod_ref.file_name.clone(),
        sha1: mod_ref.sha1.clone(),
        installed_at: Utc::now(),
        explicit,
        title: mod_ref.title.clone(),
        icon_url: mod_ref.icon_url.clone(),
        content_type: mod_ref.content_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::instances::{InstanceStatus, JvmFlagPreset};

    fn temp_instance_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-mods-compat-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fake_instance(loader: LoaderKind) -> Instance {
        Instance {
            schema_version: 1,
            id: uuid::Uuid::new_v4(),
            name: "Teste".to_string(),
            mc_version: "1.21.1".to_string(),
            loader,
            ram_min_mb: 1024,
            ram_max_mb: 2048,
            jvm_flag_preset: JvmFlagPreset::None,
            custom_jvm_args: String::new(),
            status: InstanceStatus::Ready,
            error_message: None,
            modpack_origin: None,
            missing_manual_downloads: Vec::new(),
            created_at: Utc::now(),
            last_played: None,
        }
    }

    fn fake_project(project_id: &str) -> ProjectRef {
        ProjectRef {
            source: ModSource::Modrinth,
            project_id: project_id.to_string(),
            content_type: ContentType::Mod,
            title: None,
            icon_url: None,
        }
    }

    // Regressão do bug real desta sessão: o id gravado no lock (pela
    // instalação) tem que ser o MESMO usado aqui pra comparar, senão
    // "já instalado" nunca bate. Esse teste roda sem rede — o caminho
    // "já instalado" retorna ANTES de qualquer chamada de API, então
    // dá pra testar de verdade sem mockar Modrinth/CurseForge.
    #[tokio::test]
    async fn already_installed_short_circuits_before_any_network_call() {
        let dir = temp_instance_dir();
        ModsLockStore::new(&dir)
            .upsert(ModLockEntry {
                source: ModSource::Modrinth,
                project_id: "AANobbMI".to_string(),
                version_id: "v1".to_string(),
                version_number: "1.0.0".to_string(),
                file_name: "sodium.jar".to_string(),
                sha1: "abc".to_string(),
                installed_at: Utc::now(),
                explicit: true,
                title: Some("Sodium".to_string()),
                icon_url: None,
                content_type: ContentType::Mod,
            })
            .unwrap();

        let instance = fake_instance(LoaderKind::Fabric { loader_version: "0.15.0".to_string() });
        let project = fake_project("AANobbMI");

        let compat = compute_compatibility(&instance, &dir, &project, None).await.unwrap();
        assert!(matches!(compat, InstanceCompat::AlreadyInstalled { installed_version } if installed_version == "1.0.0"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn vanilla_instance_rejects_loader_locked_content() {
        let dir = temp_instance_dir();
        let instance = fake_instance(LoaderKind::Vanilla);
        let project = fake_project("some-mod");

        let compat = compute_compatibility(&instance, &dir, &project, None).await.unwrap();
        assert!(matches!(compat, InstanceCompat::Incompatible { reason } if reason.contains("Vanilla")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn loader_name_maps_each_variant() {
        assert_eq!(loader_name(&LoaderKind::Vanilla), None);
        assert_eq!(loader_name(&LoaderKind::Fabric { loader_version: "0.15.0".to_string() }), Some("fabric"));
        assert_eq!(loader_name(&LoaderKind::Forge { forge_version: "1.20.2-47.2.0".to_string() }), Some("forge"));
        assert_eq!(loader_name(&LoaderKind::NeoForge { neoforge_version: "21.1.0".to_string() }), Some("neoforge"));
        assert_eq!(loader_name(&LoaderKind::Quilt { loader_version: "0.20.0".to_string() }), Some("quilt"));
    }
}

