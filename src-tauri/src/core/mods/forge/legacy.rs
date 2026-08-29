//! T5.5 旧版 Forge 兼容路径(≤1.12.2):
//! 复用 installer 里的 version.json / install_profile.json 与原版合并,
//! 并锁定 launchwrapper 主类 + 确保 universal jar 作为库;运行期不做 processors。
//! 说明:旧版(1.13 之前)不依赖二进制补丁处理器,universal jar + 合并后的 version.json 即可启动。

use std::path::Path;

use crate::core::version::version_json::{Download, Library, LibraryDownloads, VersionJson};
use crate::error::RmclError;

use super::profile_merge;
use super::{forge_merged_id, forge_work_dir, installer, MAVEN_BASE};

/// 旧版 Forge 合并后的 version.json(带缓存)。
/// 与原版合并后,强制主类为 launchwrapper,并确保 universal jar 出现在 libraries 中。
pub async fn resolve_legacy(
    client: &reqwest::Client,
    data_dir: &Path,
    mc_version: &str,
    forge_version: &str,
    retry_times: u32,
) -> Result<VersionJson, RmclError> {
    let merged_id = forge_merged_id(mc_version, forge_version);
    let merged_cache = data_dir
        .join("cache")
        .join("versions")
        .join(format!("{merged_id}.json"));
    if let Some(v) = load_cached(&merged_cache) {
        return Ok(v);
    }

    // 1. 下载 installer 并读取其中的 version.json / install_profile.json
    let work = forge_work_dir(data_dir, mc_version, forge_version);
    let jar = installer::download_installer(client, mc_version, forge_version, &work, retry_times).await?;
    let contents = installer::extract_installer(&jar, mc_version, forge_version)?;

    // 2. 与原版合并(若 installer 里两个 json 都没有,则退回纯原版)
    let vanilla = crate::core::loader::fetch_vanilla(client, data_dir, mc_version, retry_times).await?;
    let mut merged = if let Some(forge_json) = contents
        .version_json
        .as_ref()
        .or(contents.install_profile.as_ref())
    {
        profile_merge::merge_forge(&vanilla, forge_json)?
    } else {
        vanilla
    };

    // 3. 旧版固定主类 + 确保 universal jar 为库
    merged.main_class = "net.minecraft.launchwrapper.Launch".to_string();
    ensure_universal_jar(&mut merged, mc_version, forge_version);

    // 4. 写缓存,供启动链路复用
    if let Some(parent) = merged_cache.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&merged_cache, serde_json::to_string(&merged)?)?;
    Ok(merged)
}

/// 若 libraries 中缺少 `net.minecraftforge:forge:<mc>-<forge>`,则补一个 universal jar 库(maven 地址,sha1 留空)
fn ensure_universal_jar(merged: &mut VersionJson, mc_version: &str, forge_version: &str) {
    let dir = format!("{mc_version}-{forge_version}");
    let name = format!("net.minecraftforge:forge:{dir}");
    if merged.libraries.iter().any(|l| l.name == name) {
        return;
    }
    let path = format!("net/minecraftforge/forge/{dir}/forge-{dir}-universal.jar");
    let url = format!("{MAVEN_BASE}/{dir}/forge-{dir}-universal.jar");
    merged.libraries.push(Library {
        name,
        rules: None,
        natives: None,
        extract: None,
        downloads: Some(LibraryDownloads {
            artifact: Some(Download {
                path: Some(path),
                sha1: String::new(),
                size: 0,
                url,
            }),
            classifiers: None,
        }),
    });
}

fn load_cached(path: &Path) -> Option<VersionJson> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_universal_jar_adds_once() {
        let mut v: VersionJson = serde_json::from_str(
            r#"{
              "id": "1.12.2",
              "assetIndex": {"id":"x","sha1":"1","size":1,"url":"u"},
              "downloads": {"client": {"sha1":"2","size":1,"url":"c"}},
              "libraries": [],
              "mainClass": "net.minecraft.launchwrapper.Launch",
              "type": "release"
            }"#,
        )
        .unwrap();
        ensure_universal_jar(&mut v, "1.12.2", "14.23.5.2860");
        assert_eq!(v.libraries.len(), 1);
        let lib = &v.libraries[0];
        assert!(lib.name.starts_with("net.minecraftforge:forge:1.12.2-14.23.5.2860"));
        let artifact = lib.downloads.as_ref().unwrap().artifact.as_ref().unwrap();
        assert!(artifact.path.as_deref().unwrap().ends_with("-universal.jar"));
        assert_eq!(artifact.sha1, "");
        // 再次调用不重复添加
        ensure_universal_jar(&mut v, "1.12.2", "14.23.5.2860");
        assert_eq!(v.libraries.len(), 1);
    }
}
