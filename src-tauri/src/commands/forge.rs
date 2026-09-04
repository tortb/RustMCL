//! Forge 相关命令

use tauri::{AppHandle, Emitter, State};

use crate::core::mirror::Mirror;
use crate::core::mods::forge::installer::{extract_installer, extract_installer_files};
use crate::core::mods::forge::version_list::ForgeVersionInfo;
use crate::core::mods::forge::{
    forge_work_dir, is_legacy, resolve_forge_version, run_installer_processors,
};
use crate::error::RmclError;
use crate::{config::app_config::AppConfig, AppState};

use super::download::{run_download_for_version, DownloadFinishedEvent};

/// 返回指定 MC 版本可用的 Forge 版本(recommended/latest 已标记)
#[tauri::command]
pub async fn list_forge_versions(
    state: State<'_, AppState>,
    mc_version: String,
) -> Result<Vec<ForgeVersionInfo>, String> {
    crate::core::mods::forge::version_list::list_forge_versions(
        &state.client,
        &state.mirror(),
        &mc_version,
        state.retry_times,
    )
    .await
    .map_err(|e| e.to_string())
}

/// 后台安装 Forge:解析合并 version.json → 下载依赖(client.jar + libraries + natives + assets)
/// → 解压 installer 并运行 java 处理器 → 写缓存供启动。结果通过 "loader-install-finished" 事件通知。
/// 旧版(≤1.12.2)走 legacy 路径(universal jar)。
#[tauri::command]
pub fn install_forge(
    app: AppHandle,
    state: State<'_, AppState>,
    mc_version: String,
    forge_version: String,
) -> Result<(), String> {
    let client = state.client.clone();
    let data_dir = state.data_dir.clone();
    let config_path = state.config_path.clone();
    let retry_times = state.retry_times;
    let max_concurrent = (state.max_concurrent.max(1)) as usize;
    let mirror = state.mirror();

    tauri::async_runtime::spawn(async move {
        let result = install_forge_inner(
            client,
            &data_dir,
            &config_path,
            &mc_version,
            &forge_version,
            retry_times,
            max_concurrent,
            app.clone(),
            &mirror,
        )
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

async fn install_forge_inner(
    client: reqwest::Client,
    data_dir: &std::path::Path,
    config_path: &std::path::Path,
    mc_version: &str,
    forge_version: &str,
    retry_times: u32,
    max_concurrent: usize,
    app: AppHandle,
    mirror: &Mirror,
) -> Result<(), RmclError> {
    // 1. 解析并合并 version.json(下载 installer + 与原版合并,带缓存)。
    //    旧版(≤1.12.2)走 legacy 路径(锁定 launchwrapper + universal jar),但同样拿到合并结果。
    let version = if is_legacy(mc_version) {
        crate::core::mods::forge::legacy::resolve_legacy(
            &client,
            mirror,
            data_dir,
            mc_version,
            forge_version,
            retry_times,
        )
        .await?
    } else {
        resolve_forge_version(
            &client,
            mirror,
            data_dir,
            mc_version,
            forge_version,
            retry_times,
        )
        .await?
    };

    // 2. 下载合并版本的全部依赖(client.jar + libraries + natives + assets),供 processors 使用
    run_download_for_version(
        &client,
        data_dir,
        &version,
        retry_times,
        max_concurrent,
        app.clone(),
        mirror,
        None,
    )
    .await?;

    // 3. 旧版无需二进制补丁处理器,直接完成
    if is_legacy(mc_version) {
        return Ok(());
    }

    // 4. 新版:解压 installer(内存读取 processors + 全量解压到工作目录供 data/ 读取)
    let work = forge_work_dir(data_dir, mc_version, forge_version);
    let jar = work.join("forge-installer.jar");
    let contents = extract_installer(&jar, mc_version, forge_version)?;
    extract_installer_files(&jar, &work)?;

    // 5. 下载处理器工具链库(install_profile.libraries,processors 的 classpath/jar 依赖)
    let items = crate::core::mods::forge::processor_library_items(&contents, data_dir, mirror);
    if !items.is_empty() {
        let app2 = app.clone();
        crate::core::downloader::download_many(
            &client,
            mirror,
            items,
            max_concurrent,
            retry_times,
            None,
            move |p| {
                let _ = app2.emit(
                    "download-progress",
                    super::download::DownloadProgressEvent {
                        phase: "processor-libs".into(),
                        current: p.done,
                        total: p.total,
                        file: p.file,
                    },
                );
            },
        )
        .await?;
    }

    // 6. 运行 java 处理器(阻塞,放到 blocking 池)
    let java_path = AppConfig::load_or_create(config_path)?.java_path();
    if java_path.trim().is_empty() {
        return Err(RmclError::other("未配置 Java 路径,请先在设置页选择 Java"));
    }
    let data_dir = data_dir.to_path_buf();
    let mc_version = mc_version.to_string();
    let forge_version = forge_version.to_string();
    let res = tauri::async_runtime::spawn_blocking(move || {
        run_installer_processors(
            &contents,
            &data_dir,
            &mc_version,
            &forge_version,
            &java_path,
        )
    })
    .await
    .map_err(|e| RmclError::other(format!("处理器线程异常: {e}")))?;
    res?;

    Ok(())
}
