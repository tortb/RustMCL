//! 实例级配置,对应 ~/.runa/instances/<id>/instance.toml

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::RunaError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InstanceConfig {
    pub meta: MetaConfig,
    pub jvm: JvmConfig,
    pub game: GameConfig,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            meta: MetaConfig::default(),
            jvm: JvmConfig::default(),
            game: GameConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MetaConfig {
    pub name: String,
    pub mc_version: String,
    /// vanilla | forge | fabric | quilt
    pub loader: String,
    pub loader_version: String,
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self {
            name: "新实例".into(),
            mc_version: String::new(),
            loader: "vanilla".into(),
            loader_version: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JvmConfig {
    pub min_memory: u32,
    pub max_memory: u32,
    pub extra_args: Vec<String>,
}

impl Default for JvmConfig {
    fn default() -> Self {
        Self {
            min_memory: 1024,
            max_memory: 4096,
            extra_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameConfig {
    pub resolution: Resolution,
    pub fullscreen: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            resolution: Resolution::default(),
            fullscreen: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Default for Resolution {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
        }
    }
}

impl InstanceConfig {
    pub fn load(path: &Path) -> Result<Self, RunaError> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), RunaError> {
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
        let mut cfg = InstanceConfig::default();
        cfg.meta.name = "我的生存".into();
        cfg.meta.mc_version = "1.21.1".into();
        cfg.jvm.max_memory = 8192;
        let path = std::env::temp_dir().join("runa_instance_test.toml");
        cfg.save(&path).unwrap();
        let loaded = InstanceConfig::load(&path).unwrap();
        assert_eq!(loaded.meta.name, "我的生存");
        assert_eq!(loaded.meta.mc_version, "1.21.1");
        assert_eq!(loaded.jvm.max_memory, 8192);
        assert_eq!(loaded.game.resolution.width, 1280);
        let _ = std::fs::remove_file(&path);
    }
}
