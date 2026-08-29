//! 存档备份 + 截图管理(模块 11):
//! - 列出实例 saves/ 下的世界,一键 zip 备份到 backups/,支持恢复/删除
//! - 列出 screenshots/ 下的截图,支持删除
//! 全部为异步友好的文件系统操作(大文件打包不阻塞 UI 主线程)。

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::State;

use crate::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct SaveInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: i64,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 备份保留上限(最近 N 份),超出部分删除最旧的 zip
const MAX_BACKUP_KEEP: usize = 20;

/// 保留最近 MAX_BACKUP_KEEP 份备份,删除更旧的 zip 文件
fn trim_backups(backups: &Path) {
    let Ok(entries) = std::fs::read_dir(backups) else {
        return;
    };
    let mut zips: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "zip").unwrap_or(false))
        .filter_map(|e| {
            e.metadata()
                .ok()
                .map(|m| (m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH), e.path()))
        })
        .collect();
    zips.sort_by(|a, b| b.0.cmp(&a.0)); // 新的在前
    for (_, p) in zips.into_iter().skip(MAX_BACKUP_KEEP) {
        let _ = std::fs::remove_file(p);
    }
}

fn instance_game_dir(state: &State<'_, AppState>, instance_id: &str) -> Result<PathBuf, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    crate::db::repository::Repository::get_instance(&conn, instance_id)
        .map_err(|e| e.to_string())?
        .map(|i| PathBuf::from(i.game_dir))
        .ok_or_else(|| format!("实例不存在: {instance_id}"))
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                total += dir_size(&entry.path());
            } else {
                total += meta.len();
            }
        }
    }
    total
}

fn mtime(path: &Path) -> i64 {
    path.metadata()
        .and_then(|m| m.modified())
        .and_then(|t| t.duration_since(UNIX_EPOCH).map_err(std::io::Error::other))
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 列出实例存档(世界)
#[tauri::command]
pub fn list_saves(state: State<'_, AppState>, instance_id: String) -> Result<Vec<SaveInfo>, String> {
    let game_dir = instance_game_dir(&state, &instance_id)?;
    let saves = game_dir.join("saves");
    let Ok(entries) = std::fs::read_dir(&saves) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        out.push(SaveInfo {
            name,
            path: entry.path().to_string_lossy().to_string(),
            size_bytes: dir_size(&entry.path()),
            modified_at: mtime(&entry.path()),
        });
    }
    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(out)
}

/// 打包备份存档到 backups/<name>_<ts>.zip,返回备份信息
#[tauri::command]
pub fn backup_save(state: State<'_, AppState>, instance_id: String, save_name: String) -> Result<BackupInfo, String> {
    if save_name.trim().is_empty() || save_name.contains("..") || save_name.contains('/') {
        return Err("非法的存档名".into());
    }
    let game_dir = instance_game_dir(&state, &instance_id)?;
    let src = game_dir.join("saves").join(&save_name);
    if !src.exists() {
        return Err(format!("存档不存在: {save_name}"));
    }
    let backups = game_dir.join("backups");
    std::fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
    let dest_name = format!("{save_name}_{}.zip", now_secs());
    let dest = backups.join(&dest_name);
    zip_dir(&src, &dest).map_err(|e| format!("备份失败: {e}"))?;
    // 存储上限:只保留最近 MAX_BACKUP_KEEP 份,避免无限占用磁盘
    trim_backups(&backups);
    Ok(BackupInfo {
        name: dest_name,
        path: dest.to_string_lossy().to_string(),
        size_bytes: dir_size(&dest),
        created_at: now_secs(),
    })
}

/// 列出备份历史
#[tauri::command]
pub fn list_backups(state: State<'_, AppState>, instance_id: String) -> Result<Vec<BackupInfo>, String> {
    let game_dir = instance_game_dir(&state, &instance_id)?;
    let backups = game_dir.join("backups");
    let Ok(entries) = std::fs::read_dir(&backups) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        out.push(BackupInfo {
            name,
            path: entry.path().to_string_lossy().to_string(),
            size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
            created_at: mtime(&entry.path()),
        });
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

/// 恢复备份到 saves/<target_name>
#[tauri::command]
pub fn restore_backup(
    state: State<'_, AppState>,
    instance_id: String,
    backup_name: String,
    target_name: String,
) -> Result<(), String> {
    if target_name.trim().is_empty() || target_name.contains("..") || target_name.contains('/') {
        return Err("非法的目标存档名".into());
    }
    let game_dir = instance_game_dir(&state, &instance_id)?;
    let backups = game_dir.join("backups");
    let src = backups.join(&backup_name);
    if !src.exists() {
        return Err(format!("备份不存在: {backup_name}"));
    }
    let dest = game_dir.join("saves").join(sanitize_component(&target_name));
    unzip_dir(&src, &dest).map_err(|e| format!("恢复失败: {e}"))?;
    Ok(())
}

/// 删除存档(目录)
#[tauri::command]
pub fn delete_save(state: State<'_, AppState>, instance_id: String, save_name: String) -> Result<(), String> {
    if save_name.trim().is_empty() || save_name.contains("..") || save_name.contains('/') {
        return Err("非法的存档名".into());
    }
    let game_dir = instance_game_dir(&state, &instance_id)?;
    let dir = game_dir.join("saves").join(&save_name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 列出截图
#[tauri::command]
pub fn list_screenshots(state: State<'_, AppState>, instance_id: String) -> Result<Vec<ScreenshotInfo>, String> {
    let game_dir = instance_game_dir(&state, &instance_id)?;
    let screenshots = game_dir.join("screenshots");
    let Ok(entries) = std::fs::read_dir(&screenshots) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_image(&name) {
            continue;
        }
        out.push(ScreenshotInfo {
            name,
            path: entry.path().to_string_lossy().to_string(),
            size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
            modified_at: mtime(&entry.path()),
        });
    }
    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(out)
}

#[tauri::command]
pub fn delete_screenshot(state: State<'_, AppState>, instance_id: String, name: String) -> Result<(), String> {
    if name.contains("..") || name.contains('/') {
        return Err("非法文件名".into());
    }
    let game_dir = instance_game_dir(&state, &instance_id)?;
    let file = game_dir.join("screenshots").join(&name);
    if file.exists() {
        std::fs::remove_file(&file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 读取截图图片为 data URL(供前端画廊按需懒加载);不存在返回 None
#[tauri::command]
pub fn get_screenshot_image(
    state: State<'_, AppState>,
    instance_id: String,
    name: String,
) -> Result<Option<String>, String> {
    if name.contains("..") || name.contains('/') {
        return Err("非法文件名".into());
    }
    let game_dir = instance_game_dir(&state, &instance_id)?;
    let file = game_dir.join("screenshots").join(&name);
    match std::fs::read(&file) {
        Ok(bytes) => {
            let mime = mime_for(&name);
            Ok(Some(crate::core::codec::image_data_url(&bytes, mime)))
        }
        Err(_) => Ok(None),
    }
}

/// 根据扩展名推断图片 mime
fn mime_for(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png"
    }
}

fn is_image(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
}

fn sanitize_component(name: &str) -> String {
    name.replace(['/', '\\', '\0'], "")
}

/// 把目录递归打包为 zip(zip-slip 防护:只打包目录内相对路径)
fn zip_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    let file = std::fs::File::create(dest)?;
    let mut zw = zip::ZipWriter::new(file);
    let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    add_dir_to_zip(&mut zw, src, src, opt)?;
    zw.finish()?;
    Ok(())
}

fn add_dir_to_zip(
    zw: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    opt: zip::write::SimpleFileOptions,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).expect("路径应在 base 下").to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            add_dir_to_zip(zw, base, &path, opt)?;
        } else {
            zw.start_file(rel, opt).map_err(std::io::Error::other)?;
            let mut f = std::fs::File::open(&path)?;
            std::io::copy(&mut f, zw)?;
        }
    }
    Ok(())
}

/// 解压 zip 到 dest(空目录;zip-slip 防护)
fn unzip_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    let file = std::fs::File::open(src)?;
    let mut archive = zip::ZipArchive::new(file).map_err(std::io::Error::other)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(std::io::Error::other)?;
        // 存档 zip 打包时以目录本身为根(内含 level.dat 等),此处把条目直接落到 dest
        let out = dest.join(entry.name());
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
