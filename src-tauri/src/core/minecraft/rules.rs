use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    #[serde(default)]
    pub os: Option<OsMatch>,
    #[serde(default)]
    pub features: Option<FeatureMatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsMatch {
    #[serde(default)]
    pub name: Option<String>,
    /// Confirmado em versões reais (ex.: `-Xss1M` só em `arch: "x86"`).
    #[serde(default)]
    pub arch: Option<String>,
    /// Confirmado em versões reais (ex.: build 26.3) pra escolher entre
    /// ZGC/G1GC por build do Windows. Não sabemos checar isso ainda —
    /// uma regra que tenha esse campo é tratada como "não confirmada"
    /// (ver `rule_matches`), nunca como um match — evita risco de
    /// aplicar duas flags de GC conflitantes e travar a JVM. O preset
    /// de GC da Fase 9 resolve essa otimização de forma explícita.
    #[serde(default, rename = "versionRange")]
    pub version_range: Option<serde_json::Value>,
}

/// Captura QUALQUER chave de feature (`is_demo_user`,
/// `has_custom_resolution`, `has_quick_plays_support`,
/// `is_quick_play_singleplayer`, ...) em vez de listar campos
/// conhecidos. Motivo: nenhuma feature está implementada hoje, então
/// uma regra que exija qualquer uma delas nunca deve casar — se eu
/// listasse só as que já conheço, uma chave nova e desconhecida seria
/// silenciosamente ignorada pelo serde e a regra passaria a "casar"
/// por engano. Foi exatamente isso que aconteceu com quick play: eu só
/// checava duas chaves, e as quatro de quick play (desconhecidas pra
/// mim) vazavam como se estivessem ativas, mandando os 4 argumentos de
/// quick play ao mesmo tempo — e o Minecraft recusa mais de um.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FeatureMatch(HashMap<String, bool>);

impl FeatureMatch {
    fn any_enabled(&self) -> bool {
        self.0.values().any(|enabled| *enabled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetOs {
    Windows,
    Linux,
    MacOs,
}

impl TargetOs {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            TargetOs::Windows
        } else if cfg!(target_os = "macos") {
            TargetOs::MacOs
        } else {
            TargetOs::Linux
        }
    }

    /// Nome usado pelo schema da Mojang pra identificar este SO, tanto
    /// em `rules[].os.name` quanto nas chaves do mapa `natives` legado.
    pub fn name_key(self) -> &'static str {
        match self {
            TargetOs::Windows => "windows",
            TargetOs::Linux => "linux",
            TargetOs::MacOs => "osx",
        }
    }

    fn matches_name(self, name: &str) -> bool {
        name == self.name_key()
    }

    /// Sufixo usado no `name` maven de bibliotecas nativas modernas,
    /// ex. `org.lwjgl:lwjgl-opengl:3.4.3:natives-windows`.
    pub fn natives_classifier(self) -> &'static str {
        match self {
            TargetOs::Windows => "natives-windows",
            TargetOs::Linux => "natives-linux",
            TargetOs::MacOs => "natives-osx",
        }
    }
}

/// Nome de arquitetura como a Mojang usa em `rules[].os.arch` — hoje só
/// existe pra marcar exceções de 32 bits (`"x86"`); tudo que não for
/// isso cai em `"x86_64"` (o caso comum, sem regra alguma na prática).
fn current_arch_key() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "aarch64" => "arm64",
        _ => "x86_64",
    }
}

/// Resolve uma lista de `rules` do formato do launcher oficial: sem
/// nenhuma regra, é permitido por padrão; havendo regras, vale a ação
/// da ÚLTIMA regra que casar com o contexto atual (não a primeira —
/// é assim que o launcher da Mojang resolve, permitindo padrões como
/// "permitido em geral, exceto no macOS").
pub fn rules_allow(rules: &[Rule], os: TargetOs) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        if rule_matches(rule, os) {
            allowed = rule.action == RuleAction::Allow;
        }
    }
    allowed
}

fn rule_matches(rule: &Rule, os: TargetOs) -> bool {
    if let Some(os_match) = &rule.os {
        if os_match.version_range.is_some() {
            return false;
        }
        if let Some(name) = &os_match.name {
            if !os.matches_name(name) {
                return false;
            }
        }
        if let Some(arch) = &os_match.arch {
            if arch != current_arch_key() {
                return false;
            }
        }
    }
    // Nenhuma feature (quick play, resolução customizada, ...) está
    // habilitada hoje — uma regra que exige qualquer feature ligada
    // nunca casa, seja ela qual for.
    if let Some(features) = &rule.features {
        if features.any_enabled() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_rules_means_allowed() {
        assert!(rules_allow(&[], TargetOs::Windows));
    }

    #[test]
    fn arch_gated_rule_only_matches_that_arch() {
        // formato real confirmado em 26.3: "-Xss1M" só pra os.arch == "x86".
        let rules: Vec<Rule> =
            serde_json::from_str(r#"[{"action":"allow","os":{"arch":"x86"}}]"#).unwrap();
        let should_match = current_arch_key() == "x86";
        assert_eq!(rules_allow(&rules, TargetOs::Windows), should_match);
    }

    #[test]
    fn version_range_rule_is_never_treated_as_a_match() {
        // formato real confirmado em 26.3: duas regras "windows" que só
        // se diferenciam por versionRange (ZGC vs G1GC). Sem checar a
        // versão de verdade, nenhuma das duas pode "ganhar" — incluir
        // as duas juntas faria a JVM recusar iniciar (dois GCs setados).
        let rules: Vec<Rule> = serde_json::from_str(
            r#"[{"action":"allow","os":{"name":"windows","versionRange":{"min":"10.0.17134"}}}]"#,
        )
        .unwrap();
        assert!(!rules_allow(&rules, TargetOs::Windows));
    }

    #[test]
    fn matching_os_allow_rule_passes_only_for_that_os() {
        let rules: Vec<Rule> =
            serde_json::from_str(r#"[{"action":"allow","os":{"name":"windows"}}]"#).unwrap();
        assert!(rules_allow(&rules, TargetOs::Windows));
        assert!(!rules_allow(&rules, TargetOs::Linux));
    }

    #[test]
    fn later_disallow_rule_overrides_earlier_allow() {
        let rules: Vec<Rule> = serde_json::from_str(
            r#"[{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]"#,
        )
        .unwrap();
        assert!(rules_allow(&rules, TargetOs::Windows));
        assert!(!rules_allow(&rules, TargetOs::MacOs));
    }

    #[test]
    fn feature_gated_rule_is_disabled_by_default() {
        let rules: Vec<Rule> =
            serde_json::from_str(r#"[{"action":"allow","features":{"is_demo_user":true}}]"#)
                .unwrap();
        assert!(!rules_allow(&rules, TargetOs::Windows));
    }

    #[test]
    fn unknown_feature_key_is_also_disabled_by_default() {
        // bug real: só "is_demo_user"/"has_custom_resolution" eram
        // checados antes, então chaves como "is_quick_play_singleplayer"
        // (desconhecidas) vazavam como se estivessem ativas — mandando
        // os 4 argumentos de quick play juntos, e o Minecraft recusa
        // mais de um ao mesmo tempo ("Only one quick play option can be
        // specified"). Qualquer chave desconhecida tem que continuar
        // desativada, não só as que a gente já conhece.
        let rules: Vec<Rule> = serde_json::from_str(
            r#"[{"action":"allow","features":{"is_quick_play_singleplayer":true}}]"#,
        )
        .unwrap();
        assert!(!rules_allow(&rules, TargetOs::Windows));
    }

    #[test]
    fn natives_classifier_matches_real_mojang_convention() {
        assert_eq!(TargetOs::Windows.natives_classifier(), "natives-windows");
        assert_eq!(TargetOs::MacOs.natives_classifier(), "natives-osx");
        assert_eq!(TargetOs::Linux.natives_classifier(), "natives-linux");
    }
}
