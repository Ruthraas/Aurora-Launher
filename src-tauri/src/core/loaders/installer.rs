use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::core::download::{extract_zip_entry_to_file, read_zip_entry};
use crate::core::loaders::maven::maven_coordinate_to_path;
use crate::core::minecraft::arguments::ArgumentEntry;
use crate::core::minecraft::library::Library;
use crate::error::AppError;

/// Pipeline de instalador compartilhado por NeoForge e Forge clássico
/// (Forge é o fork original, NeoForge um fork dele — o formato do
/// `install_profile.json`/`processors[]`/`version.json` é o mesmo nos
/// dois, só mudam os nomes de artefatos e o `mainClass`).
///
/// `install_profile.json` de dentro do instalador — descreve o que
/// baixar (`libraries`) e como transformar o client.jar vanilla no jar
/// "patchado" do loader (`processors` + `data`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProfile {
    pub version: String,
    pub json: String,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub processors: Vec<ProcessorEntry>,
    #[serde(default)]
    pub data: HashMap<String, DataEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorEntry {
    #[serde(default)]
    pub sides: Option<Vec<String>>,
    pub jar: String,
    #[serde(default)]
    pub classpath: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

impl ProcessorEntry {
    pub fn runs_for_client(&self) -> bool {
        self.sides.as_ref().map(|sides| sides.iter().any(|side| side == "client")).unwrap_or(true)
    }
}

/// O Forge clássico traz `client` E `server` em cada entrada de `data`
/// (o NeoForge só publica `client`) — só usamos o lado client em
/// qualquer um dos dois, o `server` extra é ignorado pelo serde.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEntry {
    pub client: String,
}

/// O `/version.json` de dentro do instalador: mesmo espírito do
/// profile do Fabric (`inheritsFrom` a versão vanilla base + libs e
/// argumentos extras), mas sem `downloads`/`assetIndex`/`javaVersion`
/// — isso continua vindo inteiramente da versão vanilla herdada.
/// Compartilhado por Forge e NeoForge; o `mainClass` muda de versão
/// pra versão (ModLauncher/BootstrapLauncher/ForgeBootstrap no Forge,
/// FML no NeoForge) mas isso já vem pronto do JSON, sem precisar de
/// código específico por era.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersionProfile {
    pub id: String,
    pub inherits_from: String,
    pub main_class: String,
    #[serde(default)]
    pub arguments: LoaderArguments,
    #[serde(default)]
    pub libraries: Vec<Library>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoaderArguments {
    #[serde(default)]
    pub game: Vec<ArgumentEntry>,
    #[serde(default)]
    pub jvm: Vec<ArgumentEntry>,
}

/// Monta o "datamap" que alimenta a substituição de `{PLACEHOLDER}`
/// nos argumentos dos processors: entradas fixas (SIDE, ROOT, ...) +
/// as entradas de `install_profile.json`'s `data`, já resolvidas.
pub fn build_datamap(
    profile: &InstallProfile,
    root: &Path,
    installer_path: &Path,
    libraries_dir: &Path,
    mc_version: &str,
    client_jar_path: &Path,
) -> Result<HashMap<String, String>, AppError> {
    let mut map = HashMap::new();
    map.insert("SIDE".to_string(), "client".to_string());
    map.insert("MINECRAFT_JAR".to_string(), client_jar_path.to_string_lossy().into_owned());
    map.insert("MINECRAFT_VERSION".to_string(), mc_version.to_string());
    map.insert("ROOT".to_string(), root.to_string_lossy().into_owned());
    map.insert("INSTALLER".to_string(), installer_path.to_string_lossy().into_owned());
    map.insert("LIBRARY_DIR".to_string(), libraries_dir.to_string_lossy().into_owned());

    for (key, entry) in &profile.data {
        let resolved = resolve_data_value(&entry.client, installer_path, libraries_dir, root)?;
        map.insert(key.clone(), resolved);
    }

    Ok(map)
}

/// Cada valor de `data` é de um destes formatos: `[coordenada:maven]`
/// (vira caminho local em `LIBRARY_DIR`), `'literal'` (as aspas somem)
/// ou `/caminho/dentro/do/jar` (extraído do instalador pra `ROOT/data`).
fn resolve_data_value(raw: &str, installer_path: &Path, libraries_dir: &Path, root: &Path) -> Result<String, AppError> {
    if let Some(coordinate) = raw.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
        let path = maven_coordinate_to_path(coordinate)
            .ok_or_else(|| AppError::InvalidInput(format!("coordenada maven inválida no instalador: {coordinate}")))?;
        return Ok(libraries_dir.join(path).to_string_lossy().into_owned());
    }
    if let Some(literal) = raw.strip_prefix('\'').and_then(|rest| rest.strip_suffix('\'')) {
        return Ok(literal.to_string());
    }
    if let Some(jar_entry) = raw.strip_prefix('/') {
        let dest = root.join("data").join(jar_entry);
        extract_zip_entry_to_file(installer_path, jar_entry, &dest)?;
        return Ok(dest.to_string_lossy().into_owned());
    }
    Ok(raw.to_string())
}

/// Resolve um argumento de processor por completo. Além de `{CHAVE}`
/// vindo do datamap, o instalador às vezes bota uma coordenada/literal
/// CRUA direto no argumento (ex.: `--input [de.oceanlabs.mcp:mcp_config:<v>@zip]`),
/// no mesmíssimo formato das entradas de `data` — sem nenhum `{}`
/// envolvendo. Confirmado num install_profile real (Forge 1.20.2): o
/// processor MCP_DATA recebe assim, e sem resolver isso a ferramenta
/// recebe o argumento com colchete e tudo, e quebra com "Input does
/// not exist: [...]". Então primeiro tentamos resolver o argumento
/// INTEIRO como um valor de `data` (`[...]`/`'...'`/`/...`); só se não
/// bater com nenhum desses formatos é que caímos na substituição de
/// `{CHAVE}`.
fn resolve_processor_arg(
    arg: &str,
    datamap: &HashMap<String, String>,
    installer_path: &Path,
    libraries_dir: &Path,
    root: &Path,
) -> Result<String, AppError> {
    let looks_like_raw_data_value =
        (arg.starts_with('[') && arg.ends_with(']')) || (arg.starts_with('\'') && arg.ends_with('\''));
    if looks_like_raw_data_value {
        return resolve_data_value(arg, installer_path, libraries_dir, root);
    }
    Ok(substitute_processor_arg(arg, datamap))
}

/// Substitui todo `{chave}` conhecido no datamap — sintaxe diferente
/// da `${...}` do Mojang, é assim que o instalador do NeoForge/Forge
/// escreve os placeholders dos processors.
fn substitute_processor_arg(arg: &str, datamap: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(arg.len());
    let mut rest = arg;
    while let Some(start) = rest.find('{') {
        let Some(end_rel) = rest[start..].find('}') else {
            result.push_str(rest);
            return result;
        };
        let end = start + end_rel;
        result.push_str(&rest[..start]);
        let key = &rest[start + 1..end];
        match datamap.get(key) {
            Some(value) => result.push_str(value),
            None => result.push_str(&rest[start..=end]),
        }
        rest = &rest[end + 1..];
    }
    result.push_str(rest);
    result
}

fn read_main_class_from_manifest(jar_path: &Path) -> Result<String, AppError> {
    let bytes = read_zip_entry(jar_path, "META-INF/MANIFEST.MF")?;
    let text = String::from_utf8_lossy(&bytes);
    text.lines()
        .find_map(|line| line.strip_prefix("Main-Class:"))
        .map(|value| value.trim().to_string())
        .ok_or_else(|| AppError::InvalidInput(format!("{} não declara Main-Class", jar_path.display())))
}

/// Roda um processor de verdade: monta o classpath, lê a main class do
/// manifesto do jar (processors são "fatjars" autocontidos), substitui
/// os placeholders dos argumentos e espera o processo terminar — os
/// processors seguintes dependem da saída dos anteriores, então isso
/// TEM que ser sequencial e bloqueante (diferente do lançamento do
/// jogo, que é solto).
pub async fn run_processor(
    processor: &ProcessorEntry,
    datamap: &HashMap<String, String>,
    installer_path: &Path,
    libraries_dir: &Path,
    root: &Path,
    java_path: &Path,
) -> Result<(), AppError> {
    let jar_path = maven_coordinate_to_path(&processor.jar)
        .map(|path| libraries_dir.join(path))
        .ok_or_else(|| AppError::InvalidInput(format!("jar de processor inválido: {}", processor.jar)))?;

    let mut classpath_entries: Vec<String> = processor
        .classpath
        .iter()
        .filter_map(|coordinate| maven_coordinate_to_path(coordinate))
        .map(|path| libraries_dir.join(path).to_string_lossy().into_owned())
        .collect();
    classpath_entries.push(jar_path.to_string_lossy().into_owned());
    let classpath_separator = if cfg!(windows) { ';' } else { ':' };
    let classpath = classpath_entries.join(&classpath_separator.to_string());

    let main_class = read_main_class_from_manifest(&jar_path)?;
    let args: Vec<String> = processor
        .args
        .iter()
        .map(|arg| resolve_processor_arg(arg, datamap, installer_path, libraries_dir, root))
        .collect::<Result<_, _>>()?;

    let output = Command::new(java_path)
        .arg("-cp")
        .arg(&classpath)
        .arg(&main_class)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: Vec<&str> = stderr.lines().rev().take(5).collect();
        return Err(AppError::InvalidInput(format!(
            "processor \"{}\" falhou (código {:?}): {}",
            processor.jar,
            output.status.code(),
            tail.into_iter().rev().collect::<Vec<_>>().join(" / ")
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processor_without_sides_runs_for_client() {
        let entry = ProcessorEntry { sides: None, jar: String::new(), classpath: vec![], args: vec![] };
        assert!(entry.runs_for_client());
    }

    #[test]
    fn processor_with_server_only_sides_skips_client() {
        let entry =
            ProcessorEntry { sides: Some(vec!["server".to_string()]), jar: String::new(), classpath: vec![], args: vec![] };
        assert!(!entry.runs_for_client());
    }

    #[test]
    fn install_profile_parses_real_shape() {
        // formato real confirmado baixando um instalador de verdade
        // (NeoForge 21.11.45, MC 1.21.11).
        let json = r#"{
            "version": "neoforge-21.11.45",
            "json": "/version.json",
            "libraries": [],
            "processors": [
                {
                    "sides": ["server"],
                    "jar": "net.neoforged.installertools:installertools:4.0.6:fatjar",
                    "classpath": ["net.neoforged.installertools:installertools:4.0.6:fatjar"],
                    "args": ["--task", "EXTRACT_FILES"]
                },
                {
                    "jar": "net.neoforged.installertools:installertools:4.0.6:fatjar",
                    "classpath": ["net.neoforged.installertools:installertools:4.0.6:fatjar"],
                    "args": ["--task", "DOWNLOAD_MOJMAPS", "--output", "{MOJMAPS}"]
                }
            ],
            "data": {
                "MOJMAPS": {"client": "[net.minecraft:client:1.21.11:mappings@txt]"},
                "BINPATCH": {"client": "/data/client.lzma"},
                "MCP_VERSION": {"client": "'1.21.11-20251209.172050'"}
            }
        }"#;
        let profile: InstallProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.processors.len(), 2);
        assert!(!profile.processors[0].runs_for_client());
        assert!(profile.processors[1].runs_for_client());
        assert_eq!(profile.data.get("MOJMAPS").unwrap().client, "[net.minecraft:client:1.21.11:mappings@txt]");
    }

    #[test]
    fn install_profile_parses_forge_shape_with_extra_server_field() {
        // o Forge clássico publica `client` E `server` em cada entrada
        // de `data` (o NeoForge só publica `client`) — confirmado
        // baixando forge-1.20.1-47.2.0-installer.jar de verdade.
        let json = r#"{
            "version": "1.20.1-forge-47.2.0",
            "json": "/version.json",
            "libraries": [],
            "processors": [],
            "data": {
                "MOJMAPS": {
                    "client": "[net.minecraft:client:1.20.1-20230612.114412:mappings@txt]",
                    "server": "[net.minecraft:server:1.20.1-20230612.114412:mappings@txt]"
                }
            }
        }"#;
        let profile: InstallProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.data.get("MOJMAPS").unwrap().client, "[net.minecraft:client:1.20.1-20230612.114412:mappings@txt]");
    }

    #[test]
    fn resolve_processor_arg_resolves_raw_bracket_coordinate() {
        // bug real confirmado num install_profile do Forge (1.20.2): o
        // processor MCP_DATA recebe `--input [coordenada@zip]` CRU, sem
        // `{}` — não é um `{CHAVE}` do datamap, é o mesmíssimo formato
        // das entradas de `data`. Sem resolver isso, a ferramenta
        // recebia o argumento com colchete e tudo e quebrava com
        // "Input does not exist: [...]".
        let dir = std::env::temp_dir();
        let datamap = HashMap::new();
        let resolved =
            resolve_processor_arg("[de.oceanlabs.mcp:mcp_config:1.20.2-20230921.100330@zip]", &datamap, &dir, &dir, &dir)
                .unwrap();
        let expected = dir.join("de/oceanlabs/mcp/mcp_config/1.20.2-20230921.100330/mcp_config-1.20.2-20230921.100330.zip");
        assert_eq!(resolved, expected.to_string_lossy());
    }

    #[test]
    fn resolve_processor_arg_falls_back_to_placeholder_substitution() {
        let dir = std::env::temp_dir();
        let mut datamap = HashMap::new();
        datamap.insert("ROOT".to_string(), "C:/aurora".to_string());
        let resolved = resolve_processor_arg("{ROOT}/run.sh", &datamap, &dir, &dir, &dir).unwrap();
        assert_eq!(resolved, "C:/aurora/run.sh");
    }

    #[test]
    fn resolve_processor_arg_leaves_plain_flag_untouched() {
        let dir = std::env::temp_dir();
        let datamap = HashMap::new();
        let resolved = resolve_processor_arg("--task", &datamap, &dir, &dir, &dir).unwrap();
        assert_eq!(resolved, "--task");
    }

    #[test]
    fn substitutes_known_placeholder() {
        let mut datamap = HashMap::new();
        datamap.insert("ROOT".to_string(), "C:/aurora/neoforge".to_string());
        assert_eq!(substitute_processor_arg("{ROOT}/run.sh", &datamap), "C:/aurora/neoforge/run.sh");
    }

    #[test]
    fn leaves_unknown_placeholder_untouched() {
        let datamap = HashMap::new();
        assert_eq!(substitute_processor_arg("{UNKNOWN}", &datamap), "{UNKNOWN}");
    }

    #[test]
    fn passes_through_plain_argument() {
        let datamap = HashMap::new();
        assert_eq!(substitute_processor_arg("--task", &datamap), "--task");
    }

    #[test]
    fn resolve_data_value_strips_quotes_from_literal() {
        let dir = std::env::temp_dir();
        let value = resolve_data_value("'1.21.11-20251209.172050'", &dir, &dir, &dir).unwrap();
        assert_eq!(value, "1.21.11-20251209.172050");
    }

    #[test]
    fn resolve_data_value_resolves_maven_ref_under_library_dir() {
        let dir = std::env::temp_dir();
        let value = resolve_data_value("[net.minecraft:client:1.21.11:mappings@txt]", &dir, &dir, &dir).unwrap();
        let expected = dir.join("net/minecraft/client/1.21.11/client-1.21.11-mappings.txt");
        assert_eq!(value, expected.to_string_lossy());
    }
}
