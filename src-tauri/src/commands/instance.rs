//! 实例管理命令:SQLite + instance.toml 双写
//! 实例目录结构: ~/.rustmcl/instances/<id>/instance.toml

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::config::instance_config::{
    GameConfig, InstanceConfig, JvmConfig, MetaConfig, Resolution,
};
use crate::db::repository::Repository;
use crate::db::schema::Instance;
use crate::AppState;

/// 前端创建/更新实例时传入的参数(字段均可选,未传用默认)
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct InstanceInput {
    pub name: String,
    pub mc_version: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub min_memory: Option<u32>,
    pub max_memory: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Default for InstanceInput {
    fn default() -> Self {
        Self {
            name: "新实例".into(),
            mc_version: String::new(),
            loader: None,
            loader_version: None,
            min_memory: None,
            max_memory: None,
            width: None,
            height: None,
        }
    }
}

/// 实例详情:DB 记录 + TOML 配置
#[derive(Debug, Clone, Serialize)]
pub struct InstanceDetail {
    #[serde(flatten)]
    pub inst: Instance,
    pub config: InstanceConfig,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn instance_dir(data_dir: &std::path::Path, id: &str) -> std::path::PathBuf {
    data_dir.join("instances").join(id)
}

fn instance_config_path(data_dir: &std::path::Path, id: &str) -> std::path::PathBuf {
    instance_dir(data_dir, id).join("instance.toml")
}

/// 创建实例:生成 id、写 instance.toml、写 DB
#[tauri::command]
pub fn create_instance(
    state: State<'_, AppState>,
    input: InstanceInput,
) -> Result<InstanceDetail, String> {
    if input.name.trim().is_empty() {
        return Err("实例名称不能为空".into());
    }
    if input.mc_version.trim().is_empty() {
        return Err("请选择 Minecraft 版本".into());
    }
    let id = uuid::Uuid::new_v4().simple().to_string();
    let loader = input.loader.unwrap_or_else(|| "vanilla".into());
    let loader_version = input.loader_version.unwrap_or_default();
    let icfg = InstanceConfig {
        meta: MetaConfig {
            name: input.name.trim().to_string(),
            mc_version: input.mc_version.clone(),
            loader: loader.clone(),
            loader_version: loader_version.clone(),
        },
        jvm: JvmConfig {
            min_memory: input.min_memory.unwrap_or(1024),
            max_memory: input.max_memory.unwrap_or(4096),
            extra_args: Vec::new(),
        },
        game: GameConfig {
            resolution: Resolution {
                width: input.width.unwrap_or(1280),
                height: input.height.unwrap_or(720),
            },
            fullscreen: false,
        },
    };

    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;

    let inst = Instance {
        id: id.clone(),
        name: icfg.meta.name.clone(),
        mc_version: icfg.meta.mc_version.clone(),
        loader: Some(loader),
        loader_version: if loader_version.is_empty() {
            None
        } else {
            Some(loader_version)
        },
        game_dir: instance_dir(&state.data_dir, &id)
            .to_string_lossy()
            .to_string(),
        icon_path: None,
        created_at: now_secs(),
        last_played: None,
    };
    Repository::create_instance(&conn, &inst).map_err(|e| e.to_string())?;
    drop(conn);

    icfg.save(&instance_config_path(&state.data_dir, &id))
        .map_err(|e| e.to_string())?;

    Ok(InstanceDetail { inst, config: icfg })
}

/// 实例列表
#[tauri::command]
pub fn list_instances(state: State<'_, AppState>) -> Result<Vec<Instance>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::list_instances(&conn).map_err(|e| e.to_string())
}

/// 实例详情(DB + TOML)
#[tauri::command]
pub fn get_instance(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<InstanceDetail>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let inst = match Repository::get_instance(&conn, &id).map_err(|e| e.to_string())? {
        Some(i) => i,
        None => return Ok(None),
    };
    let cfg_path = instance_config_path(&state.data_dir, &id);
    let config = InstanceConfig::load(&cfg_path).map_err(|e| e.to_string())?;
    Ok(Some(InstanceDetail { inst, config }))
}

/// 更新实例(名称/版本/加载器/内存/分辨率)
#[tauri::command]
pub fn update_instance(
    state: State<'_, AppState>,
    id: String,
    input: InstanceInput,
) -> Result<InstanceDetail, String> {
    if input.name.trim().is_empty() {
        return Err("实例名称不能为空".into());
    }
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let inst = Repository::get_instance(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("实例不存在: {id}"))?;
    let cfg_path = instance_config_path(&state.data_dir, &id);
    let mut icfg = InstanceConfig::load(&cfg_path).map_err(|e| e.to_string())?;

    icfg.meta.name = input.name.trim().to_string();
    if !input.mc_version.is_empty() {
        icfg.meta.mc_version = input.mc_version.clone();
    }
    if let Some(loader) = input.loader {
        icfg.meta.loader = loader;
    }
    if let Some(lv) = input.loader_version {
        icfg.meta.loader_version = lv;
    }
    if let Some(m) = input.min_memory {
        icfg.jvm.min_memory = m;
    }
    if let Some(m) = input.max_memory {
        icfg.jvm.max_memory = m;
    }
    if let Some(w) = input.width {
        icfg.game.resolution.width = w;
    }
    if let Some(h) = input.height {
        icfg.game.resolution.height = h;
    }

    let updated = Instance {
        name: icfg.meta.name.clone(),
        mc_version: icfg.meta.mc_version.clone(),
        loader: if icfg.meta.loader.is_empty() {
            None
        } else {
            Some(icfg.meta.loader.clone())
        },
        loader_version: if icfg.meta.loader_version.is_empty() {
            None
        } else {
            Some(icfg.meta.loader_version.clone())
        },
        ..inst
    };
    Repository::update_instance(&conn, &updated).map_err(|e| e.to_string())?;
    drop(conn);

    icfg.save(&cfg_path).map_err(|e| e.to_string())?;
    Ok(InstanceDetail {
        inst: updated,
        config: icfg,
    })
}

/// 删除实例:DB 记录 + 整个实例目录
#[tauri::command]
pub fn delete_instance(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::delete_instance(&conn, &id).map_err(|e| e.to_string())?;
    drop(conn);

    let dir = instance_dir(&state.data_dir, &id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("删除实例目录失败: {e}"))?;
    }
    Ok(())
}
