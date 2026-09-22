/// Resolve uma coordenada maven (`grupo:artefato:versão[:classifier][@ext]`)
/// pro caminho padrão de repositório: `grupo/artefato/versão/artefato-versão[-classifier].ext`.
/// Usado tanto pelo Fabric quanto pelo NeoForge — os dois publicam
/// bibliotecas em repositórios maven comuns, resolvidos do mesmo jeito.
pub fn maven_coordinate_to_path(coordinate: &str) -> Option<String> {
    let (coordinate, extension) = match coordinate.split_once('@') {
        Some((base, ext)) => (base, ext),
        None => (coordinate, "jar"),
    };

    let parts: Vec<&str> = coordinate.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group_path = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts.get(3).map(|value| format!("-{value}")).unwrap_or_default();

    Some(format!("{group_path}/{artifact}/{version}/{artifact}-{version}{classifier}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_simple_coordinate() {
        assert_eq!(
            maven_coordinate_to_path("org.ow2.asm:asm:9.10.1").unwrap(),
            "org/ow2/asm/asm/9.10.1/asm-9.10.1.jar"
        );
    }

    #[test]
    fn resolves_coordinate_with_classifier_and_extension() {
        // formato real confirmado num install_profile.json de NeoForge.
        assert_eq!(
            maven_coordinate_to_path("net.neoforged:neoform:1.21.11-20251209.172050:mappings@tsrg.lzma").unwrap(),
            "net/neoforged/neoform/1.21.11-20251209.172050/neoform-1.21.11-20251209.172050-mappings.tsrg.lzma"
        );
    }

    #[test]
    fn resolves_output_only_coordinate_without_known_host() {
        // o jar "patchado" não tem URL nenhuma — é gerado localmente
        // pelos processors, mas o caminho segue a mesma convenção.
        assert_eq!(
            maven_coordinate_to_path("net.neoforged:minecraft-client-patched:21.11.45").unwrap(),
            "net/neoforged/minecraft-client-patched/21.11.45/minecraft-client-patched-21.11.45.jar"
        );
    }

    #[test]
    fn rejects_malformed_coordinate() {
        assert!(maven_coordinate_to_path("not-a-coordinate").is_none());
    }
}
