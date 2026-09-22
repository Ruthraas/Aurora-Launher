use serde::{Deserialize, Serialize};

use super::arguments::{resolve_argument_entries, split_legacy_arguments, ArgumentEntry};
use super::library::Library;
use super::rules::TargetOs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetails {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub main_class: String,
    pub downloads: VersionDownloads,
    pub asset_index: AssetIndexRef,
    pub assets: String,
    #[serde(default)]
    pub java_version: Option<JavaVersionRef>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// Só presente em versões pré-1.13 — ver `resolve_game_arguments`.
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<ArgumentEntry>,
    #[serde(default)]
    pub jvm: Vec<ArgumentEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRef {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersionRef {
    pub component: String,
    pub major_version: u32,
}

impl VersionDetails {
    /// Versões bem antigas não têm `javaVersion` no JSON — Java 8 é o
    /// fallback seguro pra elas (é o que essas versões esperavam na
    /// época).
    pub fn required_java_major_version(&self) -> u32 {
        self.java_version.as_ref().map(|j| j.major_version).unwrap_or(8)
    }

    /// Nome do runtime no manifesto de Java da Mojang (ex.:
    /// `"java-runtime-delta"`). Versões sem `javaVersion` no JSON usam
    /// o runtime legado — é o que existia antes desse campo existir.
    pub fn required_java_component(&self) -> &str {
        self.java_version.as_ref().map(|j| j.component.as_str()).unwrap_or("jre-legacy")
    }

    pub fn is_legacy(&self) -> bool {
        self.arguments.is_none()
    }

    /// Tokens de argumento de jogo (ainda com `${placeholders}`),
    /// resolvidos pro SO atual. Funciona pros dois formatos.
    pub fn resolve_game_arguments(&self, os: TargetOs) -> Vec<String> {
        match &self.arguments {
            Some(args) => resolve_argument_entries(&args.game, os),
            None => self
                .minecraft_arguments
                .as_deref()
                .map(split_legacy_arguments)
                .unwrap_or_default(),
        }
    }

    /// Idem para os argumentos de JVM. Versões legadas não trazem essa
    /// lista — quem monta o comando final aplica flags próprias nesse
    /// caso (preset de otimização, Fase 9).
    pub fn resolve_jvm_arguments(&self, os: TargetOs) -> Vec<String> {
        match &self.arguments {
            Some(args) => resolve_argument_entries(&args.jvm, os),
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modern_fixture() -> VersionDetails {
        serde_json::from_str(
            r#"{
                "id": "1.21.1",
                "type": "release",
                "mainClass": "net.minecraft.client.main.Main",
                "downloads": {"client": {"sha1": "a", "size": 1, "url": "https://x/client.jar"}},
                "assetIndex": {"id": "17", "sha1": "b", "size": 1, "url": "https://x/17.json"},
                "assets": "17",
                "javaVersion": {"component": "java-runtime-delta", "majorVersion": 21},
                "libraries": [],
                "arguments": {
                    "game": ["--username", "${auth_player_name}"],
                    "jvm": [{"rules": [{"action": "allow", "os": {"name": "windows"}}], "value": "-Dwin=1"}]
                }
            }"#,
        )
        .unwrap()
    }

    fn legacy_fixture() -> VersionDetails {
        serde_json::from_str(
            r#"{
                "id": "1.12.2",
                "type": "release",
                "mainClass": "net.minecraft.client.main.Main",
                "downloads": {"client": {"sha1": "a", "size": 1, "url": "https://x/client.jar"}},
                "assetIndex": {"id": "1.12", "sha1": "b", "size": 1, "url": "https://x/1.12.json"},
                "assets": "1.12",
                "libraries": [],
                "minecraftArguments": "--username ${auth_player_name} --uuid ${auth_uuid}"
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn modern_version_is_not_legacy_and_has_java_version() {
        let version = modern_fixture();
        assert!(!version.is_legacy());
        assert_eq!(version.required_java_major_version(), 21);
    }

    #[test]
    fn modern_version_resolves_game_and_jvm_arguments() {
        let version = modern_fixture();
        assert_eq!(version.resolve_game_arguments(TargetOs::Windows), vec!["--username", "${auth_player_name}"]);
        assert_eq!(version.resolve_jvm_arguments(TargetOs::Windows), vec!["-Dwin=1"]);
        assert!(version.resolve_jvm_arguments(TargetOs::Linux).is_empty());
    }

    #[test]
    fn legacy_version_falls_back_to_java_8_and_splits_minecraft_arguments() {
        let version = legacy_fixture();
        assert!(version.is_legacy());
        assert_eq!(version.required_java_major_version(), 8);
        assert_eq!(
            version.resolve_game_arguments(TargetOs::Windows),
            vec!["--username", "${auth_player_name}", "--uuid", "${auth_uuid}"]
        );
        assert!(version.resolve_jvm_arguments(TargetOs::Windows).is_empty());
    }
}
