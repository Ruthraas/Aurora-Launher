mod commands;
mod core;
mod error;

use core::jobs::JobRegistry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(JobRegistry::default())
        .invoke_handler(tauri::generate_handler![
            commands::app_info::get_app_info,
            commands::accounts::list_accounts,
            commands::accounts::add_offline_account,
            commands::accounts::remove_account,
            commands::accounts::set_active_account,
            commands::accounts::logout,
            commands::accounts::import_account_skin_by_username,
            commands::accounts::upload_account_skin,
            commands::accounts::upload_account_cape,
            commands::accounts::clear_account_skin,
            commands::accounts::clear_account_cape,
            commands::accounts::get_account_texture,
            commands::minecraft::fetch_version_manifest,
            commands::minecraft::fetch_version_details,
            commands::java::ensure_java_runtime,
            commands::instances::list_instances,
            commands::instances::create_instance,
            commands::instances::retry_instance_install,
            commands::instances::fetch_fabric_loader_versions,
            commands::instances::fetch_neoforge_versions,
            commands::instances::fetch_forge_versions,
            commands::instances::fetch_quilt_loader_versions,
            commands::modrinth::search_modrinth,
            commands::curseforge::search_curseforge,
            commands::settings::get_settings,
            commands::settings::set_curseforge_api_key,
            commands::instances::delete_instance,
            commands::instances::launch_instance,
            commands::instances::update_instance_settings,
            commands::instances::open_instance_folder,
            commands::instances::list_instance_logs,
            commands::instances::read_instance_log,
            commands::instances::export_instance,
            commands::instances::import_instance_from_bytes,
            commands::instances::list_instance_worlds,
            commands::instances::delete_instance_world,
            commands::instances::open_instance_world_folder,
            commands::instances::get_world_icon,
            commands::mods::get_mod_compatibility,
            commands::mods::install_mod,
            commands::mods::cancel_job,
            commands::mods::list_instance_mods,
            commands::mods::remove_instance_mod,
            commands::mods::get_recommended_optimizations,
            commands::mods::install_optimizations,
            commands::mods::optimize_instance,
            commands::modpack::get_modpack_preview,
            commands::modpack::install_modpack,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
