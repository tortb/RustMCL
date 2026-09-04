//! 通用下载器:并发控制、SHA1 校验、失败重试
//! 子模块 asset.rs / library.rs 负责把 Mojang 元数据转换为 DownloadItem 列表

pub mod asset;
pub mod library;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use futures_util::StreamExt;
use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

use crate::core::mirror::Mirror;
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

/// 一次下载的整体统计(区分"命中缓存跳过"与"实际下载",便于日志/UI 呈现)
#[derive(Debug, Clone, Copy, Default)]
pub struct DownloadStats {
    pub total: usize,
    pub downloaded: usize,
    pub cached: usize,
}

/// 计算文件 SHA1(已存在文件,用于跳过校验)
pub fn sha1_of(path: &Path) -> Result<String, RmclError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// 下载单个文件:已存在且校验通过则跳过(返回 false);否则下载到 .part 临时文件,
/// 校验 SHA1 后原子改名(返回 true)。失败按 retry_times 重试。
pub async fn download_one(
    client: &reqwest::Client,
    mirror: &Mirror,
    item: &DownloadItem,
    retry_times: u32,
) -> Result<bool, RmclError> {
    // 已存在:有 sha1 则校验,无 sha1(maven 库)只做存在性判断
    if item.dest.exists() {
        if item.sha1.is_empty() || sha1_of(&item.dest).map(|h| h == item.sha1).unwrap_or(false) {
            return Ok(false);
        }
    }
    if let Some(parent) = item.dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = item.dest.with_extension("part");
    let url = mirror.rewrite(&item.url);
    let mut last_err = None;
    for attempt in 0..=retry_times {
        match fetch_to_file(client, &url, &tmp).await {
            Ok(()) => {
                // 无 sha1 的 maven 库直接采用,有 sha1 则校验
                if item.sha1.is_empty() {
                    tokio::fs::rename(&tmp, &item.dest).await?;
                    return Ok(true);
                }
                match sha1_of(&tmp) {
                    Ok(hash) if hash == item.sha1 => {
                        tokio::fs::rename(&tmp, &item.dest).await?;
                        return Ok(true);
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

/// 并发下载整个列表,每个文件完成后回调一次进度;返回整体统计(缓存命中/实际下载)。
/// `cancel` 提供取消令牌(为 None 时行为与旧版一致):每个文件开始前检查,已置位则返回 `Cancelled`。
pub async fn download_many<F>(
    client: &reqwest::Client,
    mirror: &Mirror,
    items: Vec<DownloadItem>,
    max_concurrent: usize,
    retry_times: u32,
    cancel: Option<Arc<AtomicBool>>,
    on_progress: F,
) -> Result<DownloadStats, RmclError>
where
    F: Fn(DownloadProgress) + Send + Sync + 'static,
{
    let total = items.len();
    if total == 0 {
        return Ok(DownloadStats::default());
    }
    let on_progress = Arc::new(on_progress);
    let done = Arc::new(AtomicUsize::new(0));
    let cached = Arc::new(AtomicUsize::new(0));
    let downloaded = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(max_concurrent.max(1)));

    let mut handles = Vec::with_capacity(total);
    for item in items {
        let client = client.clone();
        let mirror = mirror.clone();
        let semaphore = semaphore.clone();
        let on_progress = on_progress.clone();
        let done = done.clone();
        let cached = cached.clone();
        let downloaded = downloaded.clone();
        let cancel = cancel.clone();
        handles.push(tokio::spawn(async move {
            // 取消优先检查:已置位则跳过后续所有文件,尽快中止
            if cancel.as_ref().map_or(false, |c| c.load(Ordering::SeqCst)) {
                return Err(RmclError::Cancelled);
            }
            let _permit = semaphore
                .acquire()
                .await
                .map_err(|_| RmclError::other("并发限制信号量关闭"))?;
            let was_downloaded = download_one(&client, &mirror, &item, retry_times).await?;
            if was_downloaded {
                downloaded.fetch_add(1, Ordering::SeqCst);
            } else {
                cached.fetch_add(1, Ordering::SeqCst);
            }
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
            Ok::<_, RmclError>(was_downloaded)
        }));
    }
    for handle in handles {
        handle
            .await
            .map_err(|e| RmclError::other(format!("下载任务异常: {e}")))??;
    }
    Ok(DownloadStats {
        total,
        downloaded: downloaded.load(Ordering::SeqCst),
        cached: cached.load(Ordering::SeqCst),
    })
}
