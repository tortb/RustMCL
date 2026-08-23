//! 下载版本资源命令:client.jar + libraries + natives + assets
//! 进度通过事件 "download-progress" 上报,结束通过 "download-finished"

use std::path::Path;

use tauri::{AppHandle, Emitter, State};

use crate::core::downloader::asset::{asset_index_item, asset_items, load_asset_index};
use crate::core::downloader::library::{client_download_item, library_items, native_items};
use crate::core::downloader::download_many;
use crate::core::version::manifest;
use crate::core::version::rules::{FeaturesCtx, RuleContext};
use crate::core::version::version_json::fetch_version_json;
use crate::error::RunaError;
use crate::AppState;

#[derive(Clone, serde::Serialize)]
pub struct DownloadProgressEvent {
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub file: String,
}

#[derive(Clone, serde::Serialize)]
pub struct DownloadFinishedEvent {
    pub ok: bool,
    pub error: String,
}

/// 目录布局(与官方启动器一致)
pub struct DirLayout {
    pub versions_dir: std::path::PathBuf,
    pub libraries_dir: std::path::PathBuf,
    pub assets_dir: std::path::PathBuf,
    pub game_dir: std::path::PathBuf,
}

impl DirLayout {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            versions_dir: data_dir.join("versions"),
            libraries_dir: data_dir.join("libraries"),
            assets_dir: data_dir.join("assets"),
            game_dir: data_dir.join("game"),
        }
    }
}

/// 在后台任务中执行下载,立即返回;结果通过事件通知前端
#[tauri::command]
pub fn download_version(
    app: AppHandle,
    state: State<'_, AppState>,
    mc_version: String,
) -> Result<(), String> {
    let client = state.client.clone();
    let data_dir = state.data_dir.clone();
    let retry_times = state.retry_times;
    let max_concurrent = (state.max_concurrent.max(1)) as usize;

    tauri::async_runtime::spawn(async move {
        let result =
            run_download(client, &data_dir, &mc_version, retry_times, max_concurrent, app.clone()).await;
        let _ = app.emit(
            "download-finished",
            match result {
                Ok(()) => DownloadFinishedEvent {
                    ok: true,
                    error: String::new(),
                },
                Err(e) => DownloadFinishedEvent {
                    ok: false,
                    error: e.to_string(),
                },
            },
        );
    });
    Ok(())
}

async fn run_download(
    client: reqwest::Client,
    data_dir: &Path,
    mc_version: &str,
    retry_times: u32,
    max_concurrent: usize,
    app: AppHandle,
) -> Result<(), RunaError> {
    // 1. 从清单定位版本
    let manifest_cache = data_dir.join("cache").join("version_manifest_v2.json");
    let manifest = manifest::get_manifest(&client, &manifest_cache, false, retry_times).await?;
    let info = manifest
        .versions
        .iter()
        .find(|v| v.id == mc_version)
        .ok_or_else(|| RunaError::other(format!("版本清单中不存在 {mc_version}")))?;

    // 2. version.json(带缓存)
    let vj_cache = data_dir
        .join("cache")
        .join("versions")
        .join(format!("{mc_version}.json"));
    let version = fetch_version_json(&client, &info.url, &vj_cache, retry_times).await?;

    let ctx = RuleContext::current(FeaturesCtx::default());
    let layout = DirLayout::new(data_dir);
    let version_dir = layout.versions_dir.join(&version.id);

    // 3. 阶段一:client.jar + libraries + natives + assetIndex
    let mut core_items = vec![client_download_item(&version, &version_dir)];
    core_items.extend(library_items(&version, &ctx, &layout.libraries_dir));
    core_items.extend(native_items(&version, &ctx, &layout.libraries_dir));
    core_items.push(asset_index_item(&version, &layout.assets_dir));

    let app1 = app.clone();
    download_many(
        &client,
        core_items,
        max_concurrent,
        retry_times,
        move |p| {
            let _ = app1.emit(
                "download-progress",
                DownloadProgressEvent {
                    phase: "core".into(),
                    current: p.done,
                    total: p.total,
                    file: p.file,
                },
            );
        },
    )
    .await?;

    // 4. 阶段二:assets objects
    let index_path = layout
        .assets_dir
        .join("indexes")
        .join(format!("{}.json", version.asset_index.id));
    let index = load_asset_index(&index_path)?;
    let items = asset_items(&index, &layout.assets_dir);
    let total = items.len();
    let app2 = app.clone();
    download_many(
        &client,
        items,
        max_concurrent,
        retry_times,
        move |p| {
            let _ = app2.emit(
                "download-progress",
                DownloadProgressEvent {
                    phase: "assets".into(),
                    current: p.done,
                    total,
                    file: p.file,
                },
            );
        },
    )
    .await?;

    Ok(())
}
