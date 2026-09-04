//! CurseForge API 集成(模块 2):搜索、按 MC 版本/加载器过滤、解析可下载文件。
//! 文档:https://docs.curseforge.com/ ;需要 API Key(通过配置注入,不硬编码)。

use serde::{Deserialize, Serialize};

use crate::error::RmclError;

const API_BASE: &str = "https://api.curseforge.com/v1";
const GAME_ID: i64 = 432; // Minecraft

/// modLoaderType:forge=1, fabric=4, quilt=5
pub fn loader_type(loader: &str) -> Option<i64> {
    match loader {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        _ => None,
    }
}

/// 搜索命中的项目摘要(与 ModrinthHit 字段对齐,便于前端统一展示)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub categories: Vec<String>,
    pub downloads: i64,
    pub icon_url: Option<String>,
    pub versions: Vec<String>,
    /// false 表示作者禁止第三方启动器分发(需引导用户手动下载)
    #[serde(default = "default_true")]
    pub allow_mod_distribution: bool,
}

fn default_true() -> bool {
    true
}

/// 单个可下载文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeFile {
    pub file_id: i64,
    pub filename: String,
    pub url: String,
    pub size: i64,
    pub sha1: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchWrap {
    #[serde(default)]
    data: Vec<CfRawProject>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfRawProject {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    download_count: i64,
    #[serde(default)]
    logo: Option<CfLogo>,
    #[serde(default)]
    categories: Vec<CfCategory>,
    #[serde(default)]
    latest_files_indexes: Vec<CfLatestFile>,
    #[serde(default)]
    allow_mod_distribution: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct CfLogo {
    #[serde(default)]
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CfCategory {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfLatestFile {
    #[serde(default)]
    game_version: String,
    #[allow(dead_code)]
    #[serde(default)]
    file_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct FilesWrap {
    #[serde(default)]
    data: Vec<CfRawFile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfRawFile {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    file_length: i64,
    #[serde(default)]
    hashes: Vec<CfHash>,
}

#[derive(Debug, Clone, Deserialize)]
struct CfHash {
    #[serde(default)]
    value: String,
    #[serde(default)]
    algo: i64,
}

/// 搜索项目
#[allow(clippy::too_many_arguments)]
pub async fn search(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    mc_version: &str,
    loader: &str,
    limit: u32,
) -> Result<Vec<CurseForgeHit>, RmclError> {
    let mut req = client
        .get(format!("{API_BASE}/mods/search"))
        .header("x-api-key", api_key)
        .query(&[
            ("gameId", GAME_ID.to_string()),
            ("searchFilter", query.to_string()),
            ("sortField", "6".to_string()),
            ("sortOrder", "desc".to_string()),
            ("pageSize", limit.to_string()),
        ]);
    if let Some(lt) = loader_type(&loader) {
        req = req.query(&[("modLoaderType", lt.to_string())]);
    }
    if !mc_version.is_empty() {
        req = req.query(&[("gameVersion", mc_version.to_string())]);
    }
    let body = fetch_body(client, req).await?;
    let wrap: SearchWrap = serde_json::from_str(&body)?;
    Ok(wrap
        .data
        .into_iter()
        .map(|p| CurseForgeHit {
            project_id: p.id.to_string(),
            slug: p.slug,
            title: p.name,
            description: p.summary,
            categories: p.categories.into_iter().map(|c| c.name).collect(),
            downloads: p.download_count,
            icon_url: p.logo.map(|l| l.url).filter(|u| !u.is_empty()),
            versions: p
                .latest_files_indexes
                .into_iter()
                .map(|f| f.game_version)
                .collect(),
            allow_mod_distribution: p.allow_mod_distribution,
        })
        .collect())
}

/// 获取某 mod 在指定 MC 版本/加载器下的可用文件列表
pub async fn file_versions(
    client: &reqwest::Client,
    api_key: &str,
    mod_id: &str,
    mc_version: &str,
    loader: &str,
) -> Result<Vec<CurseForgeFile>, RmclError> {
    let mut req = client
        .get(format!("{API_BASE}/mods/{mod_id}/files"))
        .header("x-api-key", api_key)
        .query(&[("pageSize", "50".to_string())]);
    if !mc_version.is_empty() {
        req = req.query(&[("gameVersion", mc_version.to_string())]);
    }
    if let Some(lt) = loader_type(&loader) {
        req = req.query(&[("modLoaderType", lt.to_string())]);
    }
    let body = fetch_body(client, req).await?;
    let wrap: FilesWrap = serde_json::from_str(&body)?;
    Ok(wrap
        .data
        .into_iter()
        .map(|f| {
            let sha1 = f
                .hashes
                .iter()
                .find(|h| h.algo == 1)
                .map(|h| h.value.clone())
                .unwrap_or_default();
            CurseForgeFile {
                file_id: f.id,
                filename: f.file_name,
                url: f.download_url.unwrap_or_default(),
                size: f.file_length,
                sha1,
            }
        })
        .collect())
}

pub async fn fetch_body(
    _client: &reqwest::Client,
    req: reqwest::RequestBuilder,
) -> Result<String, RmclError> {
    let resp = req.send().await?.error_for_status()?;
    Ok(resp.text().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_type_mapping() {
        assert_eq!(loader_type("forge"), Some(1));
        assert_eq!(loader_type("fabric"), Some(4));
        assert_eq!(loader_type("quilt"), Some(5));
        assert_eq!(loader_type("vanilla"), None);
    }

    #[test]
    fn parses_search_wrap() {
        let body = r#"{"data":[{"id":123,"slug":"sodium","name":"Sodium","summary":"Modern engine","downloadCount":999,"logo":{"url":"https://cdn/x.png"},"categories":[{"name":"optimization"}],"latestFilesIndexes":[{"gameVersion":"1.20.1","fileId":11}]}]}"#;
        let wrap: SearchWrap = serde_json::from_str(body).unwrap();
        assert_eq!(wrap.data.len(), 1);
        assert_eq!(wrap.data[0].id, 123);
        assert_eq!(wrap.data[0].download_count, 999);
    }

    #[test]
    fn parses_files_wrap() {
        let body = r#"{"data":[{"id":99,"fileName":"sodium.jar","downloadUrl":"https://cdn/sodium.jar","fileLength":1000,"hashes":[{"value":"abc","algo":1},{"value":"def","algo":2}]}]}"#;
        let wrap: FilesWrap = serde_json::from_str(body).unwrap();
        let f = &wrap.data[0];
        let sha1 = f.hashes.iter().find(|h| h.algo == 1).unwrap().value.clone();
        assert_eq!(sha1, "abc");
        assert_eq!(f.file_name, "sodium.jar");
    }
}
