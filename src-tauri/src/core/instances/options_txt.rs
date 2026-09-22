use std::path::Path;

use crate::error::AppError;

/// Chaves do `options.txt` que mexem direto em FPS, com os valores
/// que a comunidade usa como padrão de "máxima performance" — nada
/// exótico, é o mesmo conjunto que qualquer guia de otimização manual
/// recomenda (baixar render/simulation distance, cortar partículas,
/// desligar AO/sombra de entidade/VSync/nuvens, gráficos "fast").
/// Só essas chaves são tocadas; idioma, resolução, keybinds e tudo
/// mais que o jogador (ou o autor do modpack, via `overrides/`) já
/// tenha configurado fica como está. O parser do próprio Minecraft
/// ignora silenciosamente chave desconhecida ou fora da versão atual
/// (não é erro fatal), então é seguro aplicar o mesmo preset em
/// qualquer versão do jogo sem checar qual delas é.
const PERFORMANCE_KEYS: &[(&str, &str)] = &[
    ("renderDistance", "8"),
    ("simulationDistance", "8"),
    ("particles", "2"),
    ("ao", "0"),
    ("entityShadows", "false"),
    ("enableVsync", "false"),
    ("graphicsMode", "0"),
    ("fancyGraphics", "false"),
    ("clouds", "false"),
    ("renderClouds", "false"),
    ("biomeBlendRadius", "0"),
    ("mipmapLevels", "0"),
    ("fboEnable", "true"),
    ("useVbo", "true"),
];

/// Aplica o preset de otimização no `options.txt` da instância. Faz
/// merge em cima do que já existir (chave conhecida é sobrescrita,
/// resto é preservado) em vez de sobrescrever o arquivo inteiro — se
/// não existir ainda, cria um novo só com o preset.
pub fn apply(instance_dir: &Path) -> Result<(), AppError> {
    let path = instance_dir.join("options.txt");
    let existing = if path.exists() { std::fs::read_to_string(&path)? } else { String::new() };
    let merged = merge(&existing, PERFORMANCE_KEYS);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, merged)?;
    Ok(())
}

fn merge(existing: &str, overrides: &[(&str, &str)]) -> String {
    let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();

    for (key, value) in overrides {
        let entry = format!("{key}:{value}");
        match lines.iter().position(|line| line.split_once(':').map(|(k, _)| k) == Some(*key)) {
            Some(i) => lines[i] = entry,
            None => lines.push(entry),
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_overrides_known_key_and_preserves_others() {
        let existing = "lang:pt_br\nrenderDistance:32\nkeybind:key.jump\n";
        let merged = merge(existing, &[("renderDistance", "8")]);
        assert!(merged.contains("lang:pt_br"));
        assert!(merged.contains("keybind:key.jump"));
        assert!(merged.contains("renderDistance:8"));
        assert!(!merged.contains("renderDistance:32"));
    }

    #[test]
    fn merge_appends_missing_key() {
        let merged = merge("lang:pt_br\n", &[("particles", "2")]);
        assert!(merged.contains("lang:pt_br"));
        assert!(merged.contains("particles:2"));
    }

    #[test]
    fn merge_from_empty_creates_only_preset() {
        let merged = merge("", &[("renderDistance", "8"), ("particles", "2")]);
        assert_eq!(merged, "renderDistance:8\nparticles:2\n");
    }
}
