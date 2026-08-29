//! T5.4 Processors 执行引擎(核心难点,建议单独完整会话专攻)
//! 计划:变量替换 → 顺序执行 `java -cp <classpath> <mainClass> <args>` → 校验 outputs SHA1 断点续装
#![allow(dead_code)]

use serde_json::Value;

use crate::core::mods::forge::installer::InstallerContents;
use crate::error::RmclError;

/// 处理器执行入口(待 T5.4 单独会话实现)
pub async fn run_processors(
    _client: &reqwest::Client,
    _contents: &InstallerContents,
    _mc_version: &str,
    _forge_version: &str,
    _java_path: &str,
    _retry_times: u32,
) -> Result<(), RmclError> {
    let _ = &Value::Null;
    // TODO(T5.4): 解析 data 变量、顺序执行 processor、校验 outputs
    Err(RmclError::other("Forge 处理器执行引擎尚未实现(T5.4)"))
}
