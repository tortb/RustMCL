//! Mojang 单个版本的 version.json 结构解析与拉取

// 部分字段(如 javaVersion/logging)为解析 Mojang JSON 所必需,
// 由后续阶段(M2 收尾的 Java 版本匹配、M7 日志系统等)消费,未用时不告警
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::RunaError;

use super::rules::Rule;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "type", default)]
    pub version_type: Option<String>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// 旧版(1.13 之前)用字符串形式
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    pub asset_index: AssetIndex,
    pub downloads: Downloads,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
    pub libraries: Vec<Library>,
    pub main_class: String,
    #[serde(default)]
    pub logging: Option<Logging>,
}

impl VersionJson {
    /// 游戏参数列表(新版 arguments.game 或旧版 minecraftArguments)
    pub fn game_arg_strings(&self) -> Option<Vec<String>> {
        match (&self.arguments, &self.minecraft_arguments) {
            (Some(args), _) => Some(
                args.game
                    .iter()
                    .filter_map(|a| a.plain())
                    .cloned()
                    .collect(),
            ),
            (None, Some(s)) => Some(s.split_whitespace().map(|s| s.to_string()).collect()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Arg>,
    #[serde(default)]
    pub jvm: Vec<Arg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Arg {
    Plain(String),
    Conditional {
        rules: Vec<Rule>,
        value: OneOrMore,
    },
}

impl Arg {
    pub fn plain(&self) -> Option<&String> {
        match self {
            Arg::Plain(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMore {
    One(String),
    Many(Vec<String>),
}

impl OneOrMore {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            OneOrMore::One(s) => vec![s],
            OneOrMore::Many(v) => v,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Downloads {
    pub client: Download,
    #[serde(default)]
    pub server: Option<Download>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    #[serde(default)]
    pub path: Option<String>,
    pub sha1: String,
    pub size: i64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: i64,
    #[serde(default)]
    pub total_size: Option<i64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    /// 如 { "linux": "natives-linux", "osx": "natives-macos", "windows": "natives-windows" }
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extract: Option<Extract>,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<Download>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Download>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logging {
    #[serde(default)]
    pub client: Option<LoggingClient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingClient {
    pub argument: String,
    pub file: Download,
}

/// assets index(json 文件本体)
#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexFile {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: i64,
}

/// 拉取 version.json(带本地缓存:cache/versions/<id>.json,缓存原始 JSON 文本)
pub async fn fetch_version_json(
    client: &reqwest::Client,
    url: &str,
    cache_path: &Path,
    retry_times: u32,
) -> Result<VersionJson, RunaError> {
    if let Some(v) = load_cache(cache_path) {
        return Ok(v);
    }
    let body = fetch_body(client, url, retry_times).await?;
    save_cache(cache_path, &body)?;
    Ok(serde_json::from_str(&body)?)
}

async fn fetch_body(client: &reqwest::Client, url: &str, retry_times: u32) -> Result<String, RunaError> {
    let mut last_err = None;
    for attempt in 0..=retry_times {
        match client.get(url).send().await {
            Ok(resp) => match resp.error_for_status() {
                Ok(resp) => match resp.text().await {
                    Ok(body) => return Ok(body),
                    Err(e) => last_err = Some(RunaError::Network(e)),
                },
                Err(e) => last_err = Some(RunaError::Network(e)),
            },
            Err(e) => last_err = Some(RunaError::Network(e)),
        }
        if attempt < retry_times {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    Err(last_err.unwrap_or_else(|| RunaError::other("拉取 version.json 失败")))
}

fn load_cache(path: &Path) -> Option<VersionJson> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_cache(path: &Path, body: &str) -> Result<(), RunaError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1.21.1 version.json 关键片段(去掉大段 libraries)
    const SAMPLE: &str = r#"{
      "id": "1.21.1",
      "arguments": {
        "game": [
          "--username", "${auth_player_name}",
          "--version", "${version_name}",
          "--gameDir", "${game_directory}",
          "--assetsDir", "${assets_root}",
          "--assetIndex", "${assets_index_name}",
          {"rules": [{"action": "allow", "features": {"has_custom_resolution": true}}], "value": ["--width", "${resolution_width}", "--height", "${resolution_height}"]}
        ],
        "jvm": [
          "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_jvmwarnings.db",
          {"rules": [{"action": "allow", "os": {"name": "osx"}}], "value": ["-XstartOnFirstThread"]},
          "-Djava.library.path=${natives_directory}",
          "-cp", "${classpath}"
        ]
      },
      "assetIndex": {"id": "18", "sha1": "aabb", "size": 400000, "totalSize": 600000000, "url": "https://example.com/18.json"},
      "downloads": {
        "client": {"sha1": "ccdd", "size": 25000000, "url": "https://example.com/client.jar"},
        "client_mappings": {"sha1": "eeff", "size": 1000, "url": "https://example.com/client.txt"}
      },
      "javaVersion": {"component": "java-runtime-delta", "majorVersion": 21},
      "libraries": [
        {"downloads": {"artifact": {"path": "com/mojang/brigadier/1.1.8/brigadier-1.1.8.jar", "sha1": "1111", "size": 100, "url": "https://example.com/b.jar"}}, "name": "com.mojang:brigadier:1.1.8"},
        {"downloads": {"artifact": {"path": "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar", "sha1": "2222", "size": 200, "url": "https://example.com/l.jar"}, "classifiers": {"natives-linux": {"path": "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar", "sha1": "3333", "size": 300, "url": "https://example.com/ln.jar"}}}, "name": "org.lwjgl:lwjgl:3.3.3", "natives": {"linux": "natives-linux", "osx": "natives-macos", "windows": "natives-windows"}, "extract": {"exclude": ["META-INF/"]}}
      ],
      "mainClass": "net.minecraft.client.main.Main",
      "minimumLauncherVersion": 21,
      "releaseTime": "2024-08-08T09:31:07+00:00",
      "time": "2024-08-08T09:44:16+00:00",
      "type": "release"
    }"#;

    #[test]
    fn parses_full_version_json() {
        let v: VersionJson = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(v.id, "1.21.1".to_string());
        assert_eq!(v.main_class, "net.minecraft.client.main.Main");
        assert_eq!(v.java_version.as_ref().unwrap().major_version, 21);
        assert_eq!(v.libraries.len(), 2);
        assert_eq!(v.asset_index.id, "18");
        let args = v.arguments.as_ref().unwrap();
        assert_eq!(args.game.len(), 11);
        assert_eq!(args.jvm.len(), 5);
        assert_eq!(v.version_type.as_deref(), Some("release"));
    }

    #[test]
    fn conditional_arg_has_rules() {
        let v: VersionJson = serde_json::from_str(SAMPLE).unwrap();
        let args = v.arguments.as_ref().unwrap();
        let cond = &args.game[10];
        match cond {
            Arg::Conditional { rules, .. } => assert_eq!(rules.len(), 1),
            Arg::Plain(_) => panic!("应为条件参数"),
        }
    }

    #[test]
    fn library_natives_and_extract() {
        let v: VersionJson = serde_json::from_str(SAMPLE).unwrap();
        let lwjgl = &v.libraries[1];
        let natives = lwjgl.natives.as_ref().unwrap();
        assert_eq!(natives.get("linux").map(|s| s.as_str()), Some("natives-linux"));
        assert_eq!(lwjgl.extract.as_ref().unwrap().exclude, vec!["META-INF/"]);
        let classifiers = lwjgl.downloads.as_ref().unwrap().classifiers.as_ref().unwrap();
        assert!(classifiers.contains_key("natives-linux"));
    }
}
