use std::fs;
use std::path::Path;

use crate::error::AppError;

/// Lê uma entrada específica de um `.zip` (ex.: `install_profile.json`
/// de dentro de um instalador) pra memória, sem extrair o arquivo
/// inteiro.
pub fn read_zip_entry(zip_path: &Path, entry_name: &str) -> Result<Vec<u8>, AppError> {
    // O `std::io::Error` de um `File::open` que falha não carrega o
    // caminho na mensagem (só "arquivo não encontrado" cru) — sem
    // isso, um erro real de produção não dá pra saber qual dos vários
    // jars/zips envolvidos numa instalação de loader estava faltando.
    let file = fs::File::open(zip_path)
        .map_err(|error| AppError::NotFound(format!("arquivo \"{}\" ({error})", zip_path.display())))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = archive
        .by_name(entry_name)
        .map_err(|_| AppError::NotFound(format!("entrada \"{entry_name}\" no zip {}", zip_path.display())))?;
    let mut bytes = Vec::new();
    std::io::copy(&mut entry, &mut bytes)?;
    Ok(bytes)
}

/// Extrai só UMA entrada de um `.zip` pra `dest` (não o zip inteiro) —
/// usado pra puxar arquivos de dados de dentro do instalador do
/// NeoForge (ex.: `data/client.lzma`) sem descompactar tudo.
pub fn extract_zip_entry_to_file(zip_path: &Path, entry_name: &str, dest: &Path) -> Result<(), AppError> {
    let bytes = read_zip_entry(zip_path, entry_name)?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dest, bytes)?;
    Ok(())
}

/// Extrai um `.zip` pra `dest_dir`, pulando entradas cujo caminho
/// comece com algum prefixo de `exclude` (ex.: `"META-INF/"`, usado
/// pelas bibliotecas nativas legadas).
///
/// Proteção contra zip-slip: usa `ZipFile::enclosed_name()`, que a
/// própria crate `zip` recusa a resolver (devolve `None`) pra qualquer
/// entrada absoluta ou com `..` — essas entradas são simplesmente
/// puladas, nunca escrevem fora de `dest_dir`.
pub fn extract_zip(zip_path: &Path, dest_dir: &Path, exclude: &[String]) -> Result<(), AppError> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    fs::create_dir_all(dest_dir)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(relative_path) = entry.enclosed_name() else {
            continue;
        };

        let relative_str = relative_path.to_string_lossy().replace('\\', "/");
        if exclude.iter().any(|prefix| relative_str.starts_with(prefix.as_str())) {
            continue;
        }

        let out_path = dest_dir.join(&relative_path);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out_file = fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
    }

    Ok(())
}

/// Extrai só as entradas sob `prefix` (ex.: `"overrides/"`) pra
/// `dest_dir`, com o prefixo removido do caminho final — é assim que
/// `.mrpack`/modpacks do CurseForge embutem os arquivos extra da
/// instância (configs, resourcepacks, ...) dentro do próprio zip do
/// pack. Mesma proteção zip-slip de `extract_zip` (`enclosed_name()`).
pub fn extract_prefixed(zip_path: &Path, prefix: &str, dest_dir: &Path) -> Result<(), AppError> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(relative_path) = entry.enclosed_name() else {
            continue;
        };

        let relative_str = relative_path.to_string_lossy().replace('\\', "/");
        let Some(stripped) = relative_str.strip_prefix(prefix) else { continue };
        if stripped.is_empty() {
            continue;
        }

        let out_path = dest_dir.join(stripped);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out_file = fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
    }

    Ok(())
}

/// Garante que `META-INF/MANIFEST.MF` dentro do zip tenha a linha
/// `<attribute>: <value>`, reescrevendo o zip inteiro se precisar
/// adicionar (não mexe em mais nada se a chave já existir). Usado pro
/// NeoForge: o jar "patchado" que os processors geram não tem o
/// atributo `Minecraft-Dists` que o FancyModLoader passou a exigir a
/// partir da Minecraft 1.21.7 — atributo que normalmente só o Gradle
/// do NeoForge adiciona, não o instalador. Sem ele, o jogo recusa
/// iniciar com "Fatal Startup Error" mesmo o jar estando correto.
pub fn ensure_manifest_attribute(zip_path: &Path, attribute: &str, value: &str) -> Result<(), AppError> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let existing_manifest = {
        let mut entry = archive
            .by_name("META-INF/MANIFEST.MF")
            .map_err(|_| AppError::NotFound(format!("META-INF/MANIFEST.MF em {}", zip_path.display())))?;
        let mut content = String::new();
        std::io::Read::read_to_string(&mut entry, &mut content)?;
        content
    };

    let attribute_line = format!("{attribute}: {value}");
    if existing_manifest.lines().any(|line| line.trim() == attribute_line) {
        return Ok(()); // já está lá, nada a fazer.
    }

    // O manifest tem uma seção PRINCIPAL (Manifest-Version, Main-Class,
    // ...) seguida de uma linha em branco, e só DEPOIS as seções
    // por-entrada (Name: .../SHA-384-Digest: ...). Um atributo "solto"
    // só é lido como global se estiver na seção principal — jogá-lo no
    // final do arquivo (depois de milhares de seções por-entrada) faz
    // ele virar lixo de continuação da última entrada, e o FancyModLoader
    // simplesmente não o vê. Por isso ele precisa entrar ANTES da
    // primeira linha em branco, nunca no fim do arquivo.
    let new_manifest = match existing_manifest.split_once("\r\n\r\n").or_else(|| existing_manifest.split_once("\n\n")) {
        Some((main_section, rest)) => {
            format!("{main_section}\r\n{attribute_line}\r\n\r\n{rest}")
        }
        None => {
            // sem seções por-entrada — só a seção principal.
            let mut manifest = existing_manifest;
            if !manifest.ends_with('\n') {
                manifest.push_str("\r\n");
            }
            manifest.push_str(&attribute_line);
            manifest.push_str("\r\n");
            manifest
        }
    };

    let tmp_path = {
        let mut path = zip_path.as_os_str().to_owned();
        path.push(".rewrite");
        std::path::PathBuf::from(path)
    };
    {
        let out_file = fs::File::create(&tmp_path)?;
        let mut writer = zip::ZipWriter::new(out_file);
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();

            // Trata toda entrada (arquivo ou diretório) do mesmo jeito
            // via `start_file` — `add_directory` com um nome que já vem
            // terminado em "/" duplicava a barra e corrompia o zip
            // (jar de 34 mil entradas virou "not a jar file").
            writer.start_file(&name, options)?;
            if name == "META-INF/MANIFEST.MF" {
                std::io::Write::write_all(&mut writer, new_manifest.as_bytes())?;
            } else {
                std::io::copy(&mut entry, &mut writer)?;
            }
        }

        writer.finish()?;
    }

    // Solta o handle de leitura do zip original ANTES de renomear por
    // cima dele — no Windows, `fs::rename` pode falhar com "Acesso
    // negado (os error 5)" se o arquivo de destino ainda estiver aberto
    // (o `archive`/`file` de leitura ficava vivo até o fim da função).
    drop(archive);

    // O rename em si pode falhar se algo mais (ex.: um processo Java de
    // um lançamento anterior, ou antivírus) ainda tiver o jar aberto —
    // tenta copiar+substituir como fallback antes de desistir.
    if let Err(err) = fs::rename(&tmp_path, zip_path) {
        fs::copy(&tmp_path, zip_path).map_err(|_| err)?;
        fs::remove_file(&tmp_path).ok();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_test_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
        for (name, contents) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn extract_prefixed_strips_prefix_and_skips_other_entries() {
        let dir = std::env::temp_dir().join(format!("aurora-zip-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("pack.mrpack");
        let out_dir = dir.join("out");

        write_test_zip(
            &zip_path,
            &[
                ("modrinth.index.json", b"{}"),
                ("overrides/config/mod.toml", b"setting=1"),
                ("overrides/resourcepacks/pack.zip", b"fake-zip-bytes"),
            ],
        );

        extract_prefixed(&zip_path, "overrides/", &out_dir).unwrap();

        assert_eq!(fs::read(out_dir.join("config/mod.toml")).unwrap(), b"setting=1");
        assert!(out_dir.join("resourcepacks/pack.zip").exists());
        assert!(!out_dir.join("modrinth.index.json").exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn extracts_regular_entries() {
        let dir = std::env::temp_dir().join(format!("aurora-zip-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("in.zip");
        let out_dir = dir.join("out");

        write_test_zip(&zip_path, &[("lib.dll", b"fake-native-bytes"), ("META-INF/MANIFEST.MF", b"skip-me")]);

        extract_zip(&zip_path, &out_dir, &["META-INF/".to_string()]).unwrap();

        assert!(out_dir.join("lib.dll").exists());
        assert!(!out_dir.join("META-INF").exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ensure_manifest_attribute_adds_missing_attribute() {
        let dir = std::env::temp_dir().join(format!("aurora-zip-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("patched.jar");

        write_test_zip(
            &zip_path,
            &[
                ("META-INF/", b""),
                ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\nMain-Class: foo.Bar\r\n"),
                ("com/mojang/", b""),
                ("com/mojang/a.class", b"bytes"),
            ],
        );

        ensure_manifest_attribute(&zip_path, "Minecraft-Dists", "client").unwrap();

        let manifest = read_zip_entry(&zip_path, "META-INF/MANIFEST.MF").unwrap();
        let manifest = String::from_utf8(manifest).unwrap();
        assert!(manifest.contains("Minecraft-Dists: client"));
        assert!(manifest.contains("Main-Class: foo.Bar")); // resto preservado

        // as outras entradas (inclusive diretórios) continuam lá depois
        // de reescrever o zip, e o arquivo continua um zip válido.
        assert_eq!(read_zip_entry(&zip_path, "com/mojang/a.class").unwrap(), b"bytes");
        let file = fs::File::open(&zip_path).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        assert_eq!(archive.len(), 4);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ensure_manifest_attribute_inserts_before_per_entry_sections() {
        // Reproduz o bug real: um manifest de jar de verdade tem seção
        // principal + várias seções "Name: .../Digest: ..." por entrada.
        // O atributo precisa ir na seção PRINCIPAL, não grudado no fim
        // do arquivo (senão vira lixo de continuação da última entrada
        // e o FancyModLoader não reconhece como atributo global).
        let dir = std::env::temp_dir().join(format!("aurora-zip-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("patched.jar");

        let manifest_bytes = b"Manifest-Version: 1.0\r\nMain-Class: foo.Bar\r\n\r\nName: com/mojang/a.class\r\nSHA-384-Digest: abc123\r\n";
        write_test_zip(&zip_path, &[("META-INF/MANIFEST.MF", manifest_bytes)]);

        ensure_manifest_attribute(&zip_path, "Minecraft-Dists", "client").unwrap();

        let manifest = read_zip_entry(&zip_path, "META-INF/MANIFEST.MF").unwrap();
        let manifest = String::from_utf8(manifest).unwrap();

        let main_section = manifest.split("\r\n\r\n").next().unwrap();
        assert!(main_section.contains("Minecraft-Dists: client"), "atributo devia estar na seção principal: {manifest}");

        // seção por-entrada continua intacta, sem o atributo grudado nela.
        let per_entry_section = manifest.split("\r\n\r\n").nth(1).unwrap();
        assert!(per_entry_section.contains("SHA-384-Digest: abc123"));
        assert!(!per_entry_section.contains("Minecraft-Dists"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ensure_manifest_attribute_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("aurora-zip-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("patched.jar");

        write_test_zip(&zip_path, &[("META-INF/MANIFEST.MF", b"Minecraft-Dists: client\r\n")]);

        ensure_manifest_attribute(&zip_path, "Minecraft-Dists", "client").unwrap();
        let manifest = read_zip_entry(&zip_path, "META-INF/MANIFEST.MF").unwrap();
        let manifest = String::from_utf8(manifest).unwrap();
        assert_eq!(manifest.matches("Minecraft-Dists").count(), 1);

        fs::remove_dir_all(&dir).unwrap();
    }
}
