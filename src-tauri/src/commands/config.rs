//! 应用配置命令:读写 config.toml + Java 检测

use tauri::State;

use crate::config::app_config::AppConfig;
use crate::AppState;

/// 返回当前应用配置(含数据目录)
#[tauri::command]
pub fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    AppConfig::load_or_create(&state.config_path).map_err(|e| e.to_string())
}

/// 保存应用配置并返回最新配置
#[tauri::command]
pub fn update_app_config(state: State<'_, AppState>, config: AppConfig) -> Result<AppConfig, String> {
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    Ok(config)
}

/// 检测系统 Java 版本(通过 `java -version`),返回去掉引号的版本号,如 "21.0.5"
#[tauri::command]
pub fn detect_java() -> Result<Option<String>, String> {
    let output = std::process::Command::new("java")
        .arg("-version")
        .output()
        .map_err(|e| format!("无法执行 java -version: {e}"))?;
    // java -version 输出到 stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    let version = extract_version(&stderr);
    Ok(version)
}

/// 返回"实际用于启动"的 Java 版本:按应用配置解析 java 路径
/// (auto_detect 走 PATH,否则用 default_java_path),再执行 `-version` 解析版本号。
#[tauri::command]
pub fn get_java_version(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let cfg = AppConfig::load_or_create(&state.config_path).map_err(|e| e.to_string())?;
    let java_path = cfg.java_path();
    let output = std::process::Command::new(&java_path)
        .arg("-version")
        .output()
        .map_err(|e| format!("无法执行 {java_path} -version: {e}"))?;
    // java -version 输出到 stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(extract_version(&stderr))
}

/// 从 `-version` 输出中提取引号内的版本号
pub(crate) fn extract_version(out: &str) -> Option<String> {
    let first = out.lines().next()?;
    let start = first.find('"')? + 1;
    let end = first[start..].find('"')? + start;
    Some(first[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_quoted_version() {
        let out = "openjdk version \"21.0.5\" 2024-10-15\nOpenJDK Runtime Environment...";
        assert_eq!(extract_version(out).as_deref(), Some("21.0.5"));
    }

    #[test]
    fn extracts_java8_style() {
        let out = "java version \"1.8.0_392\"\nJava(TM) SE Runtime Environment";
        // 1.8 保留完整字符串,供前端识别为 Java 8
        assert_eq!(extract_version(out).as_deref(), Some("1.8.0_392"));
    }

    #[test]
    fn empty_output_none() {
        assert_eq!(extract_version(""), None);
    }
}
