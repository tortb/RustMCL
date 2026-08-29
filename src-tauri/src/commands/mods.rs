//! Mod 管理命令:搜索/浏览 Modrinth、安装到实例、启用/禁用/删除
//! 安装为异步下载(幂等),进度通过事件 "mod-install" 上报

use tauri::{AppHandle, Emitter, State};

use crate::config::app_config::AppConfig;
use crate::core::downloader::{download_one, DownloadItem};
use crate::core::mods::{self, curseforge, deps, modrinth};
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
    let file_name = mods::install_version(
        &state.client,
        &state.mirror(),
        &version,
        &mods_dir,
        state.retry_times,
    )
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

/// 检测安装某版本时的依赖缺失/冲突(建议性,不阻断安装)
#[tauri::command]
pub async fn check_mod_dependencies(
    state: State<'_, AppState>,
    instance_id: String,
    version_id: String,
) -> Result<deps::DepCheckResult, String> {
    let installed: Vec<(String, String)> = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::list_mods(&conn, &instance_id)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|m| (m.project_id.unwrap_or_default(), m.version_id.unwrap_or_default()))
            .filter(|(p, _)| !p.is_empty())
            .collect()
    };
    let version = modrinth::fetch_version(&state.client, &version_id, state.retry_times)
        .await
        .map_err(|e| e.to_string())?;
    Ok(deps::check(&version, &installed))
}

/// 读取 CurseForge API Key(未配置则报错引导)
fn curseforge_key(state: &State<'_, AppState>) -> Result<String, String> {
    AppConfig::load_or_create(&state.config_path)
        .map_err(|e| e.to_string())?
        .curseforge_api_key
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| "未配置 CurseForge API Key,请先在设置页「网络」补充".to_string())
}

/// 搜索 CurseForge 项目(mod 浏览页来源筛选)
#[tauri::command]
pub async fn search_curseforge_mods(
    state: State<'_, AppState>,
    query: String,
    mc_version: String,
    loader: String,
    limit: Option<u32>,
) -> Result<Vec<curseforge::CurseForgeHit>, String> {
    let key = curseforge_key(&state)?;
    curseforge::search(&state.client, &key, &query, &mc_version, &loader, limit.unwrap_or(16))
        .await
        .map_err(|e| e.to_string())
}

/// 获取某 CurseForge mod 与当前实例兼容的文件列表
#[tauri::command]
pub async fn get_curseforge_file_versions(
    state: State<'_, AppState>,
    project_id: String,
    instance_id: String,
) -> Result<Vec<curseforge::CurseForgeFile>, String> {
    let key = curseforge_key(&state)?;
    let inst = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_instance(&conn, &instance_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("实例不存在: {instance_id}"))?
    };
    curseforge::file_versions(
        &state.client,
        &key,
        &project_id,
        &inst.mc_version,
        inst.loader.as_deref().unwrap_or("vanilla"),
    )
    .await
    .map_err(|e| e.to_string())
}

/// 下载并安装一个 CurseForge 文件到实例(记录 DB, source = curseforge)
#[tauri::command]
pub async fn install_curseforge_file(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
    file: curseforge::CurseForgeFile,
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
    let mods_dir = std::path::Path::new(&inst.game_dir).join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    let dest = mods_dir.join(&file.filename);
    let item = DownloadItem {
        url: file.url.clone(),
        sha1: file.sha1.clone(),
        size: file.size,
        dest,
    };
    download_one(&state.client, &state.mirror(), &item, state.retry_times)
        .await
        .map_err(|e| e.to_string())?;

    let entry = ModEntry {
        id: uuid::Uuid::new_v4().simple().to_string(),
        instance_id,
        file_name: file.filename.clone(),
        source: Some("curseforge".into()),
        project_id: Some(project_id),
        version_id: Some(file.file_id.to_string()),
        enabled: true,
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::insert_mod(&conn, &entry).map_err(|e| e.to_string())?;

    let _ = app.emit(
        "mod-install",
        super::download::DownloadProgressEvent {
            phase: "mod".into(),
            current: 1,
            total: 1,
            file: entry.file_name.clone(),
        },
    );
    Ok(entry)
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
