//! 启动版本命令:校验文件 → 解压 natives → 拼参数 → 启动进程
//! 日志逐行通过 "game-log" 事件转发,退出通过 "game-exit" 通知

use std::path::Path;

use tauri::{AppHandle, Emitter, State};

use crate::config::app_config::AppConfig;
use crate::core::account::microsoft_auth::resolve_active_account;
use crate::core::downloader::library::{client_download_item, library_items, native_items};
use crate::core::launcher::args_builder::{build_launch_command, LaunchOptions, LaunchPaths};
use crate::core::launcher::process::launch_process;
use crate::core::launcher::{extract_natives, native_plan};
use crate::core::version::manifest;
use crate::core::version::rules::{FeaturesCtx, RuleContext};
use crate::core::version::version_json::fetch_version_json;
use crate::error::RunaError;
use crate::AppState;

use super::download::DirLayout;

#[derive(Clone, serde::Serialize)]
pub struct GameLogEvent {
    pub line: String,
}

#[derive(Clone, serde::Serialize)]
pub struct GameExitEvent {
    pub code: i32,
}

/// 在后台任务中启动游戏,立即返回;日志与退出通过事件通知前端
#[tauri::command]
pub fn launch_version(
    app: AppHandle,
    state: State<'_, AppState>,
    mc_version: String,
    username: Option<String>,
) -> Result<(), String> {
    let client = state.client.clone();
    let data_dir = state.data_dir.clone();
    let config_path = state.config_path.clone();
    let retry_times = state.retry_times;
    let username = username.unwrap_or_default();

    tauri::async_runtime::spawn(async move {
        let result =
            run_launch(client, &data_dir, &config_path, &mc_version, &username, retry_times, app.clone()).await;
        match result {
            Ok(code) => {
                let _ = app.emit("game-exit", GameExitEvent { code });
            }
            Err(e) => {
                let _ = app.emit(
                    "game-log",
                    GameLogEvent {
                        line: format!("[Runa] 启动失败: {e}"),
                    },
                );
                let _ = app.emit("game-exit", GameExitEvent { code: -1 });
            }
        }
    });
    Ok(())
}

async fn run_launch(
    client: reqwest::Client,
    data_dir: &Path,
    config_path: &Path,
    mc_version: &str,
    username: &str,
    retry_times: u32,
    app: AppHandle,
) -> Result<i32, RunaError> {
    // 1. version.json(优先本地缓存)
    let manifest_cache = data_dir.join("cache").join("version_manifest_v2.json");
    let manifest = manifest::get_manifest(&client, &manifest_cache, false, retry_times).await?;
    let info = manifest
        .versions
        .iter()
        .find(|v| v.id == mc_version)
        .ok_or_else(|| RunaError::other(format!("版本清单中不存在 {mc_version}")))?;
    let vj_cache = data_dir
        .join("cache")
        .join("versions")
        .join(format!("{mc_version}.json"));
    let version = fetch_version_json(&client, &info.url, &vj_cache, retry_times).await?;

    let ctx = RuleContext::current(FeaturesCtx::default());
    let layout = DirLayout::new(data_dir);
    let version_dir = layout.versions_dir.join(&version.id);
    let natives_dir = version_dir.join("natives");

    // 2. 检查文件是否已下载
    let client_item = client_download_item(&version, &version_dir);
    if !client_item.dest.exists() {
        return Err(RunaError::other(format!(
            "缺少客户端文件 {} ,请先在下载页获取该版本",
            client_item.dest.display()
        )));
    }
    let libs = library_items(&version, &ctx, &layout.libraries_dir);
    let natives = native_items(&version, &ctx, &layout.libraries_dir);
    for item in libs.iter().chain(natives.iter()) {
        if !item.dest.exists() {
            return Err(RunaError::other(format!(
                "缺少依赖 {} ,请先在下载页获取该版本",
                item.dest.display()
            )));
        }
    }

    // 3. 解压 natives
    extract_natives(&native_plan(&version, &ctx, &layout.libraries_dir), &natives_dir)?;

    // 4. 拼装启动参数:已登录微软账号则静默续期使用,否则离线账号
    let cfg = AppConfig::load_or_create(config_path)?;
    let java_path = cfg.java_path();
    let (username, uuid, access_token) = if username.is_empty() {
        match resolve_active_account(&client).await? {
            Some((name, uid, tok)) => (name, uid, tok),
            None => (
                "Steve".into(),
                uuid::Uuid::new_v4().to_string(),
                "0".into(),
            ),
        }
    } else {
        (
            username.to_string(),
            uuid::Uuid::new_v4().to_string(),
            "0".into(),
        )
    };
    let opts = LaunchOptions {
        username,
        uuid,
        access_token,
        ..Default::default()
    };
    let paths = LaunchPaths {
        game_dir: layout.game_dir,
        assets_dir: layout.assets_dir,
        libraries_dir: layout.libraries_dir,
        version_dir: version_dir.clone(),
        natives_dir,
    };
    let cmd = build_launch_command(&version, &paths, &opts, &java_path)?;

    // 5. 启动进程,转发日志
    let app2 = app.clone();
    let code = launch_process(&cmd.java_path, &cmd.args, move |line| {
        let _ = app2.emit("game-log", GameLogEvent { line });
    })
    .await?;
    Ok(code)
}
