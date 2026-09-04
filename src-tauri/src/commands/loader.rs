//! 加载器安装命令:
//! - install_loader:后台解析 Fabric/Quilt profile → 下载全部资源(幂等)
//! - get_latest_loader_version:查询指定 MC 版本可用的最新加载器版本
//! 进度复用 "download-progress" 事件,结束发 "loader-install-finished"

use tauri::{AppHandle, Emitter, State};

use crate::core::loader;
use crate::error::RmclError;
use crate::AppState;

use super::download::{run_download_for_version, DownloadFinishedEvent};

/// 查询最新加载器版本(用于创建实例时自动填充 loader_version)
#[tauri::command]
pub async fn get_latest_loader_version(
    state: State<'_, AppState>,
    mc_version: String,
    loader: String,
) -> Result<String, String> {
    loader::latest_loader_version(
        &state.client,
        &state.mirror(),
        &loader,
        &mc_version,
        state.retry_times,
    )
    .await
    .map_err(|e| e.to_string())
}

/// 后台安装加载器:解析合并版本 → 下载 client + libraries + natives + assets
#[tauri::command]
pub fn install_loader(
    app: AppHandle,
    state: State<'_, AppState>,
    mc_version: String,
    loader: String,
    loader_version: String,
) -> Result<(), String> {
    let client = state.client.clone();
    let data_dir = state.data_dir.clone();
    let retry_times = state.retry_times;
    let max_concurrent = (state.max_concurrent.max(1)) as usize;
    let mirror = state.mirror();

    tauri::async_runtime::spawn(async move {
        let result = (async {
            let version = loader::resolve_version(
                &client,
                &mirror,
                &data_dir,
                &mc_version,
                &loader,
                &loader_version,
                retry_times,
            )
            .await?;
            run_download_for_version(
                &client,
                &data_dir,
                &version,
                retry_times,
                max_concurrent,
                app.clone(),
                &mirror,
                None,
            )
            .await
        })
        .await;

        let _ = app.emit(
            "loader-install-finished",
            match result {
                Ok(()) => DownloadFinishedEvent {
                    ok: true,
                    error: String::new(),
                    cancelled: false,
                },
                Err(e) => DownloadFinishedEvent {
                    ok: false,
                    error: e.to_string(),
                    cancelled: matches!(e, RmclError::Cancelled),
                },
            },
        );
    });
    Ok(())
}
