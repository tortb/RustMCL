//! Mod 集成:数据来源(Modrinth)与安装逻辑

pub mod modrinth;

use std::path::Path;

use crate::core::downloader::{download_one, DownloadItem};
use crate::error::RunaError;

use self::modrinth::ModrinthVersion;

/// 下载 mod 到目标目录,返回最终文件名(幂等:同文件已存在则跳过)
pub async fn install_version(
    client: &reqwest::Client,
    version: &ModrinthVersion,
    dest_dir: &Path,
    retry_times: u32,
) -> Result<String, RunaError> {
    let file = modrinth::primary_file(version)
        .ok_or_else(|| RunaError::other(format!("版本 {} 没有可下载的文件", version.id)))?;
    let filename = &file.filename;
    // Modrinth 文件名含路径时只取最后一段
    let final_name = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .to_string();
    let dest = dest_dir.join(&final_name);
    let item = DownloadItem {
        url: file.url.clone(),
        sha1: file.hashes.get("sha1").cloned().unwrap_or_default(),
        size: file.size,
        dest: dest.clone(),
    };
    download_one(client, &item, retry_times).await?;
    Ok(final_name)
}
