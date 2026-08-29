//! Modrinth API 集成:搜索项目、按游戏版本/加载器过滤版本、解析可下载文件
//! 文档: https://docs.modrinth.com/api-spec/

use serde::{Deserialize, Serialize};

use crate::error::RmclError;

const API_BASE: &str = "https://api.modrinth.com/v2";

/// 搜索命中的项目摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthHit {
    pub project_id: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub icon_url: Option<String>,
    /// 该项目已发布版本号(用于展示最新版本)
    #[serde(default)]
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    hits: Vec<ModrinthHit>,
    #[serde(default)]
    #[allow(dead_code)]
    total_hits: i64,
}

/// 单个可下载文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub hashes: std::collections::HashMap<String, String>,
}

/// 项目版本(含文件)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthVersion {
    pub id: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version_number: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub files: Vec<ModrinthFile>,
}

/// 搜索 Modrinth 项目
pub async fn search(
    client: &reqwest::Client,
    query: &str,
    limit: u32,
    retry_times: u32,
) -> Result<Vec<ModrinthHit>, RmclError> {
    let url = format!("{API_BASE}/search");
    let body = fetch(client, &url, retry_times, &[
        ("query", query),
        ("limit", &limit.to_string()),
        ("index", "relevance"),
    ])
    .await?;
    let resp: SearchResponse = serde_json::from_str(&body)?;
    Ok(resp.hits)
}

/// 获取某项目与当前 MC 版本 + 加载器匹配的版本(新→旧),返回空表示无兼容版本
pub async fn compatible_versions(
    client: &reqwest::Client,
    project_id: &str,
    mc_version: &str,
    loader: &str,
    retry_times: u32,
) -> Result<Vec<ModrinthVersion>, RmclError> {
    let url = format!("{API_BASE}/project/{project_id}/version");
    let body = fetch(client, &url, retry_times, &[]).await?;
    let all: Vec<ModrinthVersion> = serde_json::from_str(&body)?;
    Ok(all
        .into_iter()
        .filter(|v| {
            v.game_versions.iter().any(|g| g == mc_version)
                && v.loaders.iter().any(|l| l == loader)
        })
        .collect())
}

/// 从版本中挑选 primary 文件(无则取第一个)
pub fn primary_file(version: &ModrinthVersion) -> Option<&ModrinthFile> {
    version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
}

/// 按版本 id 获取单个版本详情(安装时用)
pub async fn fetch_version(
    client: &reqwest::Client,
    version_id: &str,
    retry_times: u32,
) -> Result<ModrinthVersion, RmclError> {
    let url = format!("{API_BASE}/version/{version_id}");
    let body = fetch(client, &url, retry_times, &[]).await?;
    Ok(serde_json::from_str(&body)?)
}

async fn fetch(
    client: &reqwest::Client,
    url: &str,
    retry_times: u32,
    query: &[(&str, &str)],
) -> Result<String, RmclError> {
    let mut last_err = None;
    for attempt in 0..=retry_times {
        let mut req = client.get(url);
        for (k, v) in query {
            req = req.query(&[(k, *v)]);
        }
        match req.send().await {
            Ok(resp) => match resp.error_for_status() {
                Ok(resp) => match resp.text().await {
                    Ok(body) => return Ok(body),
                    Err(e) => last_err = Some(RmclError::Network(e)),
                },
                Err(e) => last_err = Some(RmclError::Network(e)),
            },
            Err(e) => last_err = Some(RmclError::Network(e)),
        }
        if attempt < retry_times {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    Err(last_err.unwrap_or_else(|| RmclError::other("Modrinth 请求失败")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_response() {
        let body = r#"{
          "hits": [
            {"project_id": "A", "slug": "sodium", "title": "Sodium",
             "description": "Modern rendering engine", "categories": ["performance"],
             "downloads": 123456, "icon_url": "https://cdn/icon.png", "versions": ["1.21.1"]}
          ],
          "total_hits": 1
        }"#;
        let resp: SearchResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.hits.len(), 1);
        assert_eq!(resp.hits[0].project_id, "A");
        assert_eq!(resp.total_hits, 1);
    }

    #[test]
    fn parses_version_with_files() {
        let body = r#"{
          "id": "v1", "name": "Sodium 0.6.0", "version_number": "0.6.0",
          "game_versions": ["1.21.1"], "loaders": ["fabric"],
          "files": [{"url": "https://cdn/x.jar", "filename": "sodium-0.6.0.jar",
                     "primary": true, "size": 1000, "hashes": {"sha1": "abc", "sha512": "def"}}]
        }"#;
        let v: ModrinthVersion = serde_json::from_str(body).unwrap();
        assert_eq!(v.id, "v1");
        assert_eq!(v.files.len(), 1);
        assert_eq!(primary_file(&v).unwrap().filename, "sodium-0.6.0.jar");
        assert_eq!(v.files[0].hashes.get("sha1").unwrap(), "abc");
    }

    #[test]
    fn filters_incompatible_versions() {
        let all: Vec<ModrinthVersion> = serde_json::from_str(
            r#"[
              {"id": "a", "game_versions": ["1.21.1"], "loaders": ["fabric"], "files": []},
              {"id": "b", "game_versions": ["1.20.1"], "loaders": ["fabric"], "files": []},
              {"id": "c", "game_versions": ["1.21.1"], "loaders": ["forge"], "files": []}
            ]"#,
        )
        .unwrap();
        let compatible: Vec<_> = all
            .into_iter()
            .filter(|v| {
                v.game_versions.iter().any(|g| g == "1.21.1")
                    && v.loaders.iter().any(|l| l == "fabric")
            })
            .collect();
        assert_eq!(compatible.len(), 1);
        assert_eq!(compatible[0].id, "a");
    }
}
