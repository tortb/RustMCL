use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::RmclError;

/// 应用级配置,对应 ~/.rustmcl/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub java: JavaConfig,
    pub download: DownloadConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            java: JavaConfig::default(),
            download: DownloadConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub data_dir: String,
    pub theme: String,
    pub language: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: "~/.rustmcl".into(),
            theme: "dark".into(),
            language: "zh-CN".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JavaConfig {
    pub auto_detect: bool,
    pub default_java_path: String,
}

impl Default for JavaConfig {
    fn default() -> Self {
        Self {
            auto_detect: true,
            default_java_path: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadConfig {
    pub max_concurrent: u32,
    pub mirror: String,
    pub retry_times: u32,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 8,
            mirror: "official".into(),
            retry_times: 3,
        }
    }
}

impl AppConfig {
    /// 解析最终使用的 java 可执行文件路径:
    /// auto_detect 或未配置路径时返回 "java"(依赖 PATH),否则用配置的绝对路径
    pub fn java_path(&self) -> String {
        if self.java.auto_detect || self.java.default_java_path.trim().is_empty() {
            "java".to_string()
        } else {
            self.java.default_java_path.trim().to_string()
        }
    }

    /// 加载配置;文件不存在时生成默认配置并落盘
    /// 反序列化采用 #[serde(default)],字段缺失不崩溃
    pub fn load_or_create(path: &Path) -> Result<Self, RmclError> {
        if !path.exists() {
            let cfg = Self::default();
            cfg.save(path)?;
            return Ok(cfg);
        }
        let content = std::fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&content)?;
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<(), RmclError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_values() {
        let cfg = AppConfig::default();
        let path = std::env::temp_dir().join("rmcl_config_test.toml");
        cfg.save(&path).unwrap();
        let loaded = AppConfig::load_or_create(&path).unwrap();
        assert_eq!(loaded.general.theme, "dark");
        assert_eq!(loaded.general.language, "zh-CN");
        assert_eq!(loaded.download.max_concurrent, 8);
        assert_eq!(loaded.java.auto_detect, true);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn partial_toml_does_not_panic() {
        // 缺失字段时应回退到默认值而非崩溃
        let content = "[java]\nauto_detect = false\n";
        let cfg: AppConfig = toml::from_str(content).unwrap();
        assert_eq!(cfg.java.auto_detect, false);
        assert_eq!(cfg.download.retry_times, 3);
    }
}
