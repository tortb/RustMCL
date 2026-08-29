//! 皮肤管理(模块 9)核心逻辑:本地皮肤库 + PNG 格式校验。
//! 纯业务逻辑,不依赖 tauri,便于单元测试。
//!
//! 目录结构(`data_dir/skins/<id>/`):
//!   - skin.png   皮肤文件(64x64 或 64x32)
//!   - meta.json  { name, model, width, height }

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::RmclError;

/// 本地皮肤条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinEntry {
    pub id: String,
    pub name: String,
    /// classic(64x64 或 64x32) / slim(64x64)
    pub model: String,
    pub width: u32,
    pub height: u32,
}

/// 皮肤库根目录
pub fn skins_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("skins")
}

/// 列出本地皮肤库,按导入时间排序(目录修改时间降序);损坏条目自动跳过
pub fn list_skins(data_dir: &Path) -> Result<Vec<SkinEntry>, RmclError> {
    let dir = skins_dir(data_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<(u64, SkinEntry)> = Vec::new();
    let read = fs::read_dir(&dir)
        .map_err(|e| RmclError::other(format!("读取皮肤库失败: {e}")))?;
    for item in read.flatten() {
        let p = item.path();
        if !p.is_dir() {
            continue;
        }
        let Ok(meta) = fs::read_to_string(p.join("meta.json")) else {
            continue;
        };
        let Ok(entry) = serde_json::from_str::<SkinEntry>(&meta) else {
            continue;
        };
        let mtime = item
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        entries.push((mtime, entry));
    }
    entries.sort_by(|a, b| b.0.cmp(&a.0)); // 新的在前
    Ok(entries.into_iter().map(|(_, e)| e).collect())
}

fn skin_dir(data_dir: &Path, id: &str) -> PathBuf {
    skins_dir(data_dir).join(id)
}

/// 读取皮肤 PNG 的尺寸;非 PNG / 宽高非法时返回错误
pub fn read_png_size(bytes: &[u8]) -> Result<(u32, u32), RmclError> {
    let sig: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    if bytes.len() < 24 || bytes[..8] != sig {
        return Err(RmclError::other("不是有效的 PNG 文件"));
    }
    // PNG 头: 8 字节签名 + 4 字节长度 + "IHDR" + 4 字节宽 + 4 字节高(均为大端)
    if &bytes[12..16] != b"IHDR" {
        return Err(RmclError::other("PNG 缺少 IHDR 块"));
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    // 仅允许 Minecraft 皮肤标准尺寸: 64x64(现代) 或 64x32(经典)
    if width != 64 || (height != 64 && height != 32) {
        return Err(RmclError::other(format!(
            "皮肤尺寸必须为 64x64 或 64x32,当前为 {width}x{height}"
        )));
    }
    Ok((width, height))
}

/// 校验皮肤文件并返回格式说明(model 是否合法由调用方决定,这里校验尺寸)
pub fn validate_skin(bytes: &[u8]) -> Result<(u32, u32), RmclError> {
    read_png_size(bytes)
}

/// 导入本地皮肤:复制 PNG 到皮肤库并写入 meta.json;name 为空时以文件名作为名字
pub fn import_skin(data_dir: &Path, src: &Path, name: &str, model: &str) -> Result<SkinEntry, RmclError> {
    let bytes = fs::read(src).map_err(|e| RmclError::other(format!("读取皮肤文件失败: {e}")))?;
    let (width, height) = validate_skin(&bytes)?;
    let dir = skins_dir(data_dir);
    fs::create_dir_all(&dir)
        .map_err(|e| RmclError::other(format!("创建皮肤库失败: {e}")))?;

    let id = uuid::Uuid::new_v4().to_string();
    let target = skin_dir(data_dir, &id);
    fs::create_dir_all(&target)
        .map_err(|e| RmclError::other(format!("创建皮肤目录失败: {e}")))?;

    let file_name = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("skin")
        .to_string();
    let entry = SkinEntry {
        id: id.clone(),
        name: if name.trim().is_empty() { file_name } else { name.trim().to_string() },
        model: model.to_string(),
        width,
        height,
    };
    fs::write(target.join("skin.png"), &bytes)
        .map_err(|e| RmclError::other(format!("保存皮肤失败: {e}")))?;
    fs::write(
        target.join("meta.json"),
        serde_json::to_string_pretty(&entry)
            .map_err(|e| RmclError::other(format!("序列化皮肤信息失败: {e}")))?,
    )
    .map_err(|e| RmclError::other(format!("保存皮肤信息失败: {e}")))?;
    Ok(entry)
}

/// 删除本地皮肤
pub fn remove_skin(data_dir: &Path, id: &str) -> Result<(), RmclError> {
    let dir = skin_dir(data_dir, id);
    if !dir.exists() {
        return Err(RmclError::other("皮肤不存在"));
    }
    fs::remove_dir_all(&dir)
        .map_err(|e| RmclError::other(format!("删除皮肤失败: {e}")))?;
    Ok(())
}

/// 读取皮肤 PNG 原始字节(上传与 3D 预览使用)
pub fn read_skin_png(data_dir: &Path, id: &str) -> Result<Vec<u8>, RmclError> {
    let p = skin_dir(data_dir, id).join("skin.png");
    fs::read(&p).map_err(|e| RmclError::other(format!("读取皮肤文件失败: {e}")))
}

/// 生成可注入 3D 预览 / 上传的 Base64 字符串
pub fn to_base64(bytes: &[u8]) -> String {
    crate::core::codec::base64_encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 构造一个仅含合法 IHDR 头的假 PNG(签名 + 长度 + IHDR + 宽 + 高)
    fn fake_png(width: u32, height: u32) -> Vec<u8> {
        let mut v = vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
        v.extend_from_slice(&13u32.to_be_bytes()); // IHDR length
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&width.to_be_bytes());
        v.extend_from_slice(&height.to_be_bytes());
        v
    }

    #[test]
    fn accepts_standard_sizes() {
        assert_eq!(read_png_size(&fake_png(64, 64)).unwrap(), (64, 64));
        assert_eq!(read_png_size(&fake_png(64, 32)).unwrap(), (64, 32));
    }

    #[test]
    fn rejects_bad_dimensions() {
        assert!(read_png_size(&fake_png(128, 128)).is_err());
        assert!(read_png_size(&fake_png(1, 1)).is_err());
    }

    #[test]
    fn rejects_non_png() {
        assert!(read_png_size(b"not a png").is_err());
    }
}
