//! Forge 相关命令

use tauri::State;

use crate::core::mods::forge::version_list::ForgeVersionInfo;
use crate::AppState;

/// 返回指定 MC 版本可用的 Forge 版本(recommended/latest 已标记)
#[tauri::command]
pub async fn list_forge_versions(
    state: State<'_, AppState>,
    mc_version: String,
) -> Result<Vec<ForgeVersionInfo>, String> {
    crate::core::mods::forge::version_list::list_forge_versions(
        &state.client,
        &mc_version,
        state.retry_times,
    )
    .await
    .map_err(|e| e.to_string())
}
