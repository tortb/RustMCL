//! 下载版本资源命令:client.jar + libraries + natives + assets
//! 进度通过事件 "download-progress" 上报,结束通过 "download-finished"

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::core::downloader::asset::{asset_index_item, asset_items, load_asset_index};
use crate::core::downloader::library::{client_download_item, library_items, native_items};
use crate::core::downloader::download_many;
use crate::core::loader;
use crate::core::mirror::Mirror;
use crate::core::version::rules::{FeaturesCtx, RuleContext};
use crate::core::version::version_json::VersionJson;
use crate::commands::launch::GameLogEvent;
use crate::db::repository::Repository;
use crate::error::RmclError;
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
    /// 是否为用户主动取消(区别于失败);创建实例流程据此清理
    #[serde(default)]
    pub cancelled: bool,
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
    let mirror = state.mirror();

    tauri::async_runtime::spawn(async move {
        let result = run_download(
            client,
            &data_dir,
            &mc_version,
            None,
            None,
            retry_times,
            max_concurrent,
            app.clone(),
            &mirror,
            None,
        )
        .await;
        let _ = app.emit(
            "download-finished",
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

/// 创建实例时预下载该版本的共享原版资源(client.jar + libraries + natives + assets)。
/// 加载器(Forge/Fabric/Quilt)仍由既有后台安装流程处理,这里是资源大头。
/// 进度复用 "download-progress",结束发 "download-finished"(含 cancelled 标志)。
#[tauri::command]
pub fn prepare_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<(), String> {
    // 读实例配置,取 mc_version
    let mc_version = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_instance(&conn, &instance_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("实例不存在: {instance_id}"))?
            .mc_version
    };

    let client = state.client.clone();
    let data_dir = state.data_dir.clone();
    let retry_times = state.retry_times;
    let max_concurrent = (state.max_concurrent.max(1)) as usize;
    let mirror = state.mirror();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut map = state
            .cancel_tokens
            .lock()
            .map_err(|e| format!("取消令牌锁获取失败: {e}"))?;
        map.insert(instance_id.clone(), cancel.clone());
    }

    tauri::async_runtime::spawn(async move {
        let result = run_download(
            client,
            &data_dir,
            &mc_version,
            None,
            None,
            retry_times,
            max_concurrent,
            app.clone(),
            &mirror,
            Some(cancel),
        )
        .await;
        let _ = app.emit(
            "download-finished",
            match result {
                Ok(()) => DownloadFinishedEvent {
                    ok: true,
                    error: String::new(),
                    cancelled: false,
                },
                Err(e) if matches!(e, RmclError::Cancelled) => DownloadFinishedEvent {
                    ok: false,
                    error: "已取消".into(),
                    cancelled: true,
                },
                Err(e) => DownloadFinishedEvent {
                    ok: false,
                    error: e.to_string(),
                    cancelled: false,
                },
            },
        );
    });
    Ok(())
}

/// 取消创建实例时的资源下载:置位取消令牌并清理残留的 .part 临时文件
#[tauri::command]
pub fn cancel_instance_download(state: State<'_, AppState>, instance_id: String) -> Result<(), String> {
    if let Some(flag) = state
        .cancel_tokens
        .lock()
        .map_err(|e| format!("取消令牌锁获取失败: {e}"))?
        .get(&instance_id)
        .cloned()
    {
        flag.store(true, Ordering::SeqCst);
    }
    cleanup_part_files(&state.data_dir);
    Ok(())
}

/// 递归删除版本/库/资源目录下的 .part 临时文件(取消创建流程时清理残留)
fn cleanup_part_files(data_dir: &Path) {
    for sub in ["versions", "libraries", "assets"] {
        remove_parts_recursive(&data_dir.join(sub));
    }
}

fn remove_parts_recursive(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            remove_parts_recursive(&path);
        } else if path.extension().map(|e| e == "part").unwrap_or(false) {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// 下载指定版本资源;loader 非 vanilla 时先解析并合并加载器 profile。
/// `cancel` 提供取消令牌(为 None 时不取消)。
pub(crate) async fn run_download(
    client: reqwest::Client,
    data_dir: &Path,
    mc_version: &str,
    loader: Option<&str>,
    loader_version: Option<&str>,
    retry_times: u32,
    max_concurrent: usize,
    app: AppHandle,
    mirror: &Mirror,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<(), RmclError> {
    let version = loader::resolve_version(
        &client,
        mirror,
        data_dir,
        mc_version,
        loader.unwrap_or("vanilla"),
        loader_version.unwrap_or(""),
        retry_times,
    )
    .await?;
    run_download_for_version(&client, data_dir, &version, retry_times, max_concurrent, app, mirror, cancel).await
}

/// 按已解析的 version 下载 client.jar + libraries + natives + assets(幂等)
pub(crate) async fn run_download_for_version(
    client: &reqwest::Client,
    data_dir: &Path,
    version: &VersionJson,
    retry_times: u32,
    max_concurrent: usize,
    app: AppHandle,
    mirror: &Mirror,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<(), RmclError> {
    let ctx = RuleContext::current(FeaturesCtx::default());
    let layout = DirLayout::new(data_dir);
    let version_dir = layout.versions_dir.join(&version.id);

    // 3. 阶段一:client.jar + libraries + natives + assetIndex
    let mut core_items = vec![client_download_item(&version, &version_dir)];
    core_items.extend(library_items(&version, &ctx, &layout.libraries_dir));
    core_items.extend(native_items(&version, &ctx, &layout.libraries_dir));
    core_items.push(asset_index_item(&version, &layout.assets_dir));

    let app1 = app.clone();
    let cancel_core = cancel.clone();
    let core_stats = download_many(
        &client,
        mirror,
        core_items,
        max_concurrent,
        retry_times,
        cancel_core,
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
    let _ = app.emit(
        "game-log",
        GameLogEvent {
            line: format!(
                "[RustMCL] 核心资源:命中缓存跳过 {} 个,实际下载 {} 个",
                core_stats.cached, core_stats.downloaded
            ),
        },
    );

    // 4. 阶段二:assets objects
    let index_path = layout
        .assets_dir
        .join("indexes")
        .join(format!("{}.json", version.asset_index.id));
    let index = load_asset_index(&index_path)?;
    let items = asset_items(&index, &layout.assets_dir);
    let total = items.len();
    let app2 = app.clone();
    let cancel_assets = cancel.clone();
    let asset_stats = download_many(
        &client,
        mirror,
        items,
        max_concurrent,
        retry_times,
        cancel_assets,
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
    let _ = app.emit(
        "game-log",
        GameLogEvent {
            line: format!(
                "[RustMCL] 资源文件:命中缓存跳过 {} 个,实际下载 {} 个",
                asset_stats.cached, asset_stats.downloaded
            ),
        },
    );

    Ok(())
}
