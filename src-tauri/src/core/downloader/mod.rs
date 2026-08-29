//! 通用下载器:并发控制、SHA1 校验、失败重试
//! 子模块 asset.rs / library.rs 负责把 Mojang 元数据转换为 DownloadItem 列表

pub mod asset;
pub mod library;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures_util::StreamExt;
use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

use crate::error::RmclError;

/// 单个待下载文件
#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub url: String,
    pub sha1: String,
    /// 声明大小(用于断点续传/进度展示,暂未消费)
    #[allow(dead_code)]
    pub size: i64,
    pub dest: PathBuf,
}

/// 下载进度回调数据
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub done: usize,
    pub total: usize,
    pub file: String,
}

/// 计算文件 SHA1(已存在文件,用于跳过校验)
pub fn sha1_of(path: &Path) -> Result<String, RmclError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// 下载单个文件:已存在且校验通过则跳过;否则下载到 .part 临时文件,
/// 校验 SHA1 后原子改名。失败按 retry_times 重试。
pub async fn download_one(
    client: &reqwest::Client,
    item: &DownloadItem,
    retry_times: u32,
) -> Result<(), RmclError> {
    // 已存在:有 sha1 则校验,无 sha1(maven 库)只做存在性判断
    if item.dest.exists() {
        if item.sha1.is_empty()
            || sha1_of(&item.dest).map(|h| h == item.sha1).unwrap_or(false)
        {
            return Ok(());
        }
    }
    if let Some(parent) = item.dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = item.dest.with_extension("part");
    let mut last_err = None;
    for attempt in 0..=retry_times {
        match fetch_to_file(client, &item.url, &tmp).await {
            Ok(()) => {
                // 无 sha1 的 maven 库直接采用,有 sha1 则校验
                if item.sha1.is_empty() {
                    tokio::fs::rename(&tmp, &item.dest).await?;
                    return Ok(());
                }
                match sha1_of(&tmp) {
                    Ok(hash) if hash == item.sha1 => {
                        tokio::fs::rename(&tmp, &item.dest).await?;
                        return Ok(());
                    }
                    Ok(_) => {
                        last_err = Some(RmclError::other(format!(
                            "SHA1 校验失败: {}",
                            item.dest.display()
                        )));
                    }
                    Err(e) => last_err = Some(e),
                }
            }
            Err(e) => last_err = Some(e),
        }
        if attempt < retry_times {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    let _ = tokio::fs::remove_file(&tmp).await;
    Err(last_err.unwrap_or_else(|| RmclError::other("下载失败")))
}

async fn fetch_to_file(client: &reqwest::Client, url: &str, dest: &Path) -> Result<(), RmclError> {
    let resp = client.get(url).send().await?.error_for_status()?;
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;
    Ok(())
}

/// 并发下载整个列表,每个文件完成后回调一次进度
pub async fn download_many<F>(
    client: &reqwest::Client,
    items: Vec<DownloadItem>,
    max_concurrent: usize,
    retry_times: u32,
    on_progress: F,
) -> Result<(), RmclError>
where
    F: Fn(DownloadProgress) + Send + Sync + 'static,
{
    let total = items.len();
    if total == 0 {
        return Ok(());
    }
    let on_progress = Arc::new(on_progress);
    let done = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(max_concurrent.max(1)));

    let mut handles = Vec::with_capacity(total);
    for item in items {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let on_progress = on_progress.clone();
        let done = done.clone();
        handles.push(tokio::spawn(async move {
            let _permit = semaphore
                .acquire()
                .await
                .map_err(|_| RmclError::other("并发限制信号量关闭"))?;
            let result = download_one(&client, &item, retry_times).await;
            let current = done.fetch_add(1, Ordering::SeqCst) + 1;
            let file = item
                .dest
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            on_progress(DownloadProgress {
                done: current,
                total,
                file,
            });
            result
        }));
    }
    for handle in handles {
        handle
            .await
            .map_err(|e| RmclError::other(format!("下载任务异常: {e}")))??;
    }
    Ok(())
}
