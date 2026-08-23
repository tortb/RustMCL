use tauri::State;

use crate::core::version::manifest::{self, VersionFilter, VersionInfo};
use crate::AppState;

/// 拉取(或读缓存)版本清单,返回过滤后的版本列表
#[tauri::command]
pub async fn list_versions(
    state: State<'_, AppState>,
    filter: String,
    force_refresh: bool,
) -> Result<Vec<VersionInfo>, String> {
    let f = VersionFilter::from_str(&filter);
    let cache_path = state
        .data_dir
        .join("cache")
        .join("version_manifest_v2.json");
    let manifest = manifest::get_manifest(
        &state.client,
        &cache_path,
        force_refresh,
        state.retry_times,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(manifest::list_versions(&manifest, f)
        .into_iter()
        .cloned()
        .collect())
}
