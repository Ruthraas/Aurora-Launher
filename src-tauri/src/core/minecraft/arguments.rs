use serde::{Deserialize, Serialize};

use super::rules::{rules_allow, Rule, TargetOs};

/// Um item de `arguments.game`/`arguments.jvm` (formato moderno,
/// 1.13+): ou é uma string solta, ou é condicional a `rules`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentEntry {
    Plain(String),
    Conditional {
        #[serde(default)]
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Single(String),
    Multiple(Vec<String>),
}

/// Resolve os itens condicionais de uma lista `arguments.game` ou
/// `arguments.jvm` pro SO atual, achatando em uma lista simples de
/// tokens (ainda com os placeholders `${...}` sem substituir).
pub fn resolve_argument_entries(entries: &[ArgumentEntry], os: TargetOs) -> Vec<String> {
    let mut out = Vec::new();
    for entry in entries {
        match entry {
            ArgumentEntry::Plain(value) => out.push(value.clone()),
            ArgumentEntry::Conditional { rules, value } => {
                if rules_allow(rules, os) {
                    match value {
                        ArgumentValue::Single(value) => out.push(value.clone()),
                        ArgumentValue::Multiple(values) => out.extend(values.iter().cloned()),
                    }
                }
            }
        }
    }
    out
}

/// `minecraftArguments` do formato legado (pré-1.13): uma única string
/// com os tokens separados por espaço.
pub fn split_legacy_arguments(minecraft_arguments: &str) -> Vec<String> {
    minecraft_arguments.split_whitespace().map(str::to_string).collect()
}

/// Valores conhecidos pros placeholders `${...}` de `arguments`/
/// `minecraftArguments`. Alguns (auth_xuid, clientid) só fazem sentido
/// com conta online — Aurora Launcher é offline-only por decisão do
/// produto, então ficam sempre vazios, sem quebrar a substituição dos
/// demais.
#[derive(Debug, Clone, Default)]
pub struct LaunchVariables {
    pub auth_player_name: String,
    pub auth_uuid: String,
    pub auth_access_token: String,
    pub user_type: String,
    pub version_name: String,
    pub version_type: String,
    pub game_directory: String,
    pub assets_root: String,
    pub assets_index_name: String,
    pub natives_directory: String,
    pub classpath: String,
    pub launcher_name: String,
    pub launcher_version: String,
    /// Confirmado num `version.json` real do NeoForge
    /// (`-DlibraryDirectory=${library_directory}`) — não existe no
    /// formato vanilla puro.
    pub library_directory: String,
}

impl LaunchVariables {
    fn lookup(&self, key: &str) -> Option<&str> {
        match key {
            "auth_player_name" => Some(&self.auth_player_name),
            "auth_uuid" => Some(&self.auth_uuid),
            "auth_access_token" | "auth_session" => Some(&self.auth_access_token),
            "user_type" => Some(&self.user_type),
            "version_name" => Some(&self.version_name),
            "version_type" => Some(&self.version_type),
            "game_directory" => Some(&self.game_directory),
            "assets_root" | "game_assets" => Some(&self.assets_root),
            "assets_index_name" => Some(&self.assets_index_name),
            "natives_directory" => Some(&self.natives_directory),
            "classpath" => Some(&self.classpath),
            "launcher_name" => Some(&self.launcher_name),
            "launcher_version" => Some(&self.launcher_version),
            "library_directory" => Some(&self.library_directory),
            // confirmado num jvm arg real do Forge moderno (o `-p`
            // module path do BootstrapLauncher, ex.:
            // "-p" "${library_directory}/a.jar${classpath_separator}${library_directory}/b.jar")
            // — sem resolver isso, a JVM recebe o literal
            // "${classpath_separator}" no meio do path e recusa com
            // `InvalidPathException: Illegal char <:>`.
            "classpath_separator" => Some(if cfg!(windows) { ";" } else { ":" }),
            "auth_xuid" | "clientid" | "user_properties" => Some(""),
            _ => None,
        }
    }
}

/// Substitui todo `${placeholder}` conhecido em cada token. Um
/// placeholder desconhecido é deixado como está (mais seguro que
/// remover silenciosamente um argumento que talvez importe).
pub fn substitute_placeholders(tokens: &[String], vars: &LaunchVariables) -> Vec<String> {
    tokens.iter().map(|token| substitute_one(token, vars)).collect()
}

fn substitute_one(token: &str, vars: &LaunchVariables) -> String {
    let mut result = String::with_capacity(token.len());
    let mut rest = token;
    while let Some(start) = rest.find("${") {
        let Some(end) = rest[start..].find('}') else {
            result.push_str(rest);
            return result;
        };
        result.push_str(&rest[..start]);
        let key = &rest[start + 2..start + end];
        match vars.lookup(key) {
            Some(value) => result.push_str(value),
            None => result.push_str(&rest[start..start + end + 1]),
        }
        rest = &rest[start + end + 1..];
    }
    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vars() -> LaunchVariables {
        LaunchVariables {
            auth_player_name: "BDarkBr".into(),
            auth_uuid: "b50ad385829d3141a2167e7d7539ba7f".into(),
            auth_access_token: "0".into(),
            user_type: "legacy".into(),
            version_name: "1.21.1".into(),
            version_type: "release".into(),
            game_directory: "C:/aurora/instances/meu-survival".into(),
            assets_root: "C:/aurora/assets".into(),
            assets_index_name: "26".into(),
            natives_directory: "C:/aurora/instances/meu-survival/natives".into(),
            classpath: "a.jar;b.jar".into(),
            launcher_name: "aurora-launcher".into(),
            launcher_version: "0.1.0".into(),
            library_directory: "C:/aurora/cache/libraries".into(),
        }
    }

    #[test]
    fn substitutes_known_placeholder() {
        let tokens = vec!["--username".to_string(), "${auth_player_name}".to_string()];
        let result = substitute_placeholders(&tokens, &sample_vars());
        assert_eq!(result, vec!["--username", "BDarkBr"]);
    }

    #[test]
    fn substitutes_classpath_separator_matching_current_os() {
        let tokens = vec!["a.jar${classpath_separator}b.jar".to_string()];
        let result = substitute_placeholders(&tokens, &sample_vars());
        let expected_separator = if cfg!(windows) { ";" } else { ":" };
        assert_eq!(result, vec![format!("a.jar{expected_separator}b.jar")]);
    }

    #[test]
    fn leaves_unknown_placeholder_untouched() {
        let tokens = vec!["${totally_unknown_key}".to_string()];
        let result = substitute_placeholders(&tokens, &sample_vars());
        assert_eq!(result, vec!["${totally_unknown_key}"]);
    }

    #[test]
    fn substitutes_multiple_placeholders_in_one_token() {
        let tokens = vec!["${auth_player_name}:${version_name}".to_string()];
        let result = substitute_placeholders(&tokens, &sample_vars());
        assert_eq!(result, vec!["BDarkBr:1.21.1"]);
    }

    #[test]
    fn split_legacy_arguments_tokenizes_on_whitespace() {
        let tokens = split_legacy_arguments("--username ${auth_player_name} --uuid ${auth_uuid}");
        assert_eq!(tokens, vec!["--username", "${auth_player_name}", "--uuid", "${auth_uuid}"]);
    }

    #[test]
    fn resolve_argument_entries_keeps_plain_strings() {
        let entries: Vec<ArgumentEntry> =
            serde_json::from_str(r#"["--username", "${auth_player_name}"]"#).unwrap();
        let resolved = resolve_argument_entries(&entries, TargetOs::Windows);
        assert_eq!(resolved, vec!["--username", "${auth_player_name}"]);
    }

    #[test]
    fn resolve_argument_entries_applies_os_rule() {
        // formato real confirmado em piston-meta pro arguments.jvm
        let entries: Vec<ArgumentEntry> = serde_json::from_str(
            r#"[{"rules":[{"action":"allow","os":{"name":"windows"}}],"value":"-XX:HeapDumpPath=x"}]"#,
        )
        .unwrap();
        assert_eq!(resolve_argument_entries(&entries, TargetOs::Windows), vec!["-XX:HeapDumpPath=x"]);
        assert!(resolve_argument_entries(&entries, TargetOs::Linux).is_empty());
    }

    #[test]
    fn resolve_argument_entries_disabled_feature_is_skipped() {
        // formato real confirmado pro arguments.game (quick play)
        let entries: Vec<ArgumentEntry> = serde_json::from_str(
            r#"[{"rules":[{"action":"allow","features":{"is_demo_user":true}}],"value":"--demo"}]"#,
        )
        .unwrap();
        assert!(resolve_argument_entries(&entries, TargetOs::Windows).is_empty());
    }

    #[test]
    fn resolve_argument_entries_expands_multi_value() {
        let entries: Vec<ArgumentEntry> =
            serde_json::from_str(r#"[{"rules":[],"value":["--a","--b"]}]"#).unwrap();
        assert_eq!(resolve_argument_entries(&entries, TargetOs::Windows), vec!["--a", "--b"]);
    }
}
