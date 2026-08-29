pub mod account;
pub mod config;
pub mod download;
pub mod instance;
pub mod launch;
pub mod loader;
pub mod mods;
pub mod version;

use serde::Serialize;
use tauri::State;

use crate::db::repository::Repository;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub data_dir: String,
}

/// 返回应用基本信息,验证前后端 invoke 通路
#[tauri::command]
pub fn get_app_info(state: State<AppState>) -> Result<AppInfo, String> {
    Ok(AppInfo {
        name: "Runa".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        data_dir: state.data_dir.display().to_string(),
    })
}

/// 数据库健康检查:返回已建好的数据表列表(验证 migration 生效)
#[tauri::command]
pub fn db_health(state: State<AppState>) -> Result<Vec<String>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut tables = Vec::new();
    for row in rows {
        tables.push(row.map_err(|e| e.to_string())?);
    }
    Ok(tables)
}

/// 占位:返回当前账号列表(后续 M3 完善)
#[tauri::command]
pub fn list_accounts(state: State<AppState>) -> Result<Vec<crate::db::schema::Account>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::list_accounts(&conn).map_err(|e| e.to_string())
}
