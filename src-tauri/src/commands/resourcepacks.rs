//! 资源包/光影包管理(模块 4):扫描目录同步 DB、启用/禁用(重命名)、从 Modrinth 搜索。

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::core::mods::modrinth;
use crate::db::repository::Repository;
use crate::db::schema::ResourcePackEntry;
use crate::AppState;

use super::download::DownloadProgressEvent;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn instance_game_dir(
    state: &State<'_, AppState>,
    instance_id: &str,
) -> Result<std::path::PathBuf, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::get_instance(&conn, instance_id)
        .map_err(|e| e.to_string())?
        .map(|i| std::path::PathBuf::from(i.game_dir))
        .ok_or_else(|| format!("实例不存在: {instance_id}"))
}

fn dir_for_type(game_dir: &std::path::Path, type_kind: &str) -> std::path::PathBuf {
    if type_kind == "shaderpack" {
        game_dir.join("shaderpacks")
    } else {
        game_dir.join("resourcepacks")
    }
}

/// 扫描实例下资源包/光影包目录,把文件系统状态同步进 DB(新增+删除),并返回最新列表
#[tauri::command]
pub fn scan_resource_packs(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<ResourcePackEntry>, String> {
    let game_dir = instance_game_dir(&state, &instance_id)?;
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;

    // 已有的 DB 条目(用于比对删除)
    let existing =
        Repository::list_resource_packs(&conn, &instance_id).map_err(|e| e.to_string())?;

    let mut seen_ids = std::collections::HashSet::new();
    for type_kind in ["resourcepack", "shaderpack"] {
        let dir = dir_for_type(&game_dir, type_kind);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            // 被禁用时文件名为 xxx.disabled
            let (base, enabled) = match name.strip_suffix(".disabled") {
                Some(b) => (b.to_string(), false),
                None => (name.clone(), true),
            };
            let id = format!("{instance_id}:{type_kind}:{base}");
            seen_ids.insert(id.clone());
            let p = ResourcePackEntry {
                id,
                instance_id: instance_id.clone(),
                type_kind: type_kind.to_string(),
                file_name: base,
                enabled,
                created_at: now_secs(),
            };
            Repository::upsert_resource_pack(&conn, &p).map_err(|e| e.to_string())?;
        }
    }

    // 删除磁盘上已不存在的记录
    for old in &existing {
        if !seen_ids.contains(&old.id) {
            let _ = Repository::delete_resource_pack(&conn, &old.id);
        }
    }

    Repository::list_resource_packs(&conn, &instance_id).map_err(|e| e.to_string())
}

/// 启用/禁用资源包:重命名 xxx.disabled 后缀,并更新 DB
#[tauri::command]
pub fn set_resource_pack_enabled(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let game_dir = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let p = Repository::get_resource_pack(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("资源包不存在: {id}"))?;
        let game = Repository::get_instance(&conn, &p.instance_id)
            .map_err(|e| e.to_string())?
            .map(|i| i.game_dir)
            .ok_or("实例不存在")?;
        (game, p.type_kind.clone(), p.file_name.clone())
    };
    let (game_dir, type_kind, file_name) = game_dir;
    let dir = dir_for_type(std::path::Path::new(&game_dir), &type_kind);
    let active = dir.join(&file_name);
    let disabled = dir.join(format!("{file_name}.disabled"));

    if enabled {
        if disabled.exists() {
            std::fs::rename(&disabled, &active).map_err(|e| format!("启用失败: {e}"))?;
        }
    } else if active.exists() {
        std::fs::rename(&active, &disabled).map_err(|e| format!("禁用失败: {e}"))?;
    }

    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::set_resource_pack_enabled(&conn, &id, enabled).map_err(|e| e.to_string())
}

/// 删除资源包文件 + DB 记录
#[tauri::command]
pub fn remove_resource_pack(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let game_dir = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let p = Repository::get_resource_pack(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("资源包不存在: {id}"))?;
        let game = Repository::get_instance(&conn, &p.instance_id)
            .map_err(|e| e.to_string())?
            .map(|i| i.game_dir)
            .ok_or("实例不存在")?;
        (game, p.type_kind.clone(), p.file_name.clone())
    };
    let (game_dir, type_kind, file_name) = game_dir;
    let dir = dir_for_type(std::path::Path::new(&game_dir), &type_kind);
    for candidate in [
        dir.join(&file_name),
        dir.join(format!("{file_name}.disabled")),
    ] {
        if candidate.exists() {
            let _ = std::fs::remove_file(&candidate);
        }
    }

    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::delete_resource_pack(&conn, &id).map_err(|e| e.to_string())
}

/// 从 Modrinth 搜索资源包/光影包(type: resourcepack | shader)
#[tauri::command]
pub async fn search_resource_packs(
    state: State<'_, AppState>,
    query: String,
    pack_type: String,
) -> Result<Vec<modrinth::ModrinthHit>, String> {
    modrinth::search_by_type(&state.client, &query, &pack_type, 12, state.retry_times)
        .await
        .map_err(|e| e.to_string())
}

/// 获取某资源包/光影包项目与指定实例兼容的版本列表
#[tauri::command]
pub async fn get_resource_pack_versions(
    state: State<'_, AppState>,
    project_id: String,
    instance_id: String,
) -> Result<Vec<modrinth::ModrinthVersion>, String> {
    let mc_version = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let inst = Repository::get_instance(&conn, &instance_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("实例不存在: {instance_id}"))?;
        (
            inst.mc_version,
            inst.loader.as_deref().unwrap_or("vanilla").to_string(),
        )
    };
    let (mc_version, loader) = mc_version;
    modrinth::compatible_versions(
        &state.client,
        &project_id,
        &mc_version,
        &loader,
        state.retry_times,
    )
    .await
    .map_err(|e| e.to_string())
}

/// 从 Modrinth 版本安装资源包/光影包到实例目录(resourcepacks/ 或 shaderpacks/)并记录 DB
#[tauri::command]
pub async fn install_resource_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    version_id: String,
    pack_type: String,
) -> Result<ResourcePackEntry, String> {
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

    let type_kind = if pack_type == "shaderpack" {
        "shaderpack"
    } else {
        "resourcepack"
    };
    let game_dir = std::path::Path::new(&inst.game_dir);
    let dir = dir_for_type(game_dir, type_kind);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file_name = crate::core::mods::install_version(
        &state.client,
        &state.mirror(),
        &version,
        &dir,
        state.retry_times,
    )
    .await
    .map_err(|e| e.to_string())?;

    // 与 scan_resource_packs 生成的 id 同构,保证幂等且不被下次扫描删除
    let entry = ResourcePackEntry {
        id: format!("{instance_id}:{type_kind}:{file_name}"),
        instance_id: inst.id.clone(),
        type_kind: type_kind.to_string(),
        file_name,
        enabled: true,
        created_at: now_secs(),
    };
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::upsert_resource_pack(&conn, &entry).map_err(|e| e.to_string())?;

    let _ = app.emit(
        "mod-install",
        DownloadProgressEvent {
            phase: "pack".into(),
            current: 1,
            total: 1,
            file: entry.file_name.clone(),
        },
    );
    Ok(entry)
}

/// 光影依赖检测:是否装了 Iris/OptiFine
#[derive(Debug, Serialize)]
pub struct ShaderSupportInfo {
    pub supported: bool,
    pub message: String,
}

/// 检测实例是否已安装光影加载器(Iris 或 OptiFine),未装时给出可执行的安装建议
#[tauri::command]
pub fn check_shader_support(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<ShaderSupportInfo, String> {
    let game_dir = instance_game_dir(&state, &instance_id)?;
    let mods_dir = game_dir.join("mods");
    let mut supported = false;
    if let Ok(entries) = std::fs::read_dir(&mods_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.ends_with(".jar") && (name.contains("iris") || name.contains("optifine")) {
                supported = true;
                break;
            }
        }
    }
    let message = if supported {
        "已检测到 Iris/OptiFine,可正常使用光影".to_string()
    } else {
        "未检测到 Iris 或 OptiFine。光影需要加载器:Forge 版用 OptiFine,Fabric/Quilt 版用 Iris。请先安装对应加载器后再启用光影。".to_string()
    };
    Ok(ShaderSupportInfo { supported, message })
}
