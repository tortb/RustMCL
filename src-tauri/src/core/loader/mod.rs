//! Fabric / Quilt 加载器安装:
//! meta API 拉取 profile → 与原版 version.json 合并成统一版本(缓存到 cache/versions/<merged_id>.json)
//! 合并后与 vanilla 走同一条下载/启动链路,前端无需感知差异

use std::path::Path;

use serde_json::Value;

use crate::core::version::manifest;
use crate::core::version::version_json::{
    fetch_version_json, Download, Library, LibraryDownloads, VersionJson,
};
use crate::error::RmclError;

/// 支持的加载器与对应 meta 仓库
struct LoaderMeta {
    name: &'static str,
    base: &'static str,
}

const FABRIC: LoaderMeta = LoaderMeta {
    name: "fabric",
    base: "https://meta.fabricmc.net/v2",
};

const QUILT: LoaderMeta = LoaderMeta {
    name: "quilt",
    base: "https://meta.quiltmc.org/v3",
};

/// 合并后版本的 id(同时用作缓存文件名)
pub fn merged_version_id(mc_version: &str, loader: &str, loader_version: &str) -> String {
    match loader {
        "fabric" | "quilt" => format!("{loader}-loader-{loader_version}-{mc_version}"),
        "forge" => format!("forge-{mc_version}-{loader_version}"),
        _ => mc_version.to_string(),
    }
}

/// 解析最终 version.json:vanilla 直接返回原版;fabric/quilt/forge 返回合并结果(带缓存)
pub async fn resolve_version(
    client: &reqwest::Client,
    data_dir: &Path,
    mc_version: &str,
    loader: &str,
    loader_version: &str,
    retry_times: u32,
) -> Result<VersionJson, RmclError> {
    let meta = match loader {
        "fabric" => FABRIC,
        "quilt" => QUILT,
        // Forge:由 mods/forge 解析并合并(下载 installer + 与原版合并,带缓存)
        "forge" => {
            return crate::core::mods::forge::resolve_forge_version(
                client,
                data_dir,
                mc_version,
                loader_version,
                retry_times,
            )
            .await
        }
        // vanilla:直接读原版 version.json
        "" | "vanilla" => return fetch_vanilla(client, data_dir, mc_version, retry_times).await,
        other => {
            return Err(RmclError::other(format!("暂不支持加载器: {other}")));
        }
    };

    let lv = if loader_version.trim().is_empty() {
        resolve_latest_loader(client, &meta, mc_version, retry_times).await?
    } else {
        loader_version.trim().to_string()
    };

    let merged_id = merged_version_id(mc_version, meta.name, &lv);
    let merged_cache = data_dir
        .join("cache")
        .join("versions")
        .join(format!("{merged_id}.json"));
    if let Some(v) = load_cached(&merged_cache) {
        return Ok(v);
    }

    let vanilla = fetch_vanilla(client, data_dir, mc_version, retry_times).await?;
    let profile = fetch_profile(client, &meta, mc_version, &lv, retry_times).await?;
    let merged = merge_loader(vanilla, &profile)?;
    if let Some(parent) = merged_cache.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&merged_cache, serde_json::to_string(&merged)?)?;
    Ok(merged)
}

fn load_cached(path: &Path) -> Option<VersionJson> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 原版 version.json(复用清单 + 版本缓存)
pub(crate) async fn fetch_vanilla(
    client: &reqwest::Client,
    data_dir: &Path,
    mc_version: &str,
    retry_times: u32,
) -> Result<VersionJson, RmclError> {
    let manifest_cache = data_dir.join("cache").join("version_manifest_v2.json");
    let manifest = manifest::get_manifest(client, &manifest_cache, false, retry_times).await?;
    let info = manifest
        .versions
        .iter()
        .find(|v| v.id == mc_version)
        .ok_or_else(|| RmclError::other(format!("版本清单中不存在 {mc_version}")))?;
    let vj_cache = data_dir
        .join("cache")
        .join("versions")
        .join(format!("{mc_version}.json"));
    fetch_version_json(client, &info.url, &vj_cache, retry_times).await
}

/// 供命令层使用:按名称(fabric/quilt)解析最新加载器版本
pub async fn latest_loader_version(
    client: &reqwest::Client,
    loader_name: &str,
    mc_version: &str,
    retry_times: u32,
) -> Result<String, RmclError> {
    let meta = match loader_name {
        "fabric" => FABRIC,
        "quilt" => QUILT,
        other => return Err(RmclError::other(format!("暂不支持加载器: {other}"))),
    };
    resolve_latest_loader(client, &meta, mc_version, retry_times).await
}

/// 拉取加载器版本列表,取第一个可用版本(meta 按新→旧排序)
async fn resolve_latest_loader(
    client: &reqwest::Client,
    meta: &LoaderMeta,
    mc_version: &str,
    retry_times: u32,
) -> Result<String, RmclError> {
    let url = format!("{}/versions/loader/{mc_version}", meta.base);
    let body = fetch_body(client, &url, retry_times).await?;
    let list: Vec<Value> = serde_json::from_str(&body)?;
    for item in &list {
        if let Some(v) = item
            .get("loader")
            .and_then(|l| l.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(v.to_string());
        }
    }
    Err(RmclError::other(format!(
        "{} meta 未返回可用加载器(MC {mc_version})",
        meta.name
    )))
}

/// 拉取加载器 profile(json 文本)
async fn fetch_profile(
    client: &reqwest::Client,
    meta: &LoaderMeta,
    mc_version: &str,
    loader_version: &str,
    retry_times: u32,
) -> Result<Value, RmclError> {
    let url = format!(
        "{}/versions/loader/{mc_version}/{loader_version}/profile/json",
        meta.base
    );
    let body = fetch_body(client, &url, retry_times).await?;
    Ok(serde_json::from_str(&body)?)
}

async fn fetch_body(
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
    Err(last_err.unwrap_or_else(|| RmclError::other("拉取加载器元数据失败")))
}

/// 合并规则:
/// - mainClass 取 profile 的
/// - arguments 的 game/jvm 追加到原版后面
/// - profile 的 maven 库(仅 name+url,无 sha1)转换后追加到 libraries
/// - 其余(assetIndex/downloads/javaVersion)沿用原版
fn merge_loader(mut vanilla: VersionJson, profile: &Value) -> Result<VersionJson, RmclError> {
    if let Some(mc) = profile.get("mainClass").and_then(|v| v.as_str()) {
        vanilla.main_class = mc.to_string();
    }

    if let Some(args) = profile.get("arguments") {
        if let Some(game) = args.get("game").and_then(|v| v.as_array()) {
            if let Some(vargs) = &mut vanilla.arguments {
                vargs
                    .game
                    .extend(game.iter().filter_map(|a| serde_json::from_value(a.clone()).ok()));
            }
        }
        if let Some(jvm) = args.get("jvm").and_then(|v| v.as_array()) {
            if let Some(vargs) = &mut vanilla.arguments {
                vargs
                    .jvm
                    .extend(jvm.iter().filter_map(|a| serde_json::from_value(a.clone()).ok()));
            }
        }
    }

    if let Some(libs) = profile.get("libraries").and_then(|v| v.as_array()) {
        for lib in libs {
            let name = lib.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let url = lib.get("url").and_then(|v| v.as_str()).unwrap_or_default();
            if let Some(l) = maven_to_library(name, url) {
                vanilla.libraries.push(l);
            }
        }
    }

    Ok(vanilla)
}

/// maven 坐标 → 带 downloads 的 Library;sha1/size 未知,下载时跳过校验
/// name 形如 net.fabricmc:fabric-loader:0.16.9(:classifier 可选)
fn maven_to_library(name: &str, base_url: &str) -> Option<Library> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 || parts[1].is_empty() || parts[2].is_empty() {
        return None;
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let classifier = parts.get(3).copied().filter(|c| !c.is_empty());
    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    let path = format!("{}/{artifact}/{version}/{file}", group.replace('.', "/"));
    let url = if base_url.is_empty() {
        String::new()
    } else {
        let sep = if base_url.ends_with('/') { "" } else { "/" };
        format!("{base_url}{sep}{path}")
    };
    Some(Library {
        name: name.to_string(),
        rules: None,
        natives: None,
        extract: None,
        downloads: Some(LibraryDownloads {
            artifact: Some(Download {
                path: Some(path),
                sha1: String::new(),
                size: 0,
                url,
            }),
            classifiers: None,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::version::version_json::Arguments;

    /// Fabric 1.21.1 profile 片段
    const PROFILE: &str = r#"{
      "id": "fabric-loader-0.16.9-1.21.1",
      "inheritsFrom": "1.21.1",
      "releaseTime": "2025-01-01T00:00:00+00:00",
      "time": "2025-01-01T00:00:00+00:00",
      "type": "release",
      "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
      "arguments": {
        "game": [],
        "jvm": ["-DFabricMcEmu=net.fabricmc.loader.impl.launch.knot.KnotClient"]
      },
      "libraries": [
        {"name": "net.fabricmc:intermediary:1.21.1", "url": "https://maven.fabricmc.net/"},
        {"name": "net.fabricmc:fabric-loader:0.16.9", "url": "https://maven.fabricmc.net/"}
      ]
    }"#;

    fn vanilla() -> VersionJson {
        serde_json::from_str(
            r#"{
              "id": "1.21.1",
              "arguments": {"game": ["--username", "${auth_player_name}"], "jvm": ["-cp", "${classpath}"]},
              "assetIndex": {"id": "18", "sha1": "aabb", "size": 1, "url": "https://example.com/18.json"},
              "downloads": {"client": {"sha1": "ccdd", "size": 1, "url": "https://example.com/client.jar"}},
              "libraries": [{"name": "com.mojang:brigadier:1.1.8", "downloads": {"artifact": {"path": "com/mojang/brigadier/1.1.8/brigadier-1.1.8.jar", "sha1": "1111", "size": 1, "url": "https://example.com/b.jar"}}}],
              "mainClass": "net.minecraft.client.main.Main",
              "type": "release"
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn merged_version_id_formats() {
        assert_eq!(
            merged_version_id("1.21.1", "fabric", "0.16.9"),
            "fabric-loader-0.16.9-1.21.1"
        );
        assert_eq!(merged_version_id("1.21.1", "vanilla", ""), "1.21.1");
    }

    #[test]
    fn merge_replaces_main_class_and_appends_args_and_libs() {
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        let merged = merge_loader(vanilla(), &profile).unwrap();

        assert_eq!(
            merged.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        // jvm 参数追加在最后
        let args: &Arguments = merged.arguments.as_ref().unwrap();
        assert_eq!(args.jvm.len(), 3);
        assert_eq!(args.jvm[2].plain().map(|s| s.as_str()), Some("-DFabricMcEmu=net.fabricmc.loader.impl.launch.knot.KnotClient"));
        assert_eq!(merged.libraries.len(), 3);

        // maven 库的 path/url 推导正确
        let fabric_loader = &merged.libraries[2];
        let artifact = fabric_loader.downloads.as_ref().unwrap().artifact.as_ref().unwrap();
        assert_eq!(
            artifact.path.as_deref(),
            Some("net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar")
        );
        assert_eq!(
            artifact.url,
            "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar"
        );
        assert!(artifact.sha1.is_empty(), "maven 库无 sha1,应留空以便跳过校验");
    }

    #[test]
    fn maven_to_library_with_classifier() {
        let lib = maven_to_library("org.lwjgl:lwjgl:3.3.3:natives-linux", "https://maven/").unwrap();
        let artifact = lib.downloads.unwrap().artifact.unwrap();
        assert_eq!(
            artifact.path.as_deref(),
            Some("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar")
        );
        assert_eq!(artifact.url, "https://maven/org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar");
    }

    #[test]
    fn invalid_maven_name_returns_none() {
        assert!(maven_to_library("bad-name", "https://maven/").is_none());
    }
}
