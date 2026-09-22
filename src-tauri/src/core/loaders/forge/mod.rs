pub mod install;

use crate::core::download::http::{client, with_retry};
use crate::error::AppError;

const MAVEN_METADATA_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const MAVEN_RELEASES_BASE: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

/// Versões do Forge compatíveis com `mc_version`, mais recente
/// primeiro. Diferente do NeoForge (que tem uma API JSON dedicada), o
/// Forge só publica o `maven-metadata.xml` padrão do Maven com TODAS
/// as versões de TODAS as versões do Minecraft numa lista só — o
/// formato de cada entrada é `<mc_version>-<forge_build>` (ex.:
/// "1.20.1-47.2.0"), então filtramos pelo prefixo `"<mc_version>-"`.
/// Confirmado contra o XML real.
pub async fn fetch_versions(mc_version: &str) -> Result<Vec<String>, AppError> {
    let xml = with_retry(|| async {
        let response = client().get(MAVEN_METADATA_URL).send().await?.error_for_status()?;
        Ok(response.text().await?)
    })
    .await?;

    let prefix = format!("{mc_version}-");
    let mut matching: Vec<String> = parse_maven_versions(&xml).into_iter().filter(|v| v.starts_with(&prefix)).collect();
    matching.reverse(); // o XML vem em ordem crescente, igual o NeoForge
    Ok(matching)
}

/// Extrai o conteúdo de cada `<version>...</version>` do
/// `maven-metadata.xml` — não vale a pena trazer uma dependência de
/// XML inteira só pra isso, o formato é simples e regular o bastante
/// pra um parser de string.
fn parse_maven_versions(xml: &str) -> Vec<String> {
    let mut versions = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<version>") {
        let after_tag = &rest[start + "<version>".len()..];
        let Some(end) = after_tag.find("</version>") else { break };
        versions.push(after_tag[..end].to_string());
        rest = &after_tag[end + "</version>".len()..];
    }
    versions
}

/// `forge_version` aqui é a versão combinada `<mc_version>-<build>`
/// que `fetch_versions` devolve, não só o número do build.
pub fn installer_url(forge_version: &str) -> String {
    format!("{MAVEN_RELEASES_BASE}/{forge_version}/forge-{forge_version}-installer.jar")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions_from_real_metadata_shape() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata>
  <groupId>net.minecraftforge</groupId>
  <artifactId>forge</artifactId>
  <versioning>
    <versions>
      <version>1.21-51.0.33</version>
      <version>1.20.1-47.2.0</version>
      <version>1.16.5-36.2.39</version>
    </versions>
  </versioning>
</metadata>"#;
        assert_eq!(parse_maven_versions(xml), vec!["1.21-51.0.33", "1.20.1-47.2.0", "1.16.5-36.2.39"]);
    }

    #[test]
    fn installer_url_uses_combined_version() {
        assert_eq!(
            installer_url("1.20.1-47.2.0"),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-installer.jar"
        );
    }
}
