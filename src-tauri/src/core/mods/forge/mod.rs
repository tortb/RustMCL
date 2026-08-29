//! Forge 加载器安装:旧版(≤1.12.2 universal jar)与新版(≥1.13 处理器)两条路径
//! 目录: core/mods/forge/

// 待 T5.4/T5.6 接线前,部分内部函数与常量暂未被命令层消费,不告警
#![allow(dead_code)]

pub mod installer;
pub mod legacy;
pub mod processor;
pub mod profile_merge;
pub mod version_list;

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::core::version::version_json::VersionJson;
use crate::error::RmclError;

use self::installer::InstallerContents;

/// Forge 推荐的版本清单(Mojang 版本 → Forge 版本号)
pub const PROMOTIONS_URL: &str =
    "https://files.minecraftforge.net/maven/net/minecraftforge/forge/promotions_slim.json";

/// Forge maven 仓库基址
pub const MAVEN_BASE: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

/// 合并后版本的 id(同时用作缓存文件名)
pub fn forge_merged_id(mc_version: &str, forge_version: &str) -> String {
    format!("forge-{mc_version}-{forge_version}")
}

/// installer 解压/工作目录
pub fn forge_work_dir(data_dir: &Path, mc_version: &str, forge_version: &str) -> PathBuf {
    data_dir
        .join("forge_work")
        .join(format!("{mc_version}-{forge_version}"))
}

/// 解析并(如有需要)生成 Forge 合并后的 version.json(带缓存)。
/// 若 cache/versions/<merged_id>.json 已存在则直接返回;否则下载 installer、与原版合并后写缓存。
/// 首次会下载 installer jar 到 forge_work 目录(供处理器阶段复用)。
pub async fn resolve_forge_version(
    client: &reqwest::Client,
    data_dir: &Path,
    mc_version: &str,
    forge_version: &str,
    retry_times: u32,
) -> Result<VersionJson, RmclError> {
    let forge_version = forge_version.trim();
    if forge_version.is_empty() {
        return Err(RmclError::other("未指定 Forge 版本"));
    }
    let merged_id = forge_merged_id(mc_version, forge_version);
    let merged_cache = data_dir
        .join("cache")
        .join("versions")
        .join(format!("{merged_id}.json"));
    if let Some(v) = load_cached(&merged_cache) {
        return Ok(v);
    }

    let work = forge_work_dir(data_dir, mc_version, forge_version);
    let jar = installer::download_installer(client, mc_version, forge_version, &work, retry_times).await?;
    let contents = installer::extract_installer(&jar, mc_version, forge_version)?;
    let forge_json = contents
        .version_json
        .as_ref()
        .or(contents.install_profile.as_ref())
        .ok_or_else(|| RmclError::other("Forge installer 缺少 version.json / install_profile.json"))?;
    let vanilla = crate::core::loader::fetch_vanilla(client, data_dir, mc_version, retry_times).await?;
    let mut merged = profile_merge::merge_forge(&vanilla, forge_json)?;
    // 合并后版本 id 取 forge-<mc>-<forge>,保证 client.jar、缓存文件名、launch classpath 一致
    merged.id = merged_id.clone();

    if let Some(parent) = merged_cache.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&merged_cache, serde_json::to_string(&merged)?)?;
    Ok(merged)
}

/// 运行 Forge 新版所需的 processors(仅 ≥1.13;旧版走 legacy)。
/// `{MINECRAFT_JAR}` 取 versions/<merged_id>/<merged_id>.jar(client.jar 本地路径)。
pub fn run_installer_processors(
    contents: &InstallerContents,
    data_dir: &Path,
    mc_version: &str,
    forge_version: &str,
    java_path: &str,
) -> Result<(), RmclError> {
    if is_legacy(mc_version) {
        return Ok(());
    }
    let work = forge_work_dir(data_dir, mc_version, forge_version);
    let libraries_dir = data_dir.join("libraries");
    let merged_id = forge_merged_id(mc_version, forge_version);
    let minecraft_jar = data_dir
        .join("versions")
        .join(&merged_id)
        .join(format!("{merged_id}.jar"));
    processor::run_processors(contents, &work, &libraries_dir, java_path, &minecraft_jar)
}

fn load_cached(path: &Path) -> Option<VersionJson> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 从 install_profile.json 提取处理器工具链库(processors 的 classpath/jar 依赖),
/// 转成下载项(dest 落到 libraries/<path>)。库条目通常自带 downloads.artifact。
pub fn processor_library_items(contents: &InstallerContents, data_dir: &Path) -> Vec<crate::core::downloader::DownloadItem> {
    let Some(profile) = contents.install_profile.as_ref() else {
        return Vec::new();
    };
    let Some(libs) = profile.get("libraries").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let libraries_dir = data_dir.join("libraries");
    libs.iter()
        .filter_map(|lib| {
            let dl = lib.get("downloads")?.get("artifact")?;
            let url = dl.get("url")?.as_str()?.to_string();
            let sha1 = dl.get("sha1").and_then(|s| s.as_str()).unwrap_or("").to_string();
            let size = dl.get("size").and_then(|s| s.as_i64()).unwrap_or(0);
            let path = dl.get("path").and_then(|s| s.as_str()).unwrap_or("");
            let dest = if path.is_empty() {
                let name = lib.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let rel = processor::maven_rel_path(name).unwrap_or_else(|| name.to_string());
                libraries_dir.join(rel)
            } else {
                libraries_dir.join(path)
            };
            Some(crate::core::downloader::DownloadItem { url, sha1, size, dest })
        })
        .collect()
}

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

/// 统一请求并返回文本
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

    /// 快速:加载已下载的真实 installer 并打印处理器变量与命令(不执行)。
    /// 运行:cargo test --lib forge_preview -- --ignored --nocapture
    #[ignore]
    #[test]
    fn forge_live_preview_commands() {
        let mc = "1.20.1";
        let fv = "47.2.0";
        let data_dir = std::path::PathBuf::from("/tmp/forge-verify/run");
        let work = forge_work_dir(&data_dir, mc, fv);
        let jar = work.join("forge-installer.jar");
        let contents = installer::extract_installer(&jar, mc, fv).unwrap();
        print_preview(&contents, &data_dir, mc, fv);
    }

    /// 真实环境验证:下载真实 Forge 1.20.1-47.2.0 installer,下载依赖,运行全部 java 处理器。
    /// 默认忽略(需网络 + Java)。运行:cargo test --lib forge_live -- --ignored
    #[ignore]
    #[tokio::test]
    async fn forge_live_processors_on_real_installer() {
        let client = reqwest::Client::new();
        let mc = "1.20.1";
        let fv = "47.2.0";
        let data_dir = std::path::PathBuf::from("/tmp/forge-verify/run");
        // 复用已缓存的下载(installer/依赖),便于重复验证;如需完全干净可取消下一行注释
        // let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(&data_dir).unwrap();

        // 1. 下载 installer 并解压(内存 json + 全量文件到工作目录)
        let work = forge_work_dir(&data_dir, mc, fv);
        let jar = installer::download_installer(&client, mc, fv, &work, 3).await.unwrap();
        let contents = installer::extract_installer(&jar, mc, fv).unwrap();
        installer::extract_installer_files(&jar, &work).unwrap();

        // 2. 合并 version.json(幂等,复用已下载 installer)
        let version = resolve_forge_version(&client, &data_dir, mc, fv, 3).await.unwrap();
        assert_eq!(version.id, forge_merged_id(mc, fv), "合并版本 id 应为 forge-<mc>-<forge>");

        // 3. 下载依赖(client.jar + libraries + natives + 处理器工具链库)
        let ctx = crate::core::version::rules::RuleContext::current(
            crate::core::version::rules::FeaturesCtx::default(),
        );
        let vdir = data_dir.join("versions").join(&version.id);
        let libs = data_dir.join("libraries");
        let mut items = vec![crate::core::downloader::library::client_download_item(&version, &vdir)];
        items.extend(crate::core::downloader::library::library_items(&version, &ctx, &libs));
        items.extend(crate::core::downloader::library::native_items(&version, &ctx, &libs));
        items.extend(processor_library_items(&contents, &data_dir));
        crate::core::downloader::download_many(&client, items, 4, 2, |_| {}).await.unwrap();

        // 4. 运行全部 java 处理器
        let java = std::env::var("JAVA").unwrap_or_else(|_| "java".into());
        run_installer_processors(&contents, &data_dir, mc, fv, &java).unwrap();

        // 5. 校验关键产物:patched client jar(处理器最终产出)
        let patched = libs.join("net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-client.jar");
        assert!(patched.exists(), "patched client jar 应已生成: {}", patched.display());
    }

    /// 打印处理器变量与命令(仅用于真实环境验证)。
    fn print_preview(contents: &InstallerContents, data_dir: &Path, mc: &str, fv: &str) {
        let work = forge_work_dir(data_dir, mc, fv);
        let libs = data_dir.join("libraries");
        let merged_id = forge_merged_id(mc, fv);
        let mcjar = data_dir.join("versions").join(&merged_id).join(format!("{merged_id}.jar"));

        let vars = super::processor::build_processor_vars(contents, &work, &libs, &mcjar);
        let mut keys: Vec<_> = vars.keys().cloned().collect();
        keys.sort();
        println!("=== 变量表 ===");
        for k in keys {
            println!("  {k} = {}", vars.get(&k).unwrap());
        }
        let preview = super::processor::build_processors_preview(contents, &work, &libs, &mcjar, "java").unwrap();
        println!("=== 处理器命令 ===");
        for (i, cmd) in &preview {
            println!("--- processor[{i}] ---");
            for a in cmd {
                println!("  {a}");
            }
        }
    }
}
