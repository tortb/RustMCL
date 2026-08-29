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
                outputs,
            })
        })
        .collect()
}

/// 判断某 processor 是否需要在 client 侧执行(无 sides 字段视为通用)
pub fn runs_on_client(p: &Processor) -> bool {
    // 该信息在 parse 时未保留,默认 client 场景下全部执行
    let _ = p;
    true
}

/// 收集 data 中的 client 值并合并特殊变量,形成 `{KEY}` → 值 的映射
pub fn build_vars(data: &Value, special: &HashMap<String, String>) -> HashMap<String, String> {
    let mut vars = special.clone();
    if let Some(obj) = data.as_object() {
        for (key, entry) in obj {
            if let Some(v) = client_value(entry) {
                vars.insert(key.clone(), v);
            }
        }
    }
    vars
}

/// 取 data 条目中 client(优先)的「真实字符串值」:
/// - 值为 JSON 数组 → 逗号连接
/// - 值为带引号的 JSON 字符串 → 解码去引号
/// - 值为字符串形式的 JSON 数组(如 "[...]")→ 解析后取元素
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

/// 展开 classpath 元素为本地 maven 路径(libraries_dir 下)
pub fn classpath_to_local(
    raw: &str,
    vars: &HashMap<String, String>,
    libraries_dir: &Path,
) -> String {
    let expanded = expand(raw, vars);
    libraries_dir.join(expanded).to_string_lossy().to_string()
}

/// 读取处理器主类:从 processor.jar 的 META-INF/MANIFEST.MF 读 Main-Class
fn read_jar_main_class(installer_dir: &Path, jar_rel: &str, vars: &HashMap<String, String>) -> Result<String, RmclError> {
    let jar_path = installer_dir.join(expand(jar_rel, vars));
    let file = std::fs::File::open(&jar_path)
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

/// 构建一个 processor 的完整命令行(不含程序名),含 -cp 与主类与参数
pub fn build_processor_command(
    java_path: &str,
    installer_dir: &Path,
    libraries_dir: &Path,
    p: &Processor,
    vars: &HashMap<String, String>,
) -> Result<Vec<String>, RmclError> {
    let main_class = read_jar_main_class(installer_dir, &p.jar, vars)?;
    let jar_local = installer_dir.join(expand(&p.jar, vars));
    // 处理器自身 jar 也加入 classpath 末尾
    let mut cp: Vec<String> = p
        .classpath
        .iter()
        .map(|c| classpath_to_local(c, vars, libraries_dir))
        .collect();
    cp.push(jar_local.to_string_lossy().to_string());
    let args: Vec<String> = p.args.iter().map(|a| expand(a, vars)).collect();

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

/// 执行全部 client 侧 processor,按顺序运行;失败时返回含「序号 / jar / stderr」的错误
pub fn run_processors(
    contents: &InstallerContents,
    installer_dir: &Path,
    libraries_dir: &Path,
    java_path: &str,
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
    let special = special_vars(installer_dir, libraries_dir);
    let vars = match data {
        Some(d) => build_vars(d, &special),
        None => special,
    };

    for (idx, p) in processors.iter().enumerate() {
        if !runs_on_client(p) {
            continue;
        }
        if outputs_done(&p.outputs, &vars, libraries_dir) {
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

/// 特殊占位符(非来自 data):SIDE/ROOT/INSTALLER_DIR/MINECRAFT_JAR 等
fn special_vars(installer_dir: &Path, libraries_dir: &Path) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("SIDE".into(), "client".into());
    m.insert("ROOT".into(), libraries_dir.to_string_lossy().to_string());
    m.insert("INSTALLER_DIR".into(), installer_dir.to_string_lossy().to_string());
    m.insert("MINECRAFT_JAR_DIRECTORY".into(), libraries_dir.to_string_lossy().to_string());
    m
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
        "MINECRAFT_JAR": {"client": "[net/minecraft/client/1.20.1/client-1.20.1.jar]"},
        "BINPATCH": {"client": "/data/client.lzma"},
        "BSERVICE": {"client": "\"net.minecraftforge:forge:1.20.1-47.2.0\""}
      },
      "processors": [
        {
          "jar": "META-INF/versions/1.20.1/processor.jar",
          "classpath": ["net/minecraftforge/accesstransformers/acc/0.0.1/acc-0.0.1.jar"],
          "args": ["{SIDE}", "{MINECRAFT_JAR}", "{BSERVICE}"],
          "outputs": {"data/client.lzma": "abc"}
        }
      ]
    }"#;

    fn profile() -> Value {
        serde_json::from_str(PROFILE).unwrap()
    }

    #[test]
    fn parses_processors() {
        let ps = parse_processors(&profile());
        assert_eq!(ps.len(), 1);
        let p = &ps[0];
        assert_eq!(p.jar, "META-INF/versions/1.20.1/processor.jar");
        assert_eq!(p.classpath.len(), 1);
        assert_eq!(p.args.len(), 3);
        assert_eq!(p.outputs.get("data/client.lzma").unwrap(), "abc");
    }

    #[test]
    fn client_value_decodes_json_string_and_array() {
        let data = profile().get("data").unwrap().as_object().unwrap().clone();
        let entry = data.get("MINECRAFT_JAR").unwrap();
        // Forge 的 data 值是 shell 风格 `[...]`,非合法 JSON,应原样保留
        assert_eq!(
            client_value(entry).unwrap(),
            "[net/minecraft/client/1.20.1/client-1.20.1.jar]"
        );
        let b = data.get("BSERVICE").unwrap();
        assert_eq!(client_value(b).unwrap(), "net.minecraftforge:forge:1.20.1-47.2.0");
    }

    #[test]
    fn expand_replaces_brackets() {
        let mut vars = HashMap::new();
        vars.insert("SIDE".into(), "client".into());
        assert_eq!(expand("--side {SIDE}", &vars), "--side client");
    }

    #[test]
    fn build_vars_includes_special() {
        let binding = profile();
        let data = binding.get("data").unwrap();
        let mut special = HashMap::new();
        special.insert("SIDE".into(), "client".into());
        let vars = build_vars(data, &special);
        assert_eq!(vars.get("SIDE").unwrap(), "client");
        assert_eq!(vars.get("MINECRAFT_JAR").unwrap(), "[net/minecraft/client/1.20.1/client-1.20.1.jar]");
    }
}
