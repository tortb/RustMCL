pub mod app_config;

use std::path::PathBuf;

/// 应用数据目录(默认 ~/.runa)
pub fn default_data_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".runa"))
        .unwrap_or_else(|| PathBuf::from(".runa"))
}

/// 展开路径中的 `~` 前缀(实例路径等场景使用)
#[allow(dead_code)]
pub fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    }
}
