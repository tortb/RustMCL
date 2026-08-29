//! T5.5 旧版 Forge 兼容路径(≤1.12.2):直接下载 universal jar + 手动拼装 version json,无需 processors
#![allow(dead_code)]

use crate::error::RmclError;

/// 旧版 Forge 安装入口(待 T5.5 实现)
pub async fn install_legacy(
    _client: &reqwest::Client,
    _mc_version: &str,
    _forge_version: &str,
    _retry_times: u32,
) -> Result<(), RmclError> {
    // TODO(T5.5): 下载 universal jar,拼装 version json 写入 versions/ 并下载主库
    Err(RmclError::other("Forge 旧版(≤1.12.2)安装尚未实现(T5.5)"))
}
