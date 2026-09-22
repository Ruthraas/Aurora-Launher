use serde::Serialize;

use crate::core::instances::LoaderKind;

/// Um mod da lista curada de otimização. `project_id` é o id INTERNO
/// do Modrinth (ex.: `"AANobbMI"` pro Sodium), não o slug (`"sodium"`)
/// — bug real encontrado: a busca normal (Discover) sempre grava o id
/// resolvido no `mods.lock.json` (é o que a API devolve pra
/// `project_id` de um resultado de busca), mas essa lista curada
/// gravava o slug. Instalar o MESMO mod pelos dois caminhos (Discover
/// e "Otimizar desempenho") criava duas entradas que o sistema não
/// reconhecia como a mesma coisa — "já instalado" nunca batia pra
/// quem tinha instalado via Descobrir primeiro. Resolvido usando o id
/// interno aqui também, igual todo o resto do app já fazia. Slug
/// mantido em comentário só pra referência humana / achar o mod de
/// novo na API se precisar.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationMod {
    pub project_id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

const SODIUM: OptimizationMod = OptimizationMod {
    project_id: "AANobbMI", // slug: sodium
    name: "Sodium",
    description: "Motor de renderização moderno para aumentar os FPS.",
};
const LITHIUM: OptimizationMod = OptimizationMod {
    project_id: "gvQqBUqZ", // slug: lithium
    name: "Lithium",
    description: "Otimiza a lógica do jogo sem alterar o comportamento.",
};
const FERRITE_CORE: OptimizationMod = OptimizationMod {
    project_id: "uXXizFIs", // slug: ferrite-core
    name: "FerriteCore",
    description: "Reduz o uso de memória RAM.",
};
const KRYPTON: OptimizationMod = OptimizationMod {
    project_id: "fQEb0iXm", // slug: krypton
    name: "Krypton",
    description: "Otimiza a pilha de rede (netty).",
};
const IMMEDIATELY_FAST: OptimizationMod = OptimizationMod {
    project_id: "5ZwdcRci", // slug: immediatelyfast
    name: "ImmediatelyFast",
    description: "Otimizações de renderização de itens e texto.",
};
const ENTITY_CULLING: OptimizationMod = OptimizationMod {
    project_id: "NNAgCjsB", // slug: entityculling
    name: "EntityCulling",
    description: "Para de renderizar entidades e block entities fora de visão.",
};
const MODERN_FIX: OptimizationMod = OptimizationMod {
    project_id: "nmDcB62a", // slug: modernfix
    name: "ModernFix",
    description: "Melhora o tempo de carregamento e corrige gargalos.",
};
const EMBEDDIUM: OptimizationMod = OptimizationMod {
    project_id: "sk9rgfiA", // slug: embeddium
    name: "Embeddium",
    description: "Motor de renderização moderno (família Sodium) para Forge/NeoForge.",
};
// O Sodium removeu a opção de vídeo "Fog: Off" das versões atuais
// (confirmado contra `sodium-options.json` reais de modpacks — o
// schema moderno só tem `use_fog_occlusion`, uma otimização interna,
// não um controle visual) e o `options.txt` vanilla nunca teve uma
// chave pra desligar névoa. Esse mod client-side dedicado (1.28M+
// downloads, confirmado via API do Modrinth) é o jeito real de
// cumprir "tirar o fog por padrão" — não dá pra fazer só com config.
const NO_FOG: OptimizationMod = OptimizationMod {
    project_id: "QSzy55SB", // slug: no_fog
    name: "No Fog",
    description: "Remove a névoa de distância, água, lava e nether — visibilidade total.",
};

/// Lista curada por família de loader — Fabric e Forge/NeoForge têm
/// ecossistemas de otimização DIFERENTES (mods de um não rodam no
/// outro), confirmado contra a API real antes de montar cada lista
/// (ver pesquisa que embasou isso). Quilt reaproveita a lista do
/// Fabric: praticamente todo mod de otimização Fabric roda em Quilt
/// (API de compatibilidade do Quilt), mesmo sem publicar uma versão
/// com a tag "quilt" — é por isso que `mods_compat::loader_name`
/// também consulta o Modrinth como "fabric" pra instâncias Quilt.
pub fn recommended_for(loader: &LoaderKind) -> &'static [OptimizationMod] {
    match loader {
        LoaderKind::Vanilla => &[],
        LoaderKind::Fabric { .. } | LoaderKind::Quilt { .. } => {
            &[SODIUM, LITHIUM, FERRITE_CORE, KRYPTON, IMMEDIATELY_FAST, ENTITY_CULLING, MODERN_FIX, NO_FOG]
        }
        // NeoForge moderno já recebe builds nativas do Sodium — não
        // precisa do Embeddium (evita dois renderizadores concorrentes
        // instalados juntos).
        LoaderKind::NeoForge { .. } => {
            &[SODIUM, LITHIUM, FERRITE_CORE, IMMEDIATELY_FAST, ENTITY_CULLING, MODERN_FIX, NO_FOG]
        }
        // Forge clássico não tem build do Sodium — Embeddium é o
        // substituto da família Sodium pra essa base.
        LoaderKind::Forge { .. } => &[EMBEDDIUM, FERRITE_CORE, ENTITY_CULLING, MODERN_FIX, NO_FOG],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_has_no_recommendations() {
        assert!(recommended_for(&LoaderKind::Vanilla).is_empty());
    }

    #[test]
    fn fabric_and_quilt_share_the_same_list() {
        let fabric = recommended_for(&LoaderKind::Fabric { loader_version: "0.15.0".to_string() });
        let quilt = recommended_for(&LoaderKind::Quilt { loader_version: "0.20.0".to_string() });
        assert_eq!(fabric.len(), quilt.len());
        assert!(fabric.iter().any(|m| m.project_id == SODIUM.project_id));
    }

    #[test]
    fn forge_does_not_recommend_sodium_itself() {
        let forge = recommended_for(&LoaderKind::Forge { forge_version: "47.2.0".to_string() });
        assert!(!forge.iter().any(|m| m.project_id == SODIUM.project_id));
        assert!(forge.iter().any(|m| m.project_id == EMBEDDIUM.project_id));
    }

    #[test]
    fn project_ids_are_resolved_modrinth_ids_not_slugs() {
        // regressão do bug: id interno do Modrinth não tem hífen nem é
        // tudo minúsculo com "-" como um slug seria (ex.: "ferrite-core"
        // vira "uXXizFIs") — checagem simples pra nunca voltar a colar
        // um slug aqui sem perceber.
        for m in recommended_for(&LoaderKind::Fabric { loader_version: "0.15.0".to_string() }) {
            assert!(!m.project_id.contains('-'), "{} parece um slug, não um id resolvido", m.project_id);
        }
    }
}
