//! 游戏子进程管理:stdout/stderr 逐行转发,退出码捕获

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::error::RunaError;

/// 启动子进程并逐行转发输出,等待退出后返回退出码
pub async fn launch_process<F>(java_path: &str, args: &[String], on_line: F) -> Result<i32, RunaError>
where
    F: Fn(String) + Send + Sync + 'static,
{
    let mut child = Command::new(java_path)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            RunaError::other(format!(
                "无法启动 Java 进程(路径: {java_path}): {e}。请确认已安装 Java 或在设置中指定路径"
            ))
        })?;

    let on_line = Arc::new(on_line);
    let stdout = child.stdout.take().expect("stdout 管道应存在");
    let stderr = child.stderr.take().expect("stderr 管道应存在");
    let h1 = tokio::spawn(read_lines(stdout, on_line.clone()));
    let h2 = tokio::spawn(read_lines(stderr, on_line.clone()));

    let status = child.wait().await?;
    let _ = h1.await;
    let _ = h2.await;
    Ok(status.code().unwrap_or(-1))
}

async fn read_lines<R: tokio::io::AsyncRead + Unpin>(reader: R, on_line: Arc<dyn Fn(String) + Send + Sync>) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        on_line(line);
    }
}
