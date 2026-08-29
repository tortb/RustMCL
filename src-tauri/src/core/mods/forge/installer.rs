//! T5.2 下载 Forge installer jar 并解压,读取 install_profile.json 与内嵌 version.json

use std::path::Path;

use serde_json::Value;

use crate::error::RmclError;

use super::{download_to, MAVEN_BASE};

/// Forge 安装包(installer)解压产物
pub struct InstallerContents {
    pub install_profile: Option<Value>,
    /// 内嵌的 version json(forge 侧合并源),可能为空(部分版本直接以 install_profile 承载)
    pub version_json: Option<Value>,
}

/// installer jar 的下载地址
pub fn installer_url(mc_version: &str, forge_version: &str) -> String {
    let dir = format!("{mc_version}-{forge_version}");
    format!("{MAVEN_BASE}/{dir}/forge-{dir}-installer.jar")
}

/// 下载 installer jar 到 dest_dir/(命名为 forge-installer.jar),返回本地路径。
/// 若已存在则直接复用(下载采用原子改名,不会残留半成品)。
pub async fn download_installer(
    client: &reqwest::Client,
    mc_version: &str,
    forge_version: &str,
    dest_dir: &Path,
    retry_times: u32,
) -> Result<std::path::PathBuf, RmclError> {
    std::fs::create_dir_all(dest_dir)?;
    let dest = dest_dir.join("forge-installer.jar");
    if dest.exists() {
        return Ok(dest);
    }
    let url = installer_url(mc_version, forge_version);
    download_to(client, &url, &dest, retry_times).await?;
    Ok(dest)
}

/// 把 installer jar 的全部内容解压到 dest_dir(供 processors 使用 META-INF/versions/**/processor.jar 与 data/**)。
/// 已对逐个条目做 zip-slip 防护:异常路径(绝对、含 `..`)会被跳过。
pub fn extract_installer_files(jar_path: &Path, dest_dir: &Path) -> Result<(), RmclError> {
    let file = std::fs::File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    std::fs::create_dir_all(dest_dir)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(rel) = entry.enclosed_name() else { continue };
        let out = dest_dir.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out_file = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut out_file)?;
    }
    Ok(())
}

/// 解压 installer jar,读取 install_profile.json 与尽可能匹配的 version json
pub fn extract_installer(jar_path: &Path, mc_version: &str, forge_version: &str) -> Result<InstallerContents, RmclError> {
    let file = std::fs::File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let install_profile = read_json_entry(&mut archive, "install_profile.json")?;
    // 候选 version json 文件名(Forge 各版本命名不一)
    let version = format!("{mc_version}-{forge_version}");
    let candidates = [
        "version.json",
        &format!("{version}.json"),
        &format!("forge-{version}.json"),
        &format!("forge-{mc_version}-{forge_version}-profile.json"),
    ];
    let mut version_json = None;
    for name in candidates {
        if let Some(v) = read_json_entry(&mut archive, name)? {
            version_json = Some(v);
            break;
        }
    }
    Ok(InstallerContents {
        install_profile,
        version_json,
    })
}

/// 读取 zip 里某个 json 文本并反序列化;不存在时返回 None
fn read_json_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    name: &str,
) -> Result<Option<Value>, RmclError> {
    match archive.by_name(name) {
        Ok(mut f) => {
            let mut buf = String::new();
            use std::io::Read;
            f.read_to_string(&mut buf)?;
            Ok(Some(serde_json::from_str(&buf)?))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_url_format() {
        // 1.20.1 + 47.2.0 → maven 路径带 mc-forge
        let url = installer_url("1.20.1", "47.2.0");
        assert!(url.ends_with("forge-1.20.1-47.2.0-installer.jar"));
        assert!(url.contains("1.20.1-47.2.0"));
    }
}
