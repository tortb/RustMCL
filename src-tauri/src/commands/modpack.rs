//! 整合包导入/导出命令
//! 导入为异步后台任务:解析 → 校验兼容性 → 下载 + overrides → 记录 DB,进度/结果通过事件上报。
//! 导出在游戏目录扫描已安装 mod,打包为 .mrpack。

use std::path::Path;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::config::app_config::AppConfig;
use crate::core::modpack;
use crate::db::repository::Repository;
use crate::db::schema::ModEntry;
use crate::error::RmclError;
use crate::AppState;

#[derive(Clone, serde::Serialize)]
pub struct ModpackProgressEvent {
    pub current: usize,
    pub total: usize,
    pub file: String,
}

#[derive(Clone, serde::Serialize)]
pub struct ModpackFinishedEvent {
    pub ok: bool,
    pub error: String,
    pub installed: Vec<String>,
    pub failures: Vec<String>,
    pub name: String,
}

/// 导入整合包到指定实例(后台执行,结果通过 "modpack-finished" 事件通知)
#[tauri::command]
pub fn import_modpack(
    app: AppHandle,
    state: State<'_, AppState>,
    file_path: String,
    instance_id: String,
) -> Result<(), String> {
    let client = state.client.clone();
    let mirror = state.mirror();
    let data_dir = state.data_dir.clone();
    let config_path = state.config_path.clone();
    let retry_times = state.retry_times;
    let max_concurrent = (state.max_concurrent.max(1)) as usize;

    let inst = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_instance(&conn, &instance_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("实例不存在: {instance_id}"))?
    };
    let game_dir = inst.game_dir.clone();
    let mc_version = inst.mc_version.clone();
    let loader = inst.loader.clone().unwrap_or_else(|| "vanilla".into());

    tauri::async_runtime::spawn(async move {
        let result = import_inner(
            client,
            &mirror,
            &data_dir,
            &config_path,
            &file_path,
            &instance_id,
            &game_dir,
            &mc_version,
            &loader,
            retry_times,
            max_concurrent,
            app.clone(),
        )
        .await;
        let _ = app.emit(
            "modpack-finished",
            match result {
                Ok((name, installed, failures)) => ModpackFinishedEvent {
                    ok: true,
                    error: String::new(),
                    installed,
                    failures,
                    name,
                },
                Err(e) => ModpackFinishedEvent {
                    ok: false,
                    error: e.to_string(),
                    installed: Vec::new(),
                    failures: Vec::new(),
                    name: String::new(),
                },
            },
        );
    });
    Ok(())
}

async fn import_inner(
    client: reqwest::Client,
    mirror: &crate::core::mirror::Mirror,
    data_dir: &Path,
    config_path: &Path,
    file_path: &str,
    instance_id: &str,
    game_dir: &str,
    mc_version: &str,
    loader: &str,
    retry_times: u32,
    max_concurrent: usize,
    app: AppHandle,
) -> Result<(String, Vec<String>, Vec<String>), RmclError> {
    let pack_path = Path::new(file_path);
    let info = modpack::parse(pack_path)?;
    // 面向实例的兼容性校验(不匹配提前失败,避免装一半)
    modpack::validate(&info, mc_version, loader)?;

    let cfg = AppConfig::load_or_create(config_path)?;
    let curseforge_key = cfg.curseforge_api_key.as_deref();

    std::fs::create_dir_all(game_dir)?;
    let app1 = app.clone();
    let result = modpack::install_pack(
        &client,
        mirror,
        &info,
        pack_path,
        Path::new(game_dir),
        retry_times,
        max_concurrent,
        curseforge_key,
        move |current, total, file| {
            let _ = app1.emit(
                "modpack-progress",
                ModpackProgressEvent {
                    current,
                    total,
                    file,
                },
            );
        },
    )
    .await?;

    // 记录安装的 mod 到 DB(project/version 暂缺,source 标记来源)
    {
        let state = app.state::<crate::AppState>();
        let conn = state
            .db
            .lock()
            .map_err(|e| RmclError::other(format!("数据库锁获取失败: {e}")))?;
        for file_name in &result.installed {
            let entry = ModEntry {
                id: uuid::Uuid::new_v4().simple().to_string(),
                instance_id: instance_id.to_string(),
                file_name: file_name.clone(),
                source: Some(info.source.label().into()),
                project_id: None,
                version_id: None,
                enabled: true,
            };
            Repository::insert_mod(&conn, &entry)?;
        }
    }
    let _ = data_dir;

    Ok((info.name, result.installed, result.failures))
}

/// 导出实例为 .mrpack(同步):扫描 game_dir/mods 下已安装的 mod
#[tauri::command]
pub fn export_modpack(
    state: State<'_, AppState>,
    instance_id: String,
    dest_path: String,
) -> Result<(), String> {
    let inst = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::get_instance(&conn, &instance_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("实例不存在: {instance_id}"))?
    };
    // 从 DB 读已安装 mod 作为导出清单
    let mods: Vec<String> = {
        let conn = state
            .db
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        Repository::list_mods(&conn, &instance_id)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|m| m.file_name)
            .collect()
    };
    let mc = inst.mc_version.clone();
    let loader = inst.loader.clone().unwrap_or_else(|| "vanilla".into());
    let loader_version = inst.loader_version.clone().unwrap_or_default();
    let name = inst.name.clone();
    modpack::export_mrpack(
        Path::new(&inst.game_dir),
        &mc,
        &loader,
        &loader_version,
        &name,
        &mods,
        Path::new(&dest_path),
    )
    .map_err(|e| e.to_string())
}
