//! T5.4 Processors 执行引擎:变量替换、顺序执行 `java -cp <classpath> <mainClass> <args>`、outputs 校验
//!
//! 依赖说明:processor 的 classpath 中的 maven 库需由上层安装流程预先下载到 libraries_dir,
//! 本模块只负责「解析 → 构建命令 → 执行 → 校验产出」,并上报可读的错误(第几个 processor / jar / stderr)。
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::core::downloader::sha1_of;
use crate::error::RmclError;

use super::installer::InstallerContents;

#[cfg(target_os = "windows")]
const CP_SEP: &str = ";";
#[cfg(not(target_os = "windows"))]
const CP_SEP: &str = ":";

/// 单个处理器
#[derive(Debug, Clone)]
pub struct Processor {
    pub jar: String,
    pub classpath: Vec<String>,
    pub args: Vec<String>,
    /// 适用的安装侧("client" / "server");为空表示通用
    pub sides: Vec<String>,
    /// 产出校验:文件相对安装根的路径 → 期望 SHA1
    pub outputs: HashMap<String, String>,
}

/// 解析 install_profile.json 的 processors 数组
pub fn parse_processors(profile: &Value) -> Vec<Processor> {
    let Some(arr) = profile.get("processors").and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|p| {
            let jar = p.get("jar")?.as_str()?.to_string();
            let classpath = p
                .get("classpath")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let args = p
                .get("args")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let sides = p
                .get("sides")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let outputs = p
                .get("outputs")
                .and_then(Value::as_object)
                .map(|o| {
                    o.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            Some(Processor {
                jar,
                classpath,
                args,
                sides,
                outputs,
            })
        })
        .collect()
}

/// 判断某 processor 是否需要在 client 侧执行。
/// sides 为空表示通用(scoped),不含 "client"(如 ["server"])时跳过。
pub fn runs_on_client(p: &Processor) -> bool {
    p.sides.is_empty() || p.sides.iter().any(|s| s == "client")
}

/// 收集 data 中的 client 值并合并特殊变量,形成 `{KEY}` → 值 的映射。
/// 形如 `[net/minecraft/.../x.jar]` 的 Forge「括号 maven 路径」会被解析为
/// `libraries_dir/<路径>`,使处理器命令行拿到可用的绝对路径。
pub fn build_vars(
    data: &Value,
    special: &HashMap<String, String>,
    libraries_dir: &Path,
    installer_dir: &Path,
) -> HashMap<String, String> {
    let mut vars = special.clone();
    if let Some(obj) = data.as_object() {
        for (key, entry) in obj {
            if let Some(v) = client_value(entry) {
                vars.insert(key.clone(), resolve_data_value(&v, libraries_dir, installer_dir));
            }
        }
    }
    vars
}

/// 解析 Forge data 值:
/// - `[group:artifact:version[:classifier][@ext]]` → `libraries_dir/<maven路径>`
/// - 以 `/` 开头(如 `/data/client.lzma`)→ `installer_dir/<路径>`(installer 内 data/)
/// - 其它值原样返回(先去掉外层单引号,如 `'sha1'` 字面量)
fn resolve_data_value(v: &str, libraries_dir: &Path, installer_dir: &Path) -> String {
    let v = trim_quotes(v);
    if let Some(inner) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let rel = if inner.contains(':') {
            maven_rel_path(inner).unwrap_or_else(|| inner.to_string())
        } else {
            inner.to_string()
        };
        libraries_dir.join(rel).to_string_lossy().to_string()
    } else if let Some(rel) = v.strip_prefix('/') {
        installer_dir.join(rel).to_string_lossy().to_string()
    } else {
        v.to_string()
    }
}

/// 去除 data 值外层可能包着的单/双引号(Forge 用 `'sha1'` 表示哈希字面量)
fn trim_quotes(s: &str) -> &str {
    s.trim_matches('"').trim_matches('\'')
}

/// 处理器 args 层:仅把字面 `[maven]` 括号路径解析为 libraries 下绝对路径;
/// 其它值(含已展开的绝对路径、相对路径、普通参数)原样返回。
fn resolve_bracket_path(v: &str, libraries_dir: &Path) -> String {
    let v = trim_quotes(v);
    if let Some(inner) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let rel = if inner.contains(':') {
            maven_rel_path(inner).unwrap_or_else(|| inner.to_string())
        } else {
            inner.to_string()
        };
        libraries_dir.join(rel).to_string_lossy().to_string()
    } else {
        v.to_string()
    }
}

/// 把 Forge 的 maven 坐标转换为 libraries 下的相对路径:
/// `group:artifact:version[:classifier][@ext]` → `group/…/artifact/version/artifact-version[-classifier].ext`
pub(crate) fn maven_rel_path(coords: &str) -> Option<String> {
    let (base, ext) = match coords.rsplit_once('@') {
        Some((b, e)) if !e.is_empty() => (b, e.to_string()),
        _ => (coords, "jar".to_string()),
    };
    let parts: Vec<&str> = base.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0];
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts.get(3).filter(|c| !c.is_empty());
    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    let dir = format!("{}/{artifact}", group.replace('.', "/"));
    Some(format!("{dir}/{version}/{file}"))
}

/// 取 data 条目中 client(优先)的「真实字符串值」:
/// - 值为 JSON 数组 → 逗号连接
/// - 值为带引号的 JSON 字符串 → 解码去引号
/// - 值为字符串形式的 JSON 数组(如 "[...]")→ 保留原文(括号 maven 路径由 build_vars 再解析)
/// - 值为普通字符串 → 原样
fn client_value(entry: &Value) -> Option<String> {
    let v = entry.get("client").or_else(|| entry.get("server"))?;
    match v {
        Value::Array(arr) => Some(array_to_string(arr)),
        Value::String(s) => match serde_json::from_str::<Value>(s) {
            Ok(Value::Array(arr)) => Some(array_to_string(&arr)),
            Ok(Value::String(decoded)) => Some(decoded),
            Ok(Value::Number(n)) => Some(n.to_string()),
            _ => Some(s.clone()),
        },
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn array_to_string(arr: &[Value]) -> String {
    arr.iter()
        .filter_map(|x| x.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

/// 把字符串中的 `{KEY}` 全部替换为变量值
pub fn expand(s: &str, vars: &HashMap<String, String>) -> String {
    let mut out = s.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// 展开 classpath 元素为本地 maven 路径(libraries_dir 下)。
/// classpath 元素是 maven 坐标(如 `net.md-5:SpecialSource:1.11.0`),解析为 libraries 下的 jar 路径。
pub fn classpath_to_local(
    raw: &str,
    vars: &HashMap<String, String>,
    libraries_dir: &Path,
) -> String {
    let expanded = expand(raw, vars);
    let trimmed = trim_quotes(&expanded);
    if trimmed.contains(':') {
        maven_rel_path(trimmed)
            .map(|rel| libraries_dir.join(rel).to_string_lossy().to_string())
            .unwrap_or_else(|| libraries_dir.join(trimmed).to_string_lossy().to_string())
    } else {
        libraries_dir.join(trimmed).to_string_lossy().to_string()
    }
}

/// 读取处理器主类:从 processor.jar 的 META-INF/MANIFEST.MF 读 Main-Class
fn read_jar_main_class(jar_path: &Path) -> Result<String, RmclError> {
    let file = std::fs::File::open(jar_path)
        .map_err(|e| RmclError::other(format!("无法打开处理器 jar {}: {e}", jar_path.display())))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut manifest = archive
        .by_name("META-INF/MANIFEST.MF")
        .map_err(|_| RmclError::other(format!("处理器 jar {} 缺少 MANIFEST.MF", jar_path.display())))?;
    let mut text = String::new();
    use std::io::Read;
    manifest.read_to_string(&mut text)?;
    let main = text
        .lines()
        .find_map(|l| l.strip_prefix("Main-Class:").map(|s| s.trim().to_string()))
        .ok_or_else(|| RmclError::other(format!("处理器 jar {} 缺少 Main-Class", jar_path.display())))?;
    Ok(main)
}

/// 构建一个 processor 的完整命令行(不含程序名),含 -cp 与主类与参数。
/// 处理器 jar 与 classpath 均为 maven 坐标,解析到 libraries_dir 下的本地 jar。
pub fn build_processor_command(
    java_path: &str,
    installer_dir: &Path,
    libraries_dir: &Path,
    p: &Processor,
    vars: &HashMap<String, String>,
) -> Result<Vec<String>, RmclError> {
    let jar_rel = trim_quotes(&expand(&p.jar, vars)).to_string();
    let jar_path = if jar_rel.contains(':') {
        let rel = maven_rel_path(&jar_rel)
            .ok_or_else(|| RmclError::other(format!("无法解析处理器 jar 坐标: {jar_rel}")))?;
        libraries_dir.join(rel)
    } else {
        installer_dir.join(&jar_rel)
    };
    let main_class = read_jar_main_class(&jar_path)?;
    // 处理器自身 jar 也加入 classpath 末尾
    let mut cp: Vec<String> = p
        .classpath
        .iter()
        .map(|c| classpath_to_local(c, vars, libraries_dir))
        .collect();
    cp.push(jar_path.to_string_lossy().to_string());
    // args 展开后仅解析字面 maven 括号路径(如 --input [g:a:v@ext]);
    // {变量} 值已在 build_vars 解析为绝对路径(如 {ROOT}/run.sh → /…/run.sh),这里不再重复拼接
    let args: Vec<String> = p
        .args
        .iter()
        .map(|a| resolve_bracket_path(&expand(a, vars), libraries_dir))
        .collect();

    let mut cmd = vec![java_path.to_string(), "-cp".to_string(), cp.join(CP_SEP), main_class];
    cmd.extend(args);
    Ok(cmd)
}

/// 检查某 processor 的 outputs 是否都已产出且满足 SHA1(满足则无需重跑)
pub fn outputs_done(outputs: &HashMap<String, String>, vars: &HashMap<String, String>, root: &Path) -> bool {
    if outputs.is_empty() {
        return true;
    }
    outputs.iter().all(|(rel, expect_sha1)| {
        let path = root.join(expand(rel, vars));
        match sha1_of(&path) {
            Ok(hash) => hash == *expect_sha1,
            Err(_) => false,
        }
    })
}

/// 执行全部 client 侧 processor,按顺序运行;失败时返回含「序号 / jar / stderr」的错误。
/// minecraft_jar 为 vanilla client.jar 的本地路径,填入 `{MINECRAFT_JAR}`。
pub fn run_processors(
    contents: &InstallerContents,
    installer_dir: &Path,
    libraries_dir: &Path,
    java_path: &str,
    minecraft_jar: &Path,
) -> Result<(), RmclError> {
    let profile = contents
        .install_profile
        .as_ref()
        .ok_or_else(|| RmclError::other("install_profile.json 缺失,无法解析 processors"))?;
    let processors = parse_processors(profile);
    if processors.is_empty() {
        return Ok(());
    }
    let data = profile.get("data");
    let special = special_vars(installer_dir, libraries_dir, minecraft_jar);
    let vars = match data {
        Some(d) => build_vars(d, &special, libraries_dir, installer_dir),
        None => special,
    };

    for (idx, p) in processors.iter().enumerate() {
        if !runs_on_client(p) {
            continue;
        }
        // 仅当该处理器声明了 outputs 且全部满足时才跳过(空 outputs 表示总是执行)
        if !p.outputs.is_empty() && outputs_done(&p.outputs, &vars, libraries_dir) {
            continue;
        }
        let cmd = build_processor_command(java_path, installer_dir, libraries_dir, p, &vars)?;
        let out = std::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .output()
            .map_err(|e| {
                RmclError::other(format!(
                    "第 {} 个 processor 执行失败({}):{e}",
                    idx + 1,
                    expand(&p.jar, &vars)
                ))
            })?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(RmclError::other(format!(
                "第 {} 个 processor 失败({}):stderr = {}",
                idx + 1,
                expand(&p.jar, &vars),
                stderr.trim()
            )));
        }
    }
    Ok(())
}

/// 构建处理器变量表(调试/验证用):合并特殊变量 + data 值解析。
pub fn build_processor_vars(
    contents: &InstallerContents,
    installer_dir: &Path,
    libraries_dir: &Path,
    minecraft_jar: &Path,
) -> HashMap<String, String> {
    let special = special_vars(installer_dir, libraries_dir, minecraft_jar);
    match contents.install_profile.as_ref().and_then(|p| p.get("data")) {
        Some(d) => build_vars(d, &special, libraries_dir, installer_dir),
        None => special,
    }
}

/// 构建全部处理器的 java 命令行(不执行),返回 (序号, 命令)。调试/验证用。
pub fn build_processors_preview(
    contents: &InstallerContents,
    installer_dir: &Path,
    libraries_dir: &Path,
    minecraft_jar: &Path,
    java_path: &str,
) -> Result<Vec<(usize, Vec<String>)>, RmclError> {
    let profile = contents
        .install_profile
        .as_ref()
        .ok_or_else(|| RmclError::other("install_profile.json 缺失"))?;
    let processors = parse_processors(profile);
    let vars = build_processor_vars(contents, installer_dir, libraries_dir, minecraft_jar);
    let mut out = Vec::new();
    for (i, p) in processors.iter().enumerate() {
        let cmd = build_processor_command(java_path, installer_dir, libraries_dir, p, &vars)?;
        out.push((i, cmd));
    }
    Ok(out)
}

/// 特殊占位符(非来自 data):SIDE/ROOT/INSTALLER/INSTALLER_DIR/MINECRAFT_JAR 等。
/// ROOT 取 minecraft 根目录(libraries 的父目录);INSTALLER 指向 installer jar;
/// MINECRAFT_JAR 指向 vanilla client.jar。
fn special_vars(
    installer_dir: &Path,
    libraries_dir: &Path,
    minecraft_jar: &Path,
) -> HashMap<String, String> {
    let root = libraries_dir.parent().unwrap_or(libraries_dir);
    let mut m = HashMap::new();
    m.insert("SIDE".into(), "client".into());
    m.insert("ROOT".into(), root.to_string_lossy().to_string());
    m.insert("INSTALLER_DIR".into(), installer_dir.to_string_lossy().to_string());
    m.insert("INSTALLER".into(), installer_dir.join("forge-installer.jar").to_string_lossy().to_string());
    m.insert("MINECRAFT_JAR".into(), minecraft_jar.to_string_lossy().to_string());
    m.insert("MINECRAFT_JAR_DIRECTORY".into(), libraries_dir.to_string_lossy().to_string());
    m
}

/// 从 install_profile 提取处理器工具链库的 maven 坐标列表(classpath/jar 可能用到的库)。
#[allow(dead_code)]
pub fn processor_library_coords(contents: &InstallerContents) -> Vec<String> {
    let Some(profile) = contents.install_profile.as_ref() else {
        return Vec::new();
    };
    profile
        .get("libraries")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|l| l.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// 供上层安装流程下载 processor classpath 所需 maven 库(待 T5.6 接线)
#[allow(dead_code)]
pub fn processor_classpath_libs(contents: &InstallerContents) -> Vec<String> {
    let Some(profile) = contents.install_profile.as_ref() else {
        return Vec::new();
    };
    parse_processors(profile)
        .into_iter()
        .flat_map(|p| p.classpath)
        .collect()
}

/// 把 maven 相对路径转绝对路径(libraries_dir 下)
#[allow(dead_code)]
pub fn resolve_lib_path(libraries_dir: &Path, rel: &str) -> PathBuf {
    libraries_dir.join(rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = r#"{
      "data": {
        "MINECRAFT_JAR": {"client": "[net.minecraft:client:1.20.1:slim]"},
        "BINPATCH": {"client": "/data/client.lzma"},
        "MC_SLIM_SHA": {"client": "'de86b035d2da0f78940796bb95c39a932ed84834'"},
        "BSERVICE": {"client": "\"net.minecraftforge:forge:1.20.1-47.2.0\""}
      },
      "processors": [
        {
          "jar": "net.minecraftforge:installertools:1.3.0",
          "classpath": ["net.md-5:SpecialSource:1.11.0"],
          "sides": ["client"],
          "args": ["{SIDE}", "{MINECRAFT_JAR}", "{BINPATCH}", "{BSERVICE}"],
          "outputs": {"{MINECRAFT_JAR}": "{MC_SLIM_SHA}"}
        }
      ]
    }"#;

    fn profile() -> Value {
        serde_json::from_str(PROFILE).unwrap()
    }

    fn vars_for(profile: &Value) -> (HashMap<String, String>, std::path::PathBuf, std::path::PathBuf) {
        let data = profile.get("data");
        let mut special = HashMap::new();
        special.insert("SIDE".into(), "client".into());
        let libs = std::path::PathBuf::from("/root/libraries");
        let installer = std::path::PathBuf::from("/work");
        let vars = match data {
            Some(d) => build_vars(d, &special, &libs, &installer),
            None => special,
        };
        (vars, libs, installer)
    }

    #[test]
    fn parses_processors() {
        let ps = parse_processors(&profile());
        assert_eq!(ps.len(), 1);
        let p = &ps[0];
        assert_eq!(p.jar, "net.minecraftforge:installertools:1.3.0");
        assert_eq!(p.classpath.len(), 1);
        assert_eq!(p.args.len(), 4);
        assert_eq!(p.sides, vec!["client".to_string()]);
        assert!(p.outputs.contains_key("{MINECRAFT_JAR}"));
    }

    #[test]
    fn runs_on_client_skips_server_only() {
        let make = |sides: Vec<String>| Processor {
            jar: "x".into(),
            classpath: vec![],
            args: vec![],
            sides,
            outputs: Default::default(),
        };
        assert!(!runs_on_client(&make(vec!["server".into()])), "仅 server 应跳过");
        assert!(runs_on_client(&make(vec!["client".into()])), "client 应执行");
        assert!(runs_on_client(&make(vec!["client".into(), "server".into()])), "含 client 应执行");
        assert!(runs_on_client(&make(vec![])), "无 sides(通用)应执行");
    }

    #[test]
    fn client_value_decodes_json_string_and_array() {
        let data = profile().get("data").unwrap().as_object().unwrap().clone();
        // Forge 的 data 值是 shell 风格 `[...]`,非合法 JSON,在 client_value 层应原样保留
        assert_eq!(
            client_value(data.get("MINECRAFT_JAR").unwrap()).unwrap(),
            "[net.minecraft:client:1.20.1:slim]"
        );
        let b = data.get("BSERVICE").unwrap();
        assert_eq!(client_value(b).unwrap(), "net.minecraftforge:forge:1.20.1-47.2.0");
        // 带单引号的 sha 字面量在 client_value 层仍带引号,由 resolve_data_value 剥离
        assert_eq!(
            client_value(data.get("MC_SLIM_SHA").unwrap()).unwrap(),
            "'de86b035d2da0f78940796bb95c39a932ed84834'"
        );
    }

    #[test]
    fn expand_replaces_brackets() {
        let mut vars = HashMap::new();
        vars.insert("SIDE".into(), "client".into());
        assert_eq!(expand("--side {SIDE}", &vars), "--side client");
    }

    #[test]
    fn build_vars_resolves_maven_and_installer_data_values() {
        let p = profile();
        let (vars, _, _) = vars_for(&p);
        assert_eq!(vars.get("SIDE").unwrap(), "client");
        // maven 坐标括号值 → libraries 下绝对路径
        assert_eq!(
            vars.get("MINECRAFT_JAR").unwrap(),
            "/root/libraries/net/minecraft/client/1.20.1/client-1.20.1-slim.jar"
        );
        // "/" 开头的 data 值 → installer_dir 下
        assert_eq!(vars.get("BINPATCH").unwrap(), "/work/data/client.lzma");
        // 单引号 sha 被剥离
        assert_eq!(
            vars.get("MC_SLIM_SHA").unwrap(),
            "de86b035d2da0f78940796bb95c39a932ed84834"
        );
    }

    #[test]
    fn maven_rel_path_converts_coords() {
        assert_eq!(
            maven_rel_path("net.minecraftforge:installertools:1.3.0").unwrap(),
            "net/minecraftforge/installertools/1.3.0/installertools-1.3.0.jar"
        );
        assert_eq!(
            maven_rel_path("net.minecraft:client:1.20.1-20230612.114412:slim").unwrap(),
            "net/minecraft/client/1.20.1-20230612.114412/client-1.20.1-20230612.114412-slim.jar"
        );
        assert_eq!(
            maven_rel_path("de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412:mappings@txt").unwrap(),
            "de/oceanlabs/mcp/mcp_config/1.20.1-20230612.114412/mcp_config-1.20.1-20230612.114412-mappings.txt"
        );
        assert_eq!(
            maven_rel_path("net.minecraftforge:ForgeAutoRenamingTool:0.1.22:all").unwrap(),
            "net/minecraftforge/ForgeAutoRenamingTool/0.1.22/ForgeAutoRenamingTool-0.1.22-all.jar"
        );
        assert!(maven_rel_path("bad").is_none());
    }

    #[test]
    fn classpath_to_local_resolves_maven_coords() {
        let mut vars = HashMap::new();
        vars.insert("SIDE".into(), "client".into());
        let libs = Path::new("/root/libraries");
        assert_eq!(
            classpath_to_local("net.md-5:SpecialSource:1.11.0", &vars, libs),
            "/root/libraries/net/md-5/SpecialSource/1.11.0/SpecialSource-1.11.0.jar"
        );
    }
}
