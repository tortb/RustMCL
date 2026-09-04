pub mod app_config;
pub mod instance_config;

use std::path::{Path, PathBuf};

/// 应用数据目录(默认 ~/.rustmcl)
pub fn default_data_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".rustmcl"))
        .unwrap_or_else(|| PathBuf::from(".rustmcl"))
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

/// 数据目录迁移:旧目录 ~/.runa 存在且新目录 ~/.rustmcl 不存在时,迁移一次。
/// 在应用启动早期调用,避免数据丢失。
pub fn migrate_legacy_data_dir() {
    let new_dir = default_data_dir();
    if new_dir.exists() {
        return;
    }
    if let Some(home) = dirs::home_dir() {
        let old_dir = home.join(".runa");
        migrate_dir(&old_dir, &new_dir);
    }
}

/// 若 old 存在且 new 不存在,把 old 整体迁移到 new(跨设备失败时回退到复制)。
fn migrate_dir(old: &Path, new: &Path) -> bool {
    if new.exists() || !old.exists() {
        return false;
    }
    if let Some(parent) = new.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::rename(old, new).is_ok() {
        eprintln!(
            "[rmcl] 已将数据目录从 {} 迁移到 {}",
            old.display(),
            new.display()
        );
        return true;
    }
    // 跨设备时 rename 失败,回退为复制 + 删除
    let ok = copy_dir_all(old, new).is_ok();
    if ok {
        let _ = std::fs::remove_dir_all(old);
    }
    ok
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dir_uses_rustmcl() {
        assert!(default_data_dir().ends_with(".rustmcl"));
    }

    #[test]
    fn migrate_noop_when_new_exists() {
        let root =
            std::env::temp_dir().join(format!("rmcl_mig_test_{}", uuid::Uuid::new_v4().simple()));
        let new = root.join("new");
        std::fs::create_dir_all(&new).unwrap();
        let _ = std::fs::write(new.join("x.txt"), "x");
        assert!(!migrate_dir(&root.join("old"), &new));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn migrate_noop_when_old_missing() {
        let root =
            std::env::temp_dir().join(format!("rmcl_mig_test_{}", uuid::Uuid::new_v4().simple()));
        let ok = migrate_dir(&root.join("old"), &root.join("new"));
        assert!(!ok);
        assert!(!root.join("new").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn migrate_moves_content() {
        let root =
            std::env::temp_dir().join(format!("rmcl_mig_test_{}", uuid::Uuid::new_v4().simple()));
        let old = root.join("old");
        let new = root.join("new");
        std::fs::create_dir_all(old.join("sub")).unwrap();
        std::fs::write(old.join("config.toml"), "x").unwrap();
        std::fs::write(old.join("sub/rmcl.db"), "y").unwrap();

        assert!(migrate_dir(&old, &new));
        assert!(new.join("config.toml").exists());
        assert!(new.join("sub/rmcl.db").exists());
        assert!(!old.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
