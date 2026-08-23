//! 启动命令:校验文件 → 解压 natives → 拼参数 → 启动进程
//! 日志逐行通过 "game-log" 事件转发,退出通过 "game-exit" 通知
//! 支持两种入口:launch_version(按版本)与 launch_instance(按实例,自动补资源)

use std::path::Path;

use tauri::{AppHandle, Emitter, State};

use crate::config::app_config::AppConfig;
use crate::config::instance_config::InstanceConfig;
use crate::core::account::microsoft_auth::resolve_active_account;
use crate::core::downloader::library::{client_download_item, library_items, native_items};
use crate::core::launcher::args_builder::{build_launch_command, LaunchOptions, LaunchPaths};
use crate::core::launcher::process::launch_process;
use crate::core::launcher::{extract_natives, native_plan};
use crate::core::version::manifest;
use crate::core::version::rules::{FeaturesCtx, RuleContext};
use crate::core::version::version_json::fetch_version_json;
use crate::db::repository::Repository;
use crate::error::RunaError;
use crate::AppState;

use super::download::{run_download, DirLayout};

#[derive(Clone, serde::Serialize)]
pub struct GameLogEvent {
    pub line: String,
}

#[derive(Clone, serde::Serialize)]
pub struct GameExitEvent {
    pub code: i32,
}

/// 按版本启动(下载页入口):username 为空时自动使用已登录账号
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
    let opts = LaunchOptions {
        username: username.unwrap_or_default(),
        ..Default::default()
    };

    tauri::async_runtime::spawn(async move {
        let result =
            run_launch(client, &data_dir, &config_path, &mc_version, opts, retry_times, app.clone()).await;
        emit_launch_result(&app, result);
    });
    Ok(())
}

/// 按实例启动(实例页入口):自动补齐缺失资源,参数取自 instance.toml
#[tauri::command]
pub fn launch_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<(), String> {
    let client = state.client.clone();
    let data_dir = state.data_dir.clone();
    let config_path = state.config_path.clone();
    let retry_times = state.retry_times;
    let max_concurrent = (state.max_concurrent.max(1)) as usize;

    // 读 DB 实例 + 实例配置(同步操作,在 spawn 前完成)
    let inst = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_instance(&conn, &instance_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("实例不存在: {instance_id}"))?
    };
    let mc_version = inst.mc_version.clone();
    let cfg_path = data_dir
        .join("instances")
        .join(&instance_id)
        .join("instance.toml");
    let icfg = InstanceConfig::load(&cfg_path).map_err(|e| e.to_string())?;
    let opts = LaunchOptions {
        username: String::new(),
        min_memory: icfg.jvm.min_memory,
        max_memory: icfg.jvm.max_memory,
        width: icfg.game.resolution.width,
        height: icfg.game.resolution.height,
        ..Default::default()
    };

    tauri::async_runtime::spawn(async move {
        // 1. 自动补齐资源(幂等:已下载且校验通过的文件会跳过)
        let _ = app.emit(
            "game-log",
            GameLogEvent {
                line: "[Runa] 检查并补齐资源...".into(),
            },
        );
        if let Err(e) = run_download(
            client.clone(),
            &data_dir,
            &mc_version,
            retry_times,
            max_concurrent,
            app.clone(),
        )
        .await
        {
            let _ = app.emit(
                "game-log",
                GameLogEvent {
                    line: format!("[Runa] 资源下载失败: {e}"),
                },
            );
            let _ = app.emit("game-exit", GameExitEvent { code: -1 });
            return;
        }
        // 2. 启动
        let result =
            run_launch(client, &data_dir, &config_path, &mc_version, opts, retry_times, app.clone()).await;
        emit_launch_result(&app, result);
    });
    Ok(())
}

fn emit_launch_result(app: &AppHandle, result: Result<i32, RunaError>) {
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
}

async fn run_launch(
    client: reqwest::Client,
    data_dir: &Path,
    config_path: &Path,
    mc_version: &str,
    opts: LaunchOptions,
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

    // 4. 账号解析:未指定用户名时优先使用已登录微软账号,否则离线 Steve
    let (username, uuid, access_token) = if opts.username.is_empty() {
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
            opts.username.clone(),
            uuid::Uuid::new_v4().to_string(),
            "0".into(),
        )
    };
    let opts = LaunchOptions {
        username,
        uuid,
        access_token,
        ..opts
    };

    // 5. 拼装启动参数
    let cfg = AppConfig::load_or_create(config_path)?;
    let java_path = cfg.java_path();
    let paths = LaunchPaths {
        game_dir: layout.game_dir,
        assets_dir: layout.assets_dir,
        libraries_dir: layout.libraries_dir,
        version_dir: version_dir.clone(),
        natives_dir,
    };
    let cmd = build_launch_command(&version, &paths, &opts, &java_path)?;

    // 6. 启动进程,转发日志
    let app2 = app.clone();
    let code = launch_process(&cmd.java_path, &cmd.args, move |line| {
        let _ = app2.emit("game-log", GameLogEvent { line });
    })
    .await?;
    Ok(code)
}
