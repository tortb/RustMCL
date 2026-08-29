//! assets 下载计划构建:index json + objects

use std::path::PathBuf;

use crate::error::RmclError;

use super::DownloadItem;
use crate::core::version::version_json::{AssetIndexFile, VersionJson};

/// 资源文件默认源
const ASSET_BASE: &str = "https://resources.download.minecraft.net";

/// assetIndex.json → assets/indexes/<id>.json
pub fn asset_index_item(version: &VersionJson, assets_dir: &PathBuf) -> DownloadItem {
    let idx = &version.asset_index;
    DownloadItem {
        url: idx.url.clone(),
        sha1: idx.sha1.clone(),
        size: idx.size,
        dest: assets_dir.join("indexes").join(format!("{}.json", idx.id)),
    }
}

/// 加载已下载的 assetIndex.json
pub fn load_asset_index(path: &std::path::Path) -> Result<AssetIndexFile, RmclError> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// 由 assetIndex 生成全部 object 下载项:dest = assets/objects/<h[0..2]>/<hash>
pub fn asset_items(index: &AssetIndexFile, assets_dir: &PathBuf) -> Vec<DownloadItem> {
    let objects_dir = assets_dir.join("objects");
    index
        .objects
        .iter()
        .map(|(_, obj)| {
            let prefix = &obj.hash[..2.min(obj.hash.len())];
            DownloadItem {
                url: format!("{ASSET_BASE}/{prefix}/{}", obj.hash),
                sha1: obj.hash.clone(),
                size: obj.size,
                dest: objects_dir.join(prefix).join(&obj.hash),
            }
        })
        .collect()
}
