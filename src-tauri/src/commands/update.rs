//! 自更新机制(模块 10):基于 tauri-plugin-updater 检查更新。
//! 说明:真正下载/安装需要配置更新清单(见 tauri.conf.json 的 plugins.updater)与签名密钥,
//! 由发布方可在此接入 GitHub Releases;未配置时命令返回友好提示,不影响应用其它功能。

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub has_update: bool,
    pub notes: String,
}

/// 检查更新:返回当前/最新版本与是否有可用更新;未配置更新源时返回提示
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => Ok(UpdateInfo {
                current,
                latest: update.version.clone(),
                has_update: true,
                notes: update.body.clone().unwrap_or_default(),
            }),
            Ok(None) => Ok(UpdateInfo {
                current: current.clone(),
                latest: current,
                has_update: false,
                notes: String::new(),
            }),
            Err(e) => Err(format!("检查更新失败: {e}")),
        },
        Err(e) => Err(format!("更新源未配置(请在 tauri.conf.json 配置 plugins.updater): {e}")),
    }
}

/// 下载并安装最新更新(带静默进度回调),成功后重启应用
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<String, String> {
    let updater = app
        .updater()
        .map_err(|e| format!("更新源未配置(请在 tauri.conf.json 配置 plugins.updater): {e}"))?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("检查更新失败: {e}"))?
        .ok_or_else(|| "当前已是最新版本".to_string())?;
    update
        .download_and_install(
            |_, _| {
                // 进度回调:当前简化为静默,前端可通过再次检查获取状态
            },
            || {},
        )
        .await
        .map_err(|e| format!("下载并安装更新失败: {e}"))?;
    Ok("更新已安装,应用即将重启".into())
}
