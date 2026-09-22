use std::fs;
use std::path::{Path, PathBuf};

use uuid::Uuid;
use zip::write::SimpleFileOptions;

use super::{Instance, InstanceStatus, InstanceStore, INSTANCE_FILE};
use crate::error::AppError;

/// Pastas que não fazem sentido levar num export — específicas da
/// máquina/sessão que gerou o log, sem valor nenhum pra quem importa
/// (e só engordam o `.zip`).
const EXCLUDED_DIRS: &[&str] = &["logs", "crash-reports"];

/// Empacota a instância inteira (config, `mods.lock.json`, mods,
/// resourcepacks, shaders, datapacks, saves, `options.txt`) num
/// `.zip` — NÃO inclui bibliotecas/assets/client.jar/runtime Java,
/// porque esses vivem no cache compartilhado entre instâncias (ver
/// `SharedCache`), não dentro da pasta da instância; quem importa
/// reinstala isso automaticamente (mesmo pipeline de "criar
/// instância"), o cache local é que decide se precisa baixar de novo.
pub fn export_instance(store: &InstanceStore, id: Uuid, dest_zip: &Path) -> Result<(), AppError> {
    let instance_dir = store.instance_dir(id);
    if !instance_dir.exists() {
        return Err(AppError::NotFound(format!("instância {id}")));
    }

    let file = fs::File::create(dest_zip)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    add_dir_recursive(&mut zip, &instance_dir, &instance_dir, options)?;
    zip.finish().map_err(|e| AppError::InvalidInput(e.to_string()))?;
    Ok(())
}

fn add_dir_recursive(
    zip: &mut zip::ZipWriter<fs::File>,
    base: &Path,
    dir: &Path,
    options: SimpleFileOptions,
) -> Result<(), AppError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(base).expect("entrada sempre é filha de `base`");
        let relative_str = relative.to_string_lossy().replace('\\', "/");

        if EXCLUDED_DIRS.iter().any(|excluded| relative_str == *excluded || relative_str.starts_with(&format!("{excluded}/"))) {
            continue;
        }

        if path.is_dir() {
            add_dir_recursive(zip, base, &path, options)?;
        } else {
            zip.start_file(relative_str, options).map_err(|e| AppError::InvalidInput(e.to_string()))?;
            let mut source = fs::File::open(&path)?;
            std::io::copy(&mut source, zip)?;
        }
    }
    Ok(())
}

/// Extrai um `.zip` gerado por `export_instance` pra uma instância
/// NOVA (sempre um id novo — nunca reusa o do export, tanto pra não
/// colidir com uma instância já existente quanto pra permitir
/// importar o mesmo `.zip` mais de uma vez). Devolve a `Instance` já
/// salva — o status vem como `Installing` de propósito: bibliotecas/
/// assets/client.jar/Java não vêm no `.zip` (ver `export_instance`),
/// então quem chama isso precisa disparar a mesma instalação usada em
/// "criar instância" (`spawn_install`) logo em seguida, senão a
/// instância importada não consegue rodar.
pub fn import_instance(store: &InstanceStore, zip_path: &Path) -> Result<Instance, AppError> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| AppError::InvalidInput(format!("zip inválido: {e}")))?;

    let mut instance: Instance = {
        let mut entry = archive
            .by_name(INSTANCE_FILE)
            .map_err(|_| AppError::InvalidInput("zip não é uma instância exportada pelo Aurora Launcher (sem instance.json)".to_string()))?;
        let mut raw = String::new();
        std::io::Read::read_to_string(&mut entry, &mut raw)?;
        serde_json::from_str(&raw)?
    };

    let new_id = Uuid::new_v4();
    instance.id = new_id;
    instance.name = store.unique_name(&instance.name)?;
    instance.status = InstanceStatus::Installing;
    instance.error_message = None;
    instance.created_at = chrono::Utc::now();
    instance.last_played = None;

    let dest_dir = store.instance_dir(new_id);
    fs::create_dir_all(&dest_dir)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| AppError::InvalidInput(e.to_string()))?;
        let Some(relative_path) = entry.enclosed_name() else { continue };
        if relative_path == PathBuf::from(INSTANCE_FILE) {
            // reescrito por `store.save` logo abaixo, já com
            // id/nome/status corrigidos — não extrai o cru do zip.
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

    store.save(&instance)?;
    Ok(instance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::instances::LoaderKind;

    fn temp_store() -> InstanceStore {
        InstanceStore::new(std::env::temp_dir().join(format!("aurora-transfer-{}", Uuid::new_v4())))
    }

    #[test]
    fn export_then_import_round_trips_instance_metadata() {
        let store = temp_store();
        let created = store.create("Minha Instância".to_string(), "1.21.1".to_string(), LoaderKind::Vanilla, 1024, 2048).unwrap();

        // um mod fake dentro da instância, pra confirmar que o
        // conteúdo (não só o instance.json) sobrevive à viagem.
        let mods_dir = store.instance_dir(created.id).join("mods");
        fs::create_dir_all(&mods_dir).unwrap();
        fs::write(mods_dir.join("fake.jar"), b"conteudo-fake").unwrap();

        // pasta que NÃO devia sobreviver ao export.
        let logs_dir = store.instance_dir(created.id).join("logs");
        fs::create_dir_all(&logs_dir).unwrap();
        fs::write(logs_dir.join("launcher-stdout.log"), b"log antigo").unwrap();

        let zip_path = std::env::temp_dir().join(format!("aurora-export-test-{}.zip", Uuid::new_v4()));
        export_instance(&store, created.id, &zip_path).unwrap();

        let imported = import_instance(&store, &zip_path).unwrap();
        assert_ne!(imported.id, created.id, "importar tem que gerar um id novo, nunca reusar o do export");
        // a instância original ainda existe no store, então o nome
        // colide e ganha o sufixo (mesma regra de `unique_name`).
        assert_eq!(imported.name, "Minha Instância (2)");
        assert_eq!(imported.mc_version, "1.21.1");
        assert!(matches!(imported.status, InstanceStatus::Installing));

        let imported_jar = store.instance_dir(imported.id).join("mods").join("fake.jar");
        assert_eq!(fs::read(&imported_jar).unwrap(), b"conteudo-fake");

        assert!(!store.instance_dir(imported.id).join("logs").exists(), "logs não deviam ter sido exportados");

        fs::remove_file(&zip_path).ok();
    }

    #[test]
    fn import_generates_unique_name_on_collision() {
        let store = temp_store();
        let created = store.create("Duplicada".to_string(), "1.21.1".to_string(), LoaderKind::Vanilla, 1024, 2048).unwrap();
        let zip_path = std::env::temp_dir().join(format!("aurora-export-test-{}.zip", Uuid::new_v4()));
        export_instance(&store, created.id, &zip_path).unwrap();

        // a instância original ("Duplicada") ainda existe no store
        // quando a importação roda, então o nome tem que virar
        // "Duplicada (2)".
        let imported = import_instance(&store, &zip_path).unwrap();
        assert_eq!(imported.name, "Duplicada (2)");

        fs::remove_file(&zip_path).ok();
    }

    #[test]
    fn import_rejects_zip_without_instance_json() {
        let store = temp_store();
        let zip_path = std::env::temp_dir().join(format!("aurora-bad-zip-{}.zip", Uuid::new_v4()));
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("random.txt", SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut zip, b"nao e uma instancia").unwrap();
        zip.finish().unwrap();

        assert!(import_instance(&store, &zip_path).is_err());

        fs::remove_file(&zip_path).ok();
    }
}
