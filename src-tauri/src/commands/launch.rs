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
use crate::core::loader;
use crate::core::mirror::Mirror;
use crate::core::version::rules::{FeaturesCtx, RuleContext};
use crate::db::repository::Repository;
use crate::error::RmclError;
use crate::AppState;

use super::download::{run_download, DirLayout, DownloadFinishedEvent};

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
    let mirror = state.mirror();
    let opts = LaunchOptions {
        username: username.unwrap_or_default(),
        ..Default::default()
    };

    tauri::async_runtime::spawn(async move {
        let result = run_launch(
            client,
            &data_dir,
            &config_path,
            &mc_version,
            None,
            None,
            None,
            opts,
            &mirror,
            retry_times,
            app.clone(),
        )
        .await;
        emit_launch_result(&app, result);
    });
    Ok(())
}

/// 按实例启动的公共入口(供 launch_instance 与 join_server 复用)。
/// extra_game_args 追加到游戏参数末尾,用于一键加入服务器(--server/--port)。
pub(crate) fn spawn_instance_launch(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    extra_game_args: Vec<String>,
) -> Result<(), String> {
    // 登录门禁:没有激活账号(微软或离线)时阻止启动,避免用空账号拉起游戏进程。
    // 应在 spawn 之前校验,否则后台任务会先走一遍资源下载才报"没账号",体验差。
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        if Repository::get_active_account(&conn)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Err("未登录账号,无法启动游戏。请先在左下角登录(微软或离线账号)".into());
        }
    }

    let client = state.client.clone();
    let data_dir = state.data_dir.clone();
    let config_path = state.config_path.clone();
    let retry_times = state.retry_times;
    let max_concurrent = (state.max_concurrent.max(1)) as usize;
    let mirror = state.mirror();

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
    let loader_name = icfg.meta.loader.clone();
    let loader_version = icfg.meta.loader_version.clone();
    let opts = LaunchOptions {
        username: String::new(),
        min_memory: icfg.jvm.min_memory,
        max_memory: icfg.jvm.max_memory,
        width: icfg.game.resolution.width,
        height: icfg.game.resolution.height,
        extra_game_args,
        ..Default::default()
    };

    tauri::async_runtime::spawn(async move {
        // 1. 自动补齐资源(幂等:已下载且校验通过的文件会跳过)
        let _ = app.emit(
            "game-log",
            GameLogEvent {
                line: "[RustMCL] 检查并补齐资源...".into(),
            },
        );
        if let Err(e) = run_download(
            client.clone(),
            &data_dir,
            &mc_version,
            Some(&loader_name),
            Some(&loader_version),
            retry_times,
            max_concurrent,
            app.clone(),
            &mirror,
        )
        .await
        {
            let _ = app.emit(
                "game-log",
                GameLogEvent {
                    line: format!("[RustMCL] 资源下载失败: {e}"),
                },
            );
            let _ = app.emit("game-exit", GameExitEvent { code: -1 });
            return;
        }
        // 资源补齐完成:通知前端切换进度条 → 启动阶段
        let _ = app.emit(
            "game-log",
            GameLogEvent {
                line: "[RustMCL] 资源检查完成,正在启动游戏...".into(),
            },
        );
        let _ = app.emit(
            "download-finished",
            DownloadFinishedEvent {
                ok: true,
                error: String::new(),
            },
        );
        // 2. 启动:游戏目录使用实例专属目录(保证 mods/存档隔离)
    let game_dir = inst.game_dir.clone();
    let result = run_launch(
        client,
        &data_dir,
        &config_path,
        &mc_version,
        Some(&loader_name),
         Some(&loader_version),
         Some(std::path::Path::new(&game_dir)),
         opts,
        &mirror,
        retry_times,
        app.clone(),
    )
    .await;
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
    spawn_instance_launch(app, state, instance_id, vec![])
}

fn emit_launch_result(app: &AppHandle, result: Result<i32, RmclError>) {
    match result {
        Ok(code) => {
            let _ = app.emit("game-exit", GameExitEvent { code });
        }
        Err(e) => {
            let _ = app.emit(
                "game-log",
                GameLogEvent {
                    line: format!("[RustMCL] 启动失败: {e}"),
                },
            );
            let _ = app.emit("game-exit", GameExitEvent { code: -1 });
        }
    }
}

/// 解析启动账号:
/// - DB 激活账号为离线类型 → 使用固定 UUID,access_token 置 "0"
/// - DB 激活账号为微软类型 → 通过 keyring refresh token 静默续期
/// - 无激活账号 → 离线 Steve 占位(随机 v4 UUID)
async fn resolve_launch_account(
    client: &reqwest::Client,
    db_path: &std::path::Path,
) -> Result<(String, String, String), RmclError> {
    let conn = rusqlite::Connection::open(db_path)?;
    let active = Repository::get_active_account(&conn)?;
    drop(conn);
    match active {
        Some(acc) if acc.account_type == "offline" => {
            Ok((acc.username, acc.uuid, "0".into()))
        }
        Some(_) => resolve_active_account(client)
            .await?
            .ok_or_else(|| RmclError::other("微软账号令牌已失效,请重新登录")),
        None => Ok((
            "Steve".into(),
            uuid::Uuid::new_v4().to_string(),
            "0".into(),
        )),
    }
}

async fn run_launch(
    client: reqwest::Client,
    data_dir: &Path,
    config_path: &Path,
    mc_version: &str,
    loader: Option<&str>,
    loader_version: Option<&str>,
    game_dir_override: Option<&Path>,
    opts: LaunchOptions,
    mirror: &Mirror,
    retry_times: u32,
    app: AppHandle,
) -> Result<i32, RmclError> {
    // 1. version.json(vanilla 或 loader 合并结果,均带本地缓存)
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

    let ctx = RuleContext::current(FeaturesCtx::default());
    let layout = DirLayout::new(data_dir);
    let version_dir = layout.versions_dir.join(&version.id);
    let natives_dir = version_dir.join("natives");

    // 2. 检查文件是否已下载
    let client_item = client_download_item(&version, &version_dir);
    if !client_item.dest.exists() {
        return Err(RmclError::other(format!(
            "缺少客户端文件 {} ,请先在下载页获取该版本",
            client_item.dest.display()
        )));
    }
    let libs = library_items(&version, &ctx, &layout.libraries_dir);
    let natives = native_items(&version, &ctx, &layout.libraries_dir);
    for item in libs.iter().chain(natives.iter()) {
        if !item.dest.exists() {
            return Err(RmclError::other(format!(
                "缺少依赖 {} ,请先在下载页获取该版本",
                item.dest.display()
            )));
        }
    }

    // 3. 解压 natives
    extract_natives(&native_plan(&version, &ctx, &layout.libraries_dir), &natives_dir)?;

    // 4. 账号解析:未指定用户名时优先 DB 中激活账号(离线用固定 UUID,微软走 token 续期),否则离线 Steve
    let (username, uuid, access_token) = if opts.username.is_empty() {
        let db_path = data_dir.join("rmcl.db");
        resolve_launch_account(&client, &db_path).await?
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
    let game_dir = game_dir_override
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| layout.game_dir.clone());
    let paths = LaunchPaths {
        game_dir,
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
