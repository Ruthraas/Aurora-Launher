use crate::core::instances::JvmFlagPreset;

/// "Aikar's flags" — conjunto de flags de G1GC amplamente publicado e
/// usado por servidores/launchers de Minecraft (não é invenção nossa,
/// é o padrão de fato da comunidade pra reduzir pausas de GC sem
/// mexer no comportamento do jogo). Não inclui `-Xms`/`-Xmx` — isso já
/// vem de `ram_min_mb`/`ram_max_mb` da instância.
const G1GC_OPTIMIZED: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:MaxGCPauseMillis=200",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+DisableExplicitGC",
    "-XX:+AlwaysPreTouch",
    "-XX:G1NewSizePercent=30",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:G1HeapRegionSize=8M",
    "-XX:G1ReservePercent=20",
    "-XX:G1HeapWastePercent=5",
    "-XX:G1MixedGCCountTarget=4",
    "-XX:InitiatingHeapOccupancyPercent=15",
    "-XX:G1MixedGCLiveThresholdPercent=90",
    "-XX:G1RSetUpdatingPauseTimePercent=5",
    "-XX:SurvivorRatio=32",
    "-XX:+PerfDisableSharedMem",
    "-XX:MaxTenuringThreshold=1",
];

pub fn flags(preset: JvmFlagPreset) -> Vec<String> {
    match preset {
        JvmFlagPreset::None => Vec::new(),
        JvmFlagPreset::G1gcOptimized => G1GC_OPTIMIZED.iter().map(|s| s.to_string()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_preset_adds_nothing() {
        assert!(flags(JvmFlagPreset::None).is_empty());
    }

    #[test]
    fn g1gc_preset_enables_g1gc() {
        assert!(flags(JvmFlagPreset::G1gcOptimized).contains(&"-XX:+UseG1GC".to_string()));
    }
}
