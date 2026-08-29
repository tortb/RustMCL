use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::RmclError;

/// Mojang 版本清单地址
pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<VersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    /// release | snapshot | old_beta | old_alpha
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: String,
    pub compliance_level: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionFilter {
    All,
    Release,
    Snapshot,
}

impl VersionFilter {
    pub fn from_str(s: &str) -> Self {
        match s {
            "release" => VersionFilter::Release,
            "snapshot" => VersionFilter::Snapshot,
            _ => VersionFilter::All,
        }
    }
}

/// 拉取清单,失败时按 retry_times 重试
pub async fn fetch(client: &reqwest::Client, retry_times: u32) -> Result<VersionManifest, RmclError> {
    let mut last_err = None;
    for attempt in 0..=retry_times {
        match client.get(MANIFEST_URL).send().await {
            Ok(resp) => match resp.error_for_status() {
                Ok(resp) => {
                    let body = resp.text().await?;
                    return Ok(serde_json::from_str(&body)?);
                }
                Err(e) => last_err = Some(RmclError::Network(e)),
            },
            Err(e) => last_err = Some(RmclError::Network(e)),
        }
        if attempt < retry_times {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    Err(last_err.unwrap_or_else(|| RmclError::other("拉取版本清单失败")))
}

/// 优先读本地缓存;force_refresh 时强制重新拉取
pub async fn get_manifest(
    client: &reqwest::Client,
    cache_path: &Path,
    force_refresh: bool,
    retry_times: u32,
) -> Result<VersionManifest, RmclError> {
    if !force_refresh {
        if let Some(m) = load_cache(cache_path) {
            return Ok(m);
        }
    }
    let m = fetch(client, retry_times).await?;
    save_cache(cache_path, &m)?;
    Ok(m)
}

pub fn list_versions(manifest: &VersionManifest, filter: VersionFilter) -> Vec<&VersionInfo> {
    manifest
        .versions
        .iter()
        .filter(|v| match filter {
            VersionFilter::All => true,
            VersionFilter::Release => v.version_type == "release",
            VersionFilter::Snapshot => v.version_type == "snapshot",
        })
        .collect()
}

fn load_cache(path: &Path) -> Option<VersionManifest> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_cache(path: &Path, manifest: &VersionManifest) -> Result<(), RmclError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string(manifest)?;
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 从真实清单中截取的一段数据,避免测试依赖网络
    const SAMPLE: &str = r#"{
      "latest": { "release": "1.21.4", "snapshot": "25w02a" },
      "versions": [
        { "id": "1.21.4", "type": "release", "url": "https://example.com/1.21.4.json", "time": "2024-12-03T12:14:08+00:00", "releaseTime": "2024-12-03T11:31:07+00:00", "sha1": "a1", "complianceLevel": 1 },
        { "id": "1.21.3", "type": "release", "url": "https://example.com/1.21.3.json", "time": "2024-10-23T10:24:12+00:00", "releaseTime": "2024-10-23T10:17:31+00:00", "sha1": "a2", "complianceLevel": 1 },
        { "id": "1.21.1", "type": "release", "url": "https://example.com/1.21.1.json", "time": "2024-08-08T09:44:16+00:00", "releaseTime": "2024-08-08T09:31:17+00:00", "sha1": "a3", "complianceLevel": 1 },
        { "id": "1.21",   "type": "release", "url": "https://example.com/1.21.json",   "time": "2024-06-13T10:15:10+00:00", "releaseTime": "2024-06-13T09:28:36+00:00", "sha1": "a4", "complianceLevel": 1 },
        { "id": "1.20.6", "type": "release", "url": "https://example.com/1.20.6.json", "time": "2024-04-29T11:24:17+00:00", "releaseTime": "2024-04-29T10:47:15+00:00", "sha1": "a5", "complianceLevel": 1 },
        { "id": "1.20.5", "type": "release", "url": "https://example.com/1.20.5.json", "time": "2024-04-23T12:17:39+00:00", "releaseTime": "2024-04-23T10:13:42+00:00", "sha1": "a6", "complianceLevel": 1 },
        { "id": "25w02a", "type": "snapshot", "url": "https://example.com/25w02a.json", "time": "2025-01-08T13:11:55+00:00", "releaseTime": "2025-01-08T12:21:39+00:00", "sha1": "s1" },
        { "id": "24w46a", "type": "snapshot", "url": "https://example.com/24w46a.json", "time": "2024-11-13T13:45:17+00:00", "releaseTime": "2024-11-13T12:53:42+00:00", "sha1": "s2" },
        { "id": "b1.7.3", "type": "old_beta", "url": "https://example.com/b1.7.3.json", "time": "2011-07-08T13:11:55+00:00", "releaseTime": "2011-07-08T13:11:55+00:00", "sha1": "o1" }
      ]
    }"#;

    #[test]
    fn parses_manifest_and_filters_releases() {
        let m: VersionManifest = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(m.latest.release, "1.21.4");
        let releases = list_versions(&m, VersionFilter::Release);
        assert!(releases.len() >= 5, "应至少解析出 5 个正式版,实际 {}", releases.len());
        assert!(releases.iter().all(|v| v.version_type == "release"));
    }

    #[test]
    fn filters_snapshot_and_all() {
        let m: VersionManifest = serde_json::from_str(SAMPLE).unwrap();
        let snapshots = list_versions(&m, VersionFilter::Snapshot);
        assert_eq!(snapshots.len(), 2);
        let all = list_versions(&m, VersionFilter::All);
        assert_eq!(all.len(), 9);
    }

    #[test]
    fn cache_roundtrip() {
        let m: VersionManifest = serde_json::from_str(SAMPLE).unwrap();
        let path = std::env::temp_dir().join("rmcl_manifest_cache_test.json");
        save_cache(&path, &m).unwrap();
        let loaded = load_cache(&path).unwrap();
        assert_eq!(loaded.versions.len(), m.versions.len());
        let _ = std::fs::remove_file(&path);
    }
}
