mod commands;
mod config;
mod core;
mod db;
mod error;

use std::path::PathBuf;
use std::sync::Mutex;

pub use error::RunaError;

/// Tauri 管理的全局状态
pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub data_dir: PathBuf,
    pub config_path: PathBuf,
    pub client: reqwest::Client,
    pub retry_times: u32,
    pub max_concurrent: u32,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = config::default_data_dir();
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("[runa] 创建数据目录失败: {e}");
        std::process::exit(1);
    }

    let config_path = data_dir.join("config.toml");
    let app_config = match config::app_config::AppConfig::load_or_create(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[runa] 初始化配置失败: {e}");
            std::process::exit(1);
        }
    };

    let db_path = data_dir.join("runa.db");
    let conn = match db::init(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[runa] 初始化数据库失败: {e}");
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
            eprintln!("[runa] 初始化 HTTP 客户端失败: {e}");
            std::process::exit(1);
        }
    };

    eprintln!(
        "[runa] 数据目录: {}, 主题: {}, 语言: {}",
        data_dir.display(),
        app_config.general.theme,
        app_config.general.language
    );

    let builder = tauri::Builder::default()
        .manage(AppState {
            db: Mutex::new(conn),
            data_dir,
            config_path,
            client,
            retry_times: app_config.download.retry_times,
            max_concurrent: app_config.download.max_concurrent,
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
            commands::config::get_app_config,
            commands::config::update_app_config,
            commands::config::detect_java,
        ]);

    if let Err(e) = builder.run(tauri::generate_context!()) {
        eprintln!("[runa] 启动失败: {e}");
        std::process::exit(1);
    }
}
