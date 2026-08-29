mod commands;
mod config;
mod core;
mod db;
mod error;

use std::path::PathBuf;
use std::sync::Mutex;

pub use error::RmclError;

/// Tauri 管理的全局状态
pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub data_dir: PathBuf,
    pub config_path: PathBuf,
    pub client: reqwest::Client,
    pub retry_times: u32,
    pub max_concurrent: u32,
    /// 当前生效的下载镜像(可在设置页切换)
    pub mirror: Mutex<crate::core::mirror::Mirror>,
}

impl AppState {
    /// 当前镜像的克隆(供下载管线使用)
    pub fn mirror(&self) -> crate::core::mirror::Mirror {
        self.mirror
            .lock()
            .map(|m| m.clone())
            .unwrap_or_else(|_| crate::core::mirror::Mirror::from_config("official", None))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 旧数据目录兼容:首次改名后把 ~/.runa 迁移到 ~/.rustmcl(仅当新目录不存在时)
    config::migrate_legacy_data_dir();

    let data_dir = config::default_data_dir();
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("[rmcl] 创建数据目录失败: {e}");
        std::process::exit(1);
    }

    let config_path = data_dir.join("config.toml");
    let app_config = match config::app_config::AppConfig::load_or_create(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[rmcl] 初始化配置失败: {e}");
            std::process::exit(1);
        }
    };

    let db_path = data_dir.join("rmcl.db");
    let conn = match db::init(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[rmcl] 初始化数据库失败: {e}");
            std::process::exit(1);
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[rmcl] 初始化 HTTP 客户端失败: {e}");
            std::process::exit(1);
        }
    };

    eprintln!(
        "[rmcl] 数据目录: {}, 主题: {}, 语言: {}",
        data_dir.display(),
        app_config.general.theme,
        app_config.general.language
    );

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            db: Mutex::new(conn),
            data_dir,
            config_path,
            client,
            retry_times: app_config.download.retry_times,
            max_concurrent: app_config.download.max_concurrent,
            mirror: Mutex::new(crate::core::mirror::Mirror::from_config(
                &app_config.download.mirror,
                app_config.download.mirror_custom_base.as_deref(),
            )),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::db_health,
            commands::list_accounts,
            commands::account::start_microsoft_login,
            commands::account::cancel_microsoft_login,
            commands::account::get_active_account,
            commands::account::logout_account,
            commands::version::list_versions,
            commands::download::download_version,
            commands::launch::launch_version,
            commands::launch::launch_instance,
            commands::instance::create_instance,
            commands::instance::list_instances,
            commands::instance::get_instance,
            commands::instance::update_instance,
            commands::instance::delete_instance,
            commands::loader::install_loader,
            commands::loader::get_latest_loader_version,
            commands::mods::search_mods,
            commands::mods::get_mod_versions,
            commands::mods::install_mod,
            commands::mods::list_instance_mods,
            commands::mods::set_mod_enabled,
            commands::mods::delete_mod,
            commands::mods::check_mod_dependencies,
            commands::mods::search_curseforge_mods,
            commands::mods::get_curseforge_file_versions,
            commands::mods::install_curseforge_file,
            commands::config::get_app_config,
            commands::config::update_app_config,
            commands::config::detect_java,
            commands::mirror::list_mirrors,
            commands::mirror::test_mirror_speed,
            commands::mirror::test_all_mirror_speed,
            commands::mirror::set_mirror,
            commands::diagnostics::analyze_crash_report,
            commands::diagnostics::list_crash_reports,
            commands::jvm::get_system_memory,
            commands::jvm::recommend_jvm,
            commands::modpack::import_modpack,
            commands::modpack::export_modpack,
            commands::servers::add_server,
            commands::servers::remove_server,
            commands::servers::list_servers,
            commands::servers::update_server,
            commands::servers::ping_server,
            commands::servers::join_server,
            commands::servers::import_servers,
            commands::resourcepacks::scan_resource_packs,
            commands::resourcepacks::set_resource_pack_enabled,
            commands::resourcepacks::remove_resource_pack,
            commands::resourcepacks::search_resource_packs,
            commands::resourcepacks::check_shader_support,
            commands::saves::list_saves,
            commands::saves::backup_save,
            commands::saves::list_backups,
            commands::saves::restore_backup,
            commands::saves::delete_save,
            commands::saves::list_screenshots,
            commands::saves::delete_screenshot,
            commands::saves::get_screenshot_image,
            commands::update::check_for_update,
            commands::update::install_update,
            commands::forge::list_forge_versions,
            commands::forge::install_forge,
            commands::skins::list_skins,
            commands::skins::import_skin,
            commands::skins::remove_skin,
            commands::skins::get_skin_image,
            commands::skins::upload_skin,
            commands::skins::get_offline_skin,
            commands::skins::set_offline_skin,
        ]);

    if let Err(e) = builder.run(tauri::generate_context!()) {
        eprintln!("[rmcl] 启动失败: {e}");
        std::process::exit(1);
    }
}
