// schema 中声明的模型由后续阶段(M2/M4/M6)使用,未用时不告警
#![allow(dead_code)]

use serde::Serialize;

/// 与 SQLite 表结构对应的模型,同时作为 Tauri command 的返回类型
/// 前端 `src/lib/types.ts` 中的类型与本文件保持同步

pub const TABLE_INSTANCES: &str = "instances";
pub const TABLE_ACCOUNTS: &str = "accounts";
pub const TABLE_MODS: &str = "mods";
pub const TABLE_ASSET_CACHE: &str = "asset_cache";

/// 加载器类型:vanilla | forge | fabric | quilt
#[derive(Debug, Clone, Serialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub game_dir: String,
    pub icon_path: Option<String>,
    pub created_at: i64,
    pub last_played: Option<i64>,
}

/// 账号类型: microsoft | offline
#[derive(Debug, Clone, Serialize)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub account_type: String,
    pub is_active: bool,
    pub refreshed_at: Option<i64>,
}

/// mod 来源: modrinth | curseforge | local
#[derive(Debug, Clone, Serialize)]
pub struct ModEntry {
    pub id: String,
    pub instance_id: String,
    pub file_name: String,
    pub source: Option<String>,
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetCache {
    pub sha1: String,
    pub path: String,
    pub size: i64,
}
