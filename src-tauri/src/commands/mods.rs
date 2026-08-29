//! Mod 管理命令:搜索/浏览 Modrinth、安装到实例、启用/禁用/删除
//! 安装为异步下载(幂等),进度通过事件 "mod-install" 上报

use tauri::{AppHandle, Emitter, State};

use crate::core::mods::{self, modrinth};
use crate::db::repository::Repository;
use crate::db::schema::ModEntry;
use crate::AppState;

use super::download::DownloadProgressEvent;

/// 搜索 Modrinth 项目
#[tauri::command]
pub async fn search_mods(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<modrinth::ModrinthHit>, String> {
    modrinth::search(&state.client, &query, limit.unwrap_or(16), state.retry_times)
        .await
        .map_err(|e| e.to_string())
}

/// 获取某项目与指定实例兼容的版本列表
#[tauri::command]
pub async fn get_mod_versions(
    state: State<'_, AppState>,
    project_id: String,
    instance_id: String,
) -> Result<Vec<modrinth::ModrinthVersion>, String> {
    let inst = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_instance(&conn, &instance_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("实例不存在: {instance_id}"))?
    };
    modrinth::compatible_versions(
        &state.client,
        &project_id,
        &inst.mc_version,
        inst.loader.as_deref().unwrap_or("vanilla"),
        state.retry_times,
    )
    .await
    .map_err(|e| e.to_string())
}

/// 安装 mod 到实例 mods 目录并记录 DB(幂等)
#[tauri::command]
pub async fn install_mod(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    version_id: String,
) -> Result<ModEntry, String> {
    let inst = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_instance(&conn, &instance_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("实例不存在: {instance_id}"))?
    };
    let version = modrinth::fetch_version(&state.client, &version_id, state.retry_times)
        .await
        .map_err(|e| e.to_string())?;

    let mods_dir = std::path::Path::new(&inst.game_dir).join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    let file_name = mods::install_version(&state.client, &version, &mods_dir, state.retry_times)
        .await
        .map_err(|e| e.to_string())?;

    let entry = ModEntry {
        id: uuid::Uuid::new_v4().simple().to_string(),
        instance_id: inst.id.clone(),
        file_name,
        source: Some("modrinth".into()),
        project_id: Some(version.project_id.clone()),
        version_id: Some(version.id),
        enabled: true,
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::insert_mod(&conn, &entry).map_err(|e| e.to_string())?;

    // 汇报进度(单个文件,直接标记完成)
    let _ = app.emit(
        "mod-install",
        DownloadProgressEvent {
            phase: "mod".into(),
            current: 1,
            total: 1,
            file: entry.file_name.clone(),
        },
    );
    Ok(entry)
}

/// 列出实例已安装的 mod
#[tauri::command]
pub fn list_instance_mods(state: State<'_, AppState>, instance_id: String) -> Result<Vec<ModEntry>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::list_mods(&conn, &instance_id).map_err(|e| e.to_string())
}

/// 启用/禁用 mod(仅 DB 记录;实际加载由启动时的 mods 目录决定)
#[tauri::command]
pub fn set_mod_enabled(state: State<'_, AppState>, id: String, enabled: bool) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::set_mod_enabled(&conn, &id, enabled).map_err(|e| e.to_string())
}

/// 删除 mod:DB 记录 + 实例 mods 目录下的文件
#[tauri::command]
pub fn delete_mod(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let entry = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_mod(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("mod 不存在: {id}"))?
    };
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::delete_mod(&conn, &id).map_err(|e| e.to_string())?;
    }
    // 删除文件(尽力而为)
    let inst = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_instance(&conn, &entry.instance_id)
            .map_err(|e| e.to_string())?
    };
    if let Some(inst) = inst {
        let file = std::path::Path::new(&inst.game_dir)
            .join("mods")
            .join(&entry.file_name);
        if file.exists() {
            let _ = std::fs::remove_file(file);
        }
    }
    Ok(())
}
