//! JVM 内存推荐命令:读取系统内存并给出推荐配置

use tauri::State;

use crate::core::jvm::{current_memory, is_32bit, recommend, JvmRecommendation, SystemMemory};
use crate::AppState;

/// 当前系统内存概况
#[tauri::command]
pub fn get_system_memory() -> Result<SystemMemory, String> {
    Ok(current_memory())
}

/// 按系统内存 + 可选 mod 数量返回 JVM 推荐配置(前端仅作提示,用户主动"应用"才生效)
#[tauri::command]
pub fn recommend_jvm(state: State<'_, AppState>, mod_count: Option<u32>) -> Result<JvmRecommendation, String> {
    let m = current_memory();
    // 当未传入 mod 数量时,统计当前正被管理的实例数作为粗略参考(0 表示未知)
    let count = mod_count.unwrap_or_else(|| {
        let conn = state
            .db
            .lock()
            .map_err(|_| ())
            .ok()
            .and_then(|conn| crate::db::repository::Repository::list_instances(&conn).ok());
        conn.map(|v| v.len() as u32).unwrap_or(0)
    });
    Ok(recommend(m.total_mb, m.available_mb, count, is_32bit()))
}
