//! 启动参数拼装:rules 条件解析 + token 替换

use std::path::PathBuf;

use crate::core::version::rules::{rules_allow, FeaturesCtx, RuleContext};
use crate::core::version::version_json::{Arg, VersionJson};
use crate::error::RunaError;

#[cfg(target_os = "windows")]
const CP_SEP: &str = ";";
#[cfg(not(target_os = "windows"))]
const CP_SEP: &str = ":";

const LAUNCHER_NAME: &str = "Runa";

/// 启动选项(离线 MVP)
#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub min_memory: u32,
    pub max_memory: u32,
    pub width: u32,
    pub height: u32,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            username: "Steve".into(),
            uuid: uuid::Uuid::new_v4().to_string(),
            access_token: "0".into(),
            min_memory: 1024,
            max_memory: 4096,
            width: 1280,
            height: 720,
        }
    }
}

/// 启动所需路径
#[derive(Debug, Clone)]
pub struct LaunchPaths {
    pub game_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub libraries_dir: PathBuf,
    pub version_dir: PathBuf,
    pub natives_dir: PathBuf,
}

/// 最终命令行:java_path + args(不含程序名)
#[derive(Debug, Clone)]
pub struct LaunchCommand {
    pub java_path: String,
    pub args: Vec<String>,
}

/// 拼装完整启动命令;java_path 由调用方(配置/自动检测)决定
pub fn build_launch_command(
    version: &VersionJson,
    paths: &LaunchPaths,
    opts: &LaunchOptions,
    java_path: &str,
) -> Result<LaunchCommand, RunaError> {
    let ctx = RuleContext::current(FeaturesCtx {
        has_custom_resolution: true,
        is_demo_user: false,
    });

    // 1. classpath:应用规则的 libraries artifact + client.jar
    let mut classpath: Vec<String> = Vec::new();
    for lib in &version.libraries {
        if !rules_allow(lib.rules.as_deref(), &ctx) {
            continue;
        }
        // native 库的 jar 解压到 natives 目录,不进 classpath
        if lib.natives.as_ref().is_some_and(|n| n.contains_key(ctx.os_name)) {
            continue;
        }
        if let Some(dl) = &lib.downloads {
            if let Some(artifact) = &dl.artifact {
                let path = artifact
                    .path
                    .clone()
                    .unwrap_or_else(|| lib.name.replace(':', "/") + ".jar");
                classpath.push(paths.libraries_dir.join(path).to_string_lossy().to_string());
            }
        }
    }
    classpath.push(
        paths
            .version_dir
            .join(format!("{}.jar", version.id))
            .to_string_lossy()
            .to_string(),
    );
    let classpath = classpath.join(CP_SEP);

    let tokens = TokenCtx {
        natives_directory: &paths.natives_dir.to_string_lossy(),
        classpath: &classpath,
        libraries_directory: &paths.libraries_dir.to_string_lossy(),
        version_name: &version.id,
        game_directory: &paths.game_dir.to_string_lossy(),
        assets_root: &paths.assets_dir.to_string_lossy(),
        assets_index_name: &version.asset_index.id,
        username: &opts.username,
        uuid: &opts.uuid,
        access_token: &opts.access_token,
        width: opts.width,
        height: opts.height,
        version_type: version.version_type.as_deref().unwrap_or("release"),
    };

    let mut jvm_args: Vec<String> = Vec::new();
    if opts.max_memory > 0 {
        jvm_args.push(format!("-Xmx{}M", opts.max_memory));
    }
    if opts.min_memory > 0 {
        jvm_args.push(format!("-Xms{}M", opts.min_memory));
    }

    match &version.arguments {
        Some(args) => {
            for arg in &args.jvm {
                if let Some(v) = resolve_arg(arg, &ctx) {
                    jvm_args.extend(v.into_iter().map(|s| replace_tokens(&s, &tokens)));
                }
            }
        }
        // 旧版无 arguments,手动补必要 JVM 参数
        None => {
            jvm_args.push(format!("-Djava.library.path={}", paths.natives_dir.display()));
            jvm_args.push("-cp".into());
            jvm_args.push(classpath.clone());
        }
    }

    let mut game_args: Vec<String> = Vec::new();
    match (&version.arguments, &version.minecraft_arguments) {
        (Some(args), _) => {
            for arg in &args.game {
                if let Some(v) = resolve_arg(arg, &ctx) {
                    game_args.extend(v.into_iter().map(|s| replace_tokens(&s, &tokens)));
                }
            }
        }
        (None, Some(mc)) => {
            for s in mc.split_whitespace() {
                game_args.push(replace_tokens(s, &tokens));
            }
        }
        _ => {}
    }

    let mut args = jvm_args;
    args.push(version.main_class.clone());
    args.extend(game_args);

    Ok(LaunchCommand {
        java_path: java_path.to_string(),
        args,
    })
}

fn resolve_arg(arg: &Arg, ctx: &RuleContext) -> Option<Vec<String>> {
    match arg {
        Arg::Plain(s) => Some(vec![s.clone()]),
        Arg::Conditional { rules, value } => {
            if rules_allow(Some(rules), ctx) {
                Some(value.clone().into_vec())
            } else {
                None
            }
        }
    }
}

struct TokenCtx<'a> {
    natives_directory: &'a str,
    classpath: &'a str,
    libraries_directory: &'a str,
    version_name: &'a str,
    game_directory: &'a str,
    assets_root: &'a str,
    assets_index_name: &'a str,
    username: &'a str,
    uuid: &'a str,
    access_token: &'a str,
    width: u32,
    height: u32,
    version_type: &'a str,
}

/// 官方启动器的 token 替换
fn replace_tokens(s: &str, t: &TokenCtx) -> String {
    s.replace("${natives_directory}", t.natives_directory)
        .replace("${launcher_name}", LAUNCHER_NAME)
        .replace("${launcher_version}", env!("CARGO_PKG_VERSION"))
        .replace("${classpath}", t.classpath)
        .replace("${classpath_separator}", CP_SEP)
        .replace("${library_directory}", t.libraries_directory)
        .replace("${version_name}", t.version_name)
        .replace("${game_directory}", t.game_directory)
        .replace("${assets_root}", t.assets_root)
        .replace("${game_assets}", &format!("{}/virtual/legacy", t.assets_root))
        .replace("${assets_index_name}", t.assets_index_name)
        .replace("${auth_player_name}", t.username)
        .replace("${auth_uuid}", t.uuid)
        .replace("${auth_access_token}", t.access_token)
        .replace("${auth_session}", "0")
        .replace("${user_type}", "legacy")
        .replace("${user_properties}", "{}")
        .replace("${version_type}", t.version_type)
        .replace("${clientid}", LAUNCHER_NAME)
        .replace("${auth_xuid}", "0")
        .replace("${resolution_width}", &t.width.to_string())
        .replace("${resolution_height}", &t.height.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::version::version_json::VersionJson;

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
      "assetIndex": {"id": "18", "sha1": "aabb", "size": 400000, "url": "https://example.com/18.json"},
      "downloads": {"client": {"sha1": "ccdd", "size": 25000000, "url": "https://example.com/client.jar"}},
      "libraries": [
        {"downloads": {"artifact": {"path": "com/mojang/brigadier/1.1.8/brigadier-1.1.8.jar", "sha1": "1111", "size": 100, "url": "https://example.com/b.jar"}}, "name": "com.mojang:brigadier:1.1.8"},
        {"downloads": {"artifact": {"path": "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar", "sha1": "2222", "size": 200, "url": "https://example.com/l.jar"}}, "name": "org.lwjgl:lwjgl:3.3.3", "natives": {"linux": "natives-linux"}}
      ],
      "mainClass": "net.minecraft.client.main.Main",
      "type": "release"
    }"#;

    fn paths() -> LaunchPaths {
        LaunchPaths {
            game_dir: "/game".into(),
            assets_dir: "/assets".into(),
            libraries_dir: "/libraries".into(),
            version_dir: "/versions/1.21.1".into(),
            natives_dir: "/versions/1.21.1/natives".into(),
        }
    }

    #[test]
    fn builds_command_with_tokens() {
        let version: VersionJson = serde_json::from_str(SAMPLE).unwrap();
        let opts = LaunchOptions::default();
        let cmd = build_launch_command(&version, &paths(), &opts, "java").unwrap();

        let joined = cmd.args.join(" ");
        assert!(cmd.args[0].starts_with("-Xmx"), "首参应为 -Xmx,实际 {}", cmd.args[0]);
        // token 替换生效
        assert!(joined.contains("--username Steve"));
        assert!(joined.contains("--gameDir /game"));
        assert!(joined.contains("--assetsDir /assets"));
        assert!(joined.contains("--assetIndex 18"));
        assert!(joined.contains("-Djava.library.path=/versions/1.21.1/natives"));
        assert!(joined.contains("--width 1280 --height 720"));
        // classpath 包含 client.jar 与库
        assert!(joined.contains("brigadier-1.1.8.jar"));
        assert!(joined.contains("/versions/1.21.1/1.21.1.jar"));
        // main class 在参数中间
        assert!(joined.contains("net.minecraft.client.main.Main"));
        // 残留 token 不应存在
        assert!(!joined.contains("${"));
    }

    #[test]
    fn native_library_not_in_classpath() {
        let version: VersionJson = serde_json::from_str(SAMPLE).unwrap();
        let opts = LaunchOptions::default();
        let cmd = build_launch_command(&version, &paths(), &opts, "java").unwrap();
        let joined = cmd.args.join(" ");
        assert!(joined.contains("brigadier-1.1.8.jar"));
        assert!(!joined.contains("lwjgl-3.3.3.jar"), "native 库不应进 classpath");
    }
}
