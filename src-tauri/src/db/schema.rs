// schema 中声明的模型由后续阶段(M2/M4/M6)使用,未用时不告警
#![allow(dead_code)]

use serde::Serialize;

/// 与 SQLite 表结构对应的模型,同时作为 Tauri command 的返回类型
/// 前端 `src/lib/types.ts` 中的类型与本文件保持同步

pub const TABLE_INSTANCES: &str = "instances";
pub const TABLE_ACCOUNTS: &str = "accounts";
pub const TABLE_MODS: &str = "mods";
pub const TABLE_ASSET_CACHE: &str = "asset_cache";
pub const TABLE_SERVERS: &str = "servers";
pub const TABLE_RESOURCE_PACKS: &str = "resource_packs";

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

/// 服务器条目(模块 1)
#[derive(Debug, Clone, Serialize)]
pub struct ServerEntry {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub is_favorite: bool,
    pub icon_base64: Option<String>,
    pub last_ping_ms: Option<i64>,
    pub sort_order: i64,
    pub created_at: i64,
}

/// 服务器 ping 结果(模块 1.3)
#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub id: String,
    pub motd: String,
    pub players_online: i64,
    pub players_max: i64,
    pub latency_ms: u64,
    pub favicon: Option<String>,
    pub ok: bool,
}

/// 资源包/光影包条目(模块 4)
#[derive(Debug, Clone, Serialize)]
pub struct ResourcePackEntry {
    pub id: String,
    pub instance_id: String,
    /// resourcepack | shaderpack
    pub type_kind: String,
    pub file_name: String,
    pub enabled: bool,
    pub created_at: i64,
}
