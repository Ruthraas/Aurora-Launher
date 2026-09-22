use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::rules::{rules_allow, Rule, TargetOs};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    /// Só existe no formato legado (pré-1.19): mapa SO → classifier.
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    /// Só existe no formato legado, junto com `natives`.
    #[serde(default)]
    pub extract: Option<Extract>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NativeArtifact {
    pub artifact: Artifact,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedLibraries {
    /// Jars que vão direto no `-cp` da JVM.
    pub classpath: Vec<Artifact>,
    /// Jars que precisam ser baixados e EXTRAÍDOS (contêm .dll/.so/.dylib,
    /// não vão no classpath).
    pub natives: Vec<NativeArtifact>,
}

/// Decide, pra um SO alvo, quais bibliotecas de uma versão entram no
/// classpath e quais são natives a extrair. Cobre os dois formatos:
/// o legado (mapa `natives` + `downloads.classifiers`) e o moderno
/// (biblioteca normal cujo `name` termina em `:natives-<os>`).
pub fn resolve_libraries(libraries: &[Library], os: TargetOs) -> ResolvedLibraries {
    let mut resolved = ResolvedLibraries::default();

    for library in libraries {
        if !rules_allow(&library.rules, os) {
            continue;
        }
        let Some(downloads) = &library.downloads else {
            continue;
        };

        if let Some(natives_map) = &library.natives {
            if let Some(classifier) = natives_map.get(os.name_key()) {
                if let Some(artifact) = downloads.classifiers.as_ref().and_then(|c| c.get(classifier)) {
                    resolved.natives.push(NativeArtifact {
                        artifact: artifact.clone(),
                        exclude: library.extract.clone().unwrap_or_default().exclude,
                    });
                }
            }
        }

        if let Some(artifact) = &downloads.artifact {
            if library.name.ends_with(os.natives_classifier()) {
                // LWJGL moderno (3.4+) guarda os .dll em subpastas dentro do
                // próprio jar (ex. `windows/x64/org/lwjgl/lwjgl.dll`) e tem
                // seu próprio mecanismo de auto-extração via
                // `-Dorg.lwjgl.system.SharedLibraryExtractPath`, que lê o
                // binário certo como recurso do classpath — por isso o jar
                // native também precisa ir pro classpath, não só ser
                // extraído (extrair sozinho não é o bastante: o
                // `-Djava.library.path` não pesquisa subpastas).
                resolved.classpath.push(artifact.clone());
                resolved.natives.push(NativeArtifact { artifact: artifact.clone(), exclude: Vec::new() });
            } else {
                resolved.classpath.push(artifact.clone());
            }
        }
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_library_goes_to_classpath() {
        let libraries: Vec<Library> = serde_json::from_str(
            r#"[{
                "name": "com.google.code.gson:gson:2.11.0",
                "downloads": {
                    "artifact": {"path": "a/b.jar", "sha1": "abc", "size": 10, "url": "https://libraries.minecraft.net/a/b.jar"}
                }
            }]"#,
        )
        .unwrap();

        let resolved = resolve_libraries(&libraries, TargetOs::Windows);
        assert_eq!(resolved.classpath.len(), 1);
        assert!(resolved.natives.is_empty());
    }

    #[test]
    fn modern_native_library_goes_to_classpath_and_natives() {
        // formato real confirmado em piston-meta: LWJGL 3.4.3, sem
        // "natives"/"classifiers", identificado só pelo sufixo do name.
        // Precisa ir pros dois: classpath (LWJGL se auto-extrai lendo o
        // jar como recurso) E natives (outras libs, tipo jtracy, ainda
        // dependem da extração tradicional pro java.library.path).
        let libraries: Vec<Library> = serde_json::from_str(
            r#"[{
                "name": "org.lwjgl:lwjgl-opengl:3.4.3:natives-windows",
                "downloads": {
                    "artifact": {
                        "path": "org/lwjgl/lwjgl-opengl/3.4.3/lwjgl-opengl-3.4.3-natives-windows.jar",
                        "sha1": "8f46f873190aa1d89f0f59818afcf125bd88adde",
                        "size": 98282,
                        "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl-opengl/3.4.3/lwjgl-opengl-3.4.3-natives-windows.jar"
                    }
                },
                "rules": [{"action": "allow", "os": {"name": "windows"}}]
            }]"#,
        )
        .unwrap();

        let resolved = resolve_libraries(&libraries, TargetOs::Windows);
        assert_eq!(resolved.classpath.len(), 1);
        assert_eq!(resolved.natives.len(), 1);

        let resolved_linux = resolve_libraries(&libraries, TargetOs::Linux);
        assert!(resolved_linux.classpath.is_empty());
        assert!(resolved_linux.natives.is_empty());
    }

    #[test]
    fn legacy_native_library_uses_classifiers_map() {
        let libraries: Vec<Library> = serde_json::from_str(
            r#"[{
                "name": "org.lwjgl.lwjgl:lwjgl-platform:2.9.4-nightly-20150209",
                "natives": {"windows": "natives-windows", "linux": "natives-linux", "osx": "natives-osx"},
                "downloads": {
                    "classifiers": {
                        "natives-windows": {"path": "n/win.jar", "sha1": "a", "size": 1, "url": "https://libraries.minecraft.net/n/win.jar"},
                        "natives-linux": {"path": "n/linux.jar", "sha1": "b", "size": 1, "url": "https://libraries.minecraft.net/n/linux.jar"}
                    }
                },
                "extract": {"exclude": ["META-INF/"]}
            }]"#,
        )
        .unwrap();

        let resolved = resolve_libraries(&libraries, TargetOs::Windows);
        assert!(resolved.classpath.is_empty());
        assert_eq!(resolved.natives.len(), 1);
        assert_eq!(resolved.natives[0].artifact.path, "n/win.jar");
        assert_eq!(resolved.natives[0].exclude, vec!["META-INF/".to_string()]);
    }

    #[test]
    fn rules_filter_out_libraries_for_other_os() {
        let libraries: Vec<Library> = serde_json::from_str(
            r#"[{
                "name": "some:lib:1.0",
                "downloads": {"artifact": {"path": "a.jar", "sha1": "a", "size": 1, "url": "https://libraries.minecraft.net/a.jar"}},
                "rules": [{"action": "allow", "os": {"name": "osx"}}]
            }]"#,
        )
        .unwrap();

        assert!(resolve_libraries(&libraries, TargetOs::Windows).classpath.is_empty());
        assert_eq!(resolve_libraries(&libraries, TargetOs::MacOs).classpath.len(), 1);
    }
}
