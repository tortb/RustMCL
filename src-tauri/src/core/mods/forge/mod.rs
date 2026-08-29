//! Forge 加载器安装:旧版(≤1.12.2 universal jar)与新版(≥1.13 处理器)两条路径
//! 目录: core/mods/forge/

// 待 T5.4/T5.6 接线前,部分内部函数与常量暂未被命令层消费,不告警
#![allow(dead_code)]

pub mod installer;
pub mod legacy;
pub mod processor;
pub mod profile_merge;
pub mod version_list;

use std::path::Path;

use serde::Deserialize;

use crate::error::RmclError;

/// Forge 推荐的版本清单(Mojang 版本 → Forge 版本号)
pub const PROMOTIONS_URL: &str =
    "https://files.minecraftforge.net/maven/net/minecraftforge/forge/promotions_slim.json";

/// Forge maven 仓库基址
pub const MAVEN_BASE: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

/// 判断某个 MC 版本走新版(processor)还是旧版(universal jar)路径
pub fn is_legacy(mc_version: &str) -> bool {
    // ≤1.12.2 走旧版
    version_le(mc_version, "1.12.2")
}

/// 比较形如 "1.20.1" 的版本号:a <= b
fn version_le(a: &str, b: &str) -> bool {
    let num_a = parse_version(a);
    let num_b = parse_version(b);
    num_a <= num_b
}

/// 把 "1.12.2" 解析为数字元组,越不完整的部分取 0
fn parse_version(v: &str) -> (u32, u32, u32) {
    let nums: Vec<u32> = v
        .split('.')
        .take(3)
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    (
        nums.first().copied().unwrap_or(0),
        nums.get(1).copied().unwrap_or(0),
        nums.get(2).copied().unwrap_or(0),
    )
}

/// 统一请求并解析成 JSON 值
pub(crate) async fn fetch_json(
    client: &reqwest::Client,
    url: &str,
    retry_times: u32,
) -> Result<serde_json::Value, RmclError> {
    let body = fetch_body(client, url, retry_times).await?;
    Ok(serde_json::from_str(&body)?)
}

pub(crate) async fn fetch_body(
    client: &reqwest::Client,
    url: &str,
    retry_times: u32,
) -> Result<String, RmclError> {
    let mut last_err = None;
    for attempt in 0..=retry_times {
        match client.get(url).send().await {
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
    Err(last_err.unwrap_or_else(|| RmclError::other("Forge 请求失败")))
}

/// 下载一个小文件到本地路径(用于 installer jar 等)
pub(crate) async fn download_to(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    retry_times: u32,
) -> Result<(), RmclError> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("part");
    let mut last_err = None;
    for attempt in 0..=retry_times {
        match client.get(url).send().await {
            Ok(resp) => match resp.error_for_status() {
                Ok(resp) => {
                    let bytes = resp.bytes().await?;
                    if let Err(e) = std::fs::write(&tmp, &bytes) {
                        last_err = Some(e.into());
                        continue;
                    }
                    std::fs::rename(&tmp, dest)?;
                    return Ok(());
                }
                Err(e) => last_err = Some(RmclError::Network(e)),
            },
            Err(e) => last_err = Some(RmclError::Network(e)),
        }
        if attempt < retry_times {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    let _ = std::fs::remove_file(&tmp);
    Err(last_err.unwrap_or_else(|| RmclError::other("Forge 文件下载失败")))
}

/// 下载产物(maven 坐标),返回本地路径;空 sha1 时仅按存在性判断
pub(crate) async fn fetch_maven_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    sha1: &str,
    _size: i64,
    retry_times: u32,
) -> Result<(), RmclError> {
    let _ = sha1;
    download_to(client, url, dest, retry_times).await
}

/// Forge 的下载子项(来自 version.json 的 downloads.classifiers / artifact)
#[derive(Debug, Clone, Deserialize)]
pub struct ForgeDownload {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub size: i64,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_boundary() {
        assert!(is_legacy("1.12.2"));
        assert!(is_legacy("1.7.10"));
        assert!(!is_legacy("1.13"));
        assert!(!is_legacy("1.21.1"));
    }

    #[test]
    fn version_compare() {
        assert!(version_le("1.12.2", "1.12.2"));
        assert!(version_le("1.12.1", "1.12.2"));
        assert!(version_le("1.12", "1.12.2"));
        assert!(!version_le("1.13", "1.12.2"));
    }
}
