//! T5.3 合并 version.json:以原版为基础,叠加 Forge 的 libraries/mainClass/arguments

use std::collections::HashSet;

use serde_json::Value;

use crate::core::version::version_json::{
    Arg, Arguments, Download, Library, LibraryDownloads, VersionJson,
};
use crate::error::RmclError;

/// 把 Forge 侧 JSON(version.json 或 install_profile.json)合并进原版 version.json
/// - mainClass 以 Forge 为准
/// - arguments.jvm/game 追加合并
/// - libraries 去重(同 groupId:artifactId 以 Forge 侧覆盖),maven 库补齐 url 前缀
pub fn merge_forge(vanilla: &VersionJson, forge: &Value) -> Result<VersionJson, RmclError> {
    let mut merged = vanilla.clone();

    if let Some(mc) = forge.get("mainClass").and_then(Value::as_str) {
        merged.main_class = mc.to_string();
    }

    append_arguments(&mut merged, forge);

    // 收集 forge 侧 libraries
    if let Some(libs) = forge.get("libraries").and_then(Value::as_array) {
        let mut seen: HashSet<String> = dedupe_keys(&merged.libraries);
        for lib in libs {
            if let Some(l) = to_library(lib) {
                let key = library_key(&l.name);
                // 已存在(vanilla 或之前 forge)则覆盖
                if seen.contains(&key) {
                    if let Some(pos) = merged.libraries.iter().position(|x| library_key(&x.name) == key) {
                        merged.libraries[pos] = l;
                    }
                } else {
                    seen.insert(key);
                    merged.libraries.push(l);
                }
            }
        }
    }

    Ok(merged)
}

/// 追加 forge 的 arguments.game/jvm(过滤掉与 vanilla 完全相同的 plain 项)
fn append_arguments(merged: &mut VersionJson, forge: &Value) {
    let Some(fargs) = forge.get("arguments") else { return };
    if let Some(fargs) = fargs.as_object() {
        if merged.arguments.is_none() {
            merged.arguments = Some(Arguments {
                game: Vec::new(),
                jvm: Vec::new(),
            });
        }
        let args = merged.arguments.as_mut().unwrap();
        if let Some(game) = fargs.get("game").and_then(Value::as_array) {
            args.game.extend(parse_args(game));
        }
        if let Some(jvm) = fargs.get("jvm").and_then(Value::as_array) {
            args.jvm.extend(parse_args(jvm));
        }
    }
}

fn parse_args(arr: &[Value]) -> Vec<Arg> {
    arr.iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect()
}

/// 计算去重 key:name 的 groupId:artifactId
fn library_key(name: &str) -> String {
    let mut parts = name.split(':');
    match (parts.next(), parts.next()) {
        (Some(g), Some(a)) => format!("{g}:{a}"),
        _ => name.to_string(),
    }
}

fn dedupe_keys(libs: &[Library]) -> HashSet<String> {
    libs.iter().map(|l| library_key(&l.name)).collect()
}

/// 把 forge library(可能是 maven name+url,或带 downloads)转为 Library
fn to_library(value: &Value) -> Option<Library> {
    let name = value.get("name")?.as_str()?.to_string();

    // 直接带 downloads.artifact
    if let Some(dl) = value.get("downloads") {
        if let Some(artifact) = dl.get("artifact") {
            let download: Download = serde_json::from_value(artifact.clone()).ok()?;
            return Some(Library {
                name,
                rules: None,
                natives: None,
                extract: None,
                downloads: Some(LibraryDownloads {
                    artifact: Some(download),
                    classifiers: None,
                }),
            });
        }
    }

    // maven name + url 形式
    let url = value.get("url").and_then(Value::as_str).unwrap_or("");
    maven_to_library(&name, url)
}

/// maven 坐标 → 带 artifact 的 Library(sha1 未知留空,下载时跳过校验)
fn maven_to_library(name: &str, base_url: &str) -> Option<Library> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
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
    use crate::core::version::version_json::VersionJson;

    fn vanilla() -> VersionJson {
        serde_json::from_str(
            r#"{
              "id": "1.20.1",
              "arguments": {"game": ["--username", "${auth_player_name}"], "jvm": ["-cp", "${classpath}"]},
              "assetIndex": {"id": "18", "sha1": "aabb", "size": 1, "url": "https://example.com/18.json"},
              "downloads": {"client": {"sha1": "ccdd", "size": 1, "url": "https://example.com/client.jar"}},
              "libraries": [{"name": "net.minecraftforge:forge:1.20.1-47.2.0", "downloads": {"artifact": {"path": "net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0.jar", "sha1": "1111", "size": 1, "url": "https://maven/forge.jar"}}}],
              "mainClass": "net.minecraft.client.main.Main",
              "type": "release"
            }"#,
        )
        .unwrap()
    }

    const FORGE: &str = r#"{
      "id": "1.20.1-forge-47.2.0",
      "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
      "arguments": {
        "game": ["--launchTarget", "forgeclient"],
        "jvm": ["-Dforge.logging.level=info"]
      },
      "libraries": [
        {"name": "net.minecraftforge:forge:1.20.1-47.2.0", "url": "https://maven.minecraftforge.net/"},
        {"name": "org.apache.logging.log4j:log4j-core:2.20.0", "downloads": {"artifact": {"path": "org/apache/logging/log4j/log4j-core/2.20.0/log4j-core-2.20.0.jar", "sha1": "2222", "size": 100, "url": "https://maven/launcher.jar"}}}
      ]
    }"#;

    #[test]
    fn replaces_main_class_and_appends_args() {
        let forge: Value = serde_json::from_str(FORGE).unwrap();
        let merged = merge_forge(&vanilla(), &forge).unwrap();
        assert_eq!(
            merged.main_class,
            "cpw.mods.bootstraplauncher.BootstrapLauncher"
        );
        let args = merged.arguments.as_ref().unwrap();
        // vanilla jvm 2(-cp, ${classpath}) + forge jvm 1 = 3
        assert_eq!(args.jvm.len(), 3);
        // vanilla game 2(--username, ${auth_player_name}) + forge game 2 = 4
        assert_eq!(args.game.len(), 4);
    }

    #[test]
    fn dedupes_libraries_keeping_forge_wins() {
        let forge: Value = serde_json::from_str(FORGE).unwrap();
        let merged = merge_forge(&vanilla(), &forge).unwrap();
        // vanilla 已有 net.minecraftforge:forge:...(带 downloads),forge side 用 maven url 覆盖
        // 去重后应为 2 个 library(forge + log4j)
        assert_eq!(merged.libraries.len(), 2);
        let forge_lib = &merged.libraries[0];
        // 以 forge 侧 maven url 为准
        let artifact = forge_lib.downloads.as_ref().unwrap().artifact.as_ref().unwrap();
        assert_eq!(
            artifact.url,
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0.jar"
        );
    }

    #[test]
    fn keeps_unique_library_ordering_stable() {
        let forge: Value = serde_json::from_str(FORGE).unwrap();
        let merged = merge_forge(&vanilla(), &forge).unwrap();
        // log4j 是新增的
        assert!(merged
            .libraries
            .iter()
            .any(|l| l.name == "org.apache.logging.log4j:log4j-core:2.20.0"));
    }
}
