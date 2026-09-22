use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

use super::install::SharedCache;
use super::{Instance, LoaderKind};
use crate::core::accounts::Account;
use crate::core::loaders::{fabric, forge, neoforge, quilt};
use crate::core::minecraft::arguments::{resolve_argument_entries, substitute_placeholders, LaunchVariables};
use crate::core::minecraft::launch::{build_classpath, build_launch_argv};
use crate::core::minecraft::library::resolve_libraries;
use crate::core::minecraft::rules::TargetOs;
use crate::core::minecraft::version::VersionDetails;
use crate::error::AppError;

/// Lança o jogo de verdade: monta o classpath, os argumentos (com
/// placeholders substituídos) e chama `java`/`javaw` como processo
/// solto (não esperamos ele terminar — a UI não pode travar enquanto
/// o Minecraft roda). Pra Fabric/NeoForge, busca o profile do loader e
/// funde nas listas antes de montar o comando — sem gerar um
/// `VersionDetails` artificial "fundido".
pub async fn launch(
    instance: &Instance,
    version: &VersionDetails,
    account: &Account,
    java_path: &Path,
    instance_dir: &Path,
    cache: &SharedCache<'_>,
) -> Result<(), AppError> {
    let os = TargetOs::current();
    let resolved = resolve_libraries(&version.libraries, os);

    let mut classpath_entries: Vec<String> = resolved
        .classpath
        .iter()
        .map(|artifact| cache.libraries_dir().join(&artifact.path).to_string_lossy().into_owned())
        .collect();

    let mut main_class = version.main_class.clone();
    let mut extra_game_args: Vec<String> = Vec::new();
    let mut extra_jvm_args: Vec<String> = Vec::new();

    match &instance.loader {
        LoaderKind::Vanilla => {
            classpath_entries.push(cache.client_jar_path(&version.id).to_string_lossy().into_owned());
        }
        LoaderKind::Fabric { loader_version } => {
            // client.jar vanilla continua no classpath — o Fabric só
            // acrescenta bibliotecas por cima, não substitui o jar base.
            classpath_entries.push(cache.client_jar_path(&version.id).to_string_lossy().into_owned());

            let profile = fabric::fetch_profile(&instance.mc_version, loader_version).await?;
            for library in &profile.libraries {
                if let Some(path) = library.maven_path() {
                    classpath_entries.push(cache.libraries_dir().join(path).to_string_lossy().into_owned());
                }
            }
            main_class = profile.main_class.clone();
            extra_game_args = resolve_argument_entries(&profile.arguments.game, os);
            extra_jvm_args = resolve_argument_entries(&profile.arguments.jvm, os);
        }
        LoaderKind::Quilt { loader_version } => {
            // mesmo esquema do Fabric — o Quilt também só acrescenta
            // bibliotecas por cima do client.jar vanilla.
            classpath_entries.push(cache.client_jar_path(&version.id).to_string_lossy().into_owned());

            let profile = quilt::fetch_profile(&instance.mc_version, loader_version).await?;
            for library in &profile.libraries {
                if let Some(path) = library.maven_path() {
                    classpath_entries.push(cache.libraries_dir().join(path).to_string_lossy().into_owned());
                }
            }
            main_class = profile.main_class.clone();
            extra_game_args = resolve_argument_entries(&profile.arguments.game, os);
            extra_jvm_args = resolve_argument_entries(&profile.arguments.jvm, os);
        }
        LoaderKind::NeoForge { neoforge_version } => {
            // aqui o jar "patchado" (client.jar + NeoForge já mesclados
            // pelos processors na instalação) ENTRA no lugar do
            // client.jar vanilla puro — colocar os dois juntos duplicaria
            // toda classe do jogo no classloader.
            let patched = neoforge::install::resolve_patched_client_path(cache.root, neoforge_version)?;
            classpath_entries.push(patched.to_string_lossy().into_owned());

            // o jar patchado só tem o Minecraft com os hooks do
            // NeoForge — as classes do mod loader em si (NeoForgeMod,
            // ClientNeoForgeMod, ...) vêm do jar "universal" separado.
            let universal = neoforge::install::resolve_universal_jar_path(cache.root, neoforge_version)?;
            classpath_entries.push(universal.to_string_lossy().into_owned());

            let profile = neoforge::install::read_cached_version_profile(cache.root, neoforge_version)?;
            for library in &profile.libraries {
                if let Some(artifact) = library.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
                    classpath_entries.push(cache.libraries_dir().join(&artifact.path).to_string_lossy().into_owned());
                }
            }
            main_class = profile.main_class.clone();
            extra_game_args = resolve_argument_entries(&profile.arguments.game, os);
            extra_jvm_args = resolve_argument_entries(&profile.arguments.jvm, os);
        }
        LoaderKind::Forge { forge_version } => {
            // mesmo esquema do NeoForge (patchado no lugar do
            // client.jar vanilla + universal com as classes do mod
            // loader), mas o Forge clássico precisa de um TERCEIRO jar
            // no classpath: o "client-extra" (recursos do client.jar
            // reempacotados pelo processor MC_EXTRA) — sem ele o
            // BootstrapLauncher/ModLauncher não acha os assets
            // embutidos no jar. Confirmado comparando install_profile
            // reais do Forge (que tem MC_EXTRA) contra o NeoForge (que
            // não tem).
            let patched = forge::install::resolve_patched_client_path(cache.root, forge_version)?;
            classpath_entries.push(patched.to_string_lossy().into_owned());

            let universal = forge::install::resolve_universal_jar_path(cache.root, forge_version)?;
            classpath_entries.push(universal.to_string_lossy().into_owned());

            let client_extra = forge::install::resolve_client_extra_jar_path(cache.root, forge_version)?;
            classpath_entries.push(client_extra.to_string_lossy().into_owned());

            let profile = forge::install::read_cached_version_profile(cache.root, forge_version)?;
            for library in &profile.libraries {
                if let Some(artifact) = library.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
                    classpath_entries.push(cache.libraries_dir().join(&artifact.path).to_string_lossy().into_owned());
                }
            }
            main_class = profile.main_class.clone();
            extra_game_args = resolve_argument_entries(&profile.arguments.game, os);
            extra_jvm_args = resolve_argument_entries(&profile.arguments.jvm, os);
        }
    }

    let natives_dir = cache.natives_dir(&version.id);
    let assets_dir = cache.assets_dir();

    let Account::Offline { uuid, username, .. } = account;
    let (username, uuid, access_token, user_type) =
        (username.clone(), uuid.simple().to_string(), "0".to_string(), "legacy".to_string());

    let vars = LaunchVariables {
        auth_player_name: username,
        auth_uuid: uuid,
        auth_access_token: access_token,
        user_type,
        version_name: version.id.clone(),
        version_type: version.version_type.clone(),
        game_directory: instance_dir.to_string_lossy().into_owned(),
        assets_root: assets_dir.to_string_lossy().into_owned(),
        assets_index_name: version.assets.clone(),
        natives_directory: natives_dir.to_string_lossy().into_owned(),
        classpath: build_classpath(&classpath_entries, os),
        launcher_name: "aurora-launcher".to_string(),
        launcher_version: env!("CARGO_PKG_VERSION").to_string(),
        library_directory: cache.libraries_dir().to_string_lossy().into_owned(),
    };

    let mut game_args = substitute_placeholders(&version.resolve_game_arguments(os), &vars);
    let mut jvm_args = substitute_placeholders(&version.resolve_jvm_arguments(os), &vars);

    game_args.extend(substitute_placeholders(&extra_game_args, &vars));
    jvm_args.extend(substitute_placeholders(&extra_jvm_args, &vars));

    // Versões legadas não trazem `arguments.jvm` no JSON — o launcher
    // oficial sempre montou essas flags na mão nesse caso, então
    // fazemos o mesmo.
    if version.is_legacy() {
        jvm_args.push(format!("-Djava.library.path={}", vars.natives_directory));
        jvm_args.push("-cp".to_string());
        jvm_args.push(vars.classpath.clone());
    }

    for (i, flag) in crate::core::jvm_flags::flags(instance.jvm_flag_preset).into_iter().enumerate() {
        jvm_args.insert(i, flag);
    }
    jvm_args.insert(0, format!("-Xms{}M", instance.ram_min_mb));
    jvm_args.insert(1, format!("-Xmx{}M", instance.ram_max_mb));

    let argv = build_launch_argv(&jvm_args, &main_class, &game_args);

    tokio::fs::create_dir_all(instance_dir).await?;

    // O jogo só cria `logs/latest.log` (via log4j) depois que a JVM já
    // subiu e o Minecraft inicializou — um crash ANTES disso (jar
    // faltando, classpath errado, erro de módulo do ModLauncher/
    // BootstrapLauncher) não deixa rastro nenhum se a saída for pro
    // vazio. Por isso a stdout/stderr do processo vai pra um arquivo
    // nosso, sempre, independente do jogo ter chegado a logar algo.
    let logs_dir = instance_dir.join("logs");
    tokio::fs::create_dir_all(&logs_dir).await?;
    let stdout_file = std::fs::File::create(logs_dir.join("launcher-stdout.log"))?;
    let stderr_file = std::fs::File::create(logs_dir.join("launcher-stderr.log"))?;

    Command::new(java_path)
        .args(&argv)
        .current_dir(instance_dir)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()?;

    Ok(())
}
