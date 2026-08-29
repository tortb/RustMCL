//! 服务器列表命令:增删查 + ping + 一键加入
//! 一键加入复用启动器逻辑,在游戏参数末尾追加 --server/--port。

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, State};

use crate::core::server_ping;
use crate::db::repository::Repository;
use crate::db::schema::{ServerEntry, ServerStatus};
use crate::AppState;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 添加服务器
#[tauri::command]
pub fn add_server(
    state: State<'_, AppState>,
    name: String,
    address: String,
    port: u16,
    favorite: Option<bool>,
) -> Result<ServerEntry, String> {
    if address.trim().is_empty() {
        return Err("服务器地址不能为空".into());
    }
    if port == 0 {
        return Err("端口非法".into());
    }
    let id = uuid::Uuid::new_v4().simple().to_string();
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    // 新服务器排到列表末尾
    let sort_order: i64 = conn
        .query_row("SELECT COUNT(*) FROM servers", [], |r| r.get(0))
        .unwrap_or(0);
    let entry = ServerEntry {
        id: id.clone(),
        name: name.trim().to_string(),
        address: address.trim().to_string(),
        port,
        is_favorite: favorite.unwrap_or(false),
        icon_base64: None,
        last_ping_ms: None,
        sort_order,
        created_at: now_secs(),
    };
    Repository::insert_server(&conn, &entry).map_err(|e| e.to_string())?;
    Ok(entry)
}

#[tauri::command]
pub fn remove_server(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::delete_server(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_servers(state: State<'_, AppState>) -> Result<Vec<ServerEntry>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::list_servers(&conn).map_err(|e| e.to_string())
}

/// 从 Minecraft 原生 servers.dat 批量导入服务器列表
#[tauri::command]
pub fn import_servers(
    state: State<'_, AppState>,
    dat_path: String,
) -> Result<Vec<crate::core::servers_import::ImportedServer>, String> {
    let raw = std::fs::read(&dat_path).map_err(|e| format!("读取 servers.dat 失败: {e}"))?;
    let servers = crate::core::servers_import::parse_servers(&raw).map_err(|e| e.to_string())?;
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let base_order: i64 = conn
        .query_row("SELECT COUNT(*) FROM servers", [], |r| r.get(0))
        .unwrap_or(0);
    for (i, s) in servers.iter().enumerate() {
        if s.address.trim().is_empty() {
            continue;
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        let entry = ServerEntry {
            id,
            name: s.name.clone(),
            address: s.address.trim().to_string(),
            port: s.port,
            is_favorite: false,
            icon_base64: None,
            last_ping_ms: None,
            sort_order: base_order + i as i64,
            created_at: now_secs(),
        };
        Repository::insert_server(&conn, &entry).map_err(|e| e.to_string())?;
    }
    Ok(servers)
}

/// 更新服务器(名称 / 收藏 / 排序)
#[tauri::command]
pub fn update_server(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    favorite: Option<bool>,
    sort_order: Option<i64>,
) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::update_server(&conn, &id, name.as_deref(), favorite, sort_order)
        .map_err(|e| e.to_string())
}

/// 对已保存的服务器做 ping,并把延迟写回 DB;不可达返回错误(前端展示离线态)
#[tauri::command]
pub async fn ping_server(state: State<'_, AppState>, id: String) -> Result<ServerStatus, String> {
    let server = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_server(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("服务器不存在: {id}"))?
    };
    match server_ping::ping(&server.address, server.port).await {
        Ok(mut st) => {
            st.id = id.clone();
            let conn = state
                .db
                .lock()
                .map_err(|e| format!("数据库锁获取失败: {e}"))?;
            let _ = Repository::set_server_ping(&conn, &id, st.latency_ms as i64);
            Ok(st)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// 一键加入服务器:用指定实例启动,并在游戏参数末尾追加 --server/--port
#[tauri::command]
pub fn join_server(
    app: AppHandle,
    state: State<'_, AppState>,
    server_id: String,
    instance_id: String,
) -> Result<(), String> {
    let server = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_server(&conn, &server_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("服务器不存在: {server_id}"))?
    };
    let extra = vec![
        "--server".to_string(),
        server.address.clone(),
        "--port".to_string(),
        server.port.to_string(),
    ];
    crate::commands::launch::spawn_instance_launch(app, state, instance_id, extra)
}
