//! 整合包导入/导出:
//! - 导入:识别 Modrinth `.mrpack`(含 download URL)或 CurseForge 整合包(manifest.json + 需 API Key 解析文件)
//! - 导出:把实例已安装的 mod + 加载器信息打包为 `.mrpack`
//! overrides 文件夹(config/、resourcepacks/ 等)会完整复制到实例的 game_dir。

#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::downloader::{sha1_of, DownloadItem};
use crate::core::mirror::Mirror;
use crate::error::RmclError;

const FORGE_API_BASE: &str = "https://api.curseforge.com/v1";

/// 整合包来源
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackSource {
    Modrinth,
    CurseForge,
}

impl PackSource {
    pub fn label(&self) -> &'static str {
        match self {
            PackSource::Modrinth => "modrinth",
            PackSource::CurseForge => "curseforge",
        }
    }
}

/// 需要下载的单个文件
#[derive(Debug, Clone)]
pub struct ModDownload {
    pub rel_path: String,
    pub url: String,
    pub sha1: String,
}

/// 解析后的整合包元数据
#[derive(Debug, Clone)]
pub struct ModpackInfo {
    pub source: PackSource,
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: String,
    /// zip 内 overrides 目录的相对路径
    pub overrides_relative: String,
    /// 已解析的可下载文件(mrpack);CurseForge 在安装时再解析
    pub downloads: Vec<ModDownload>,
    /// CurseForge 原始文件引用(project_id, file_id, required)
    pub curse_files: Vec<CurseFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurseFile {
    #[serde(rename = "projectID")]
    pub project_id: u64,
    #[serde(rename = "fileID")]
    pub file_id: u64,
    #[serde(default)]
    pub required: bool,
}

// ---------- Modrinth mrpack ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpackIndex {
    #[serde(rename = "formatVersion")]
    pub format_version: u64,
    #[serde(default)]
    pub game: String,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub files: Vec<MrpackFile>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpackFile {
    pub path: String,
    #[serde(default)]
    pub hashes: HashMap<String, String>,
    #[serde(default)]
    pub downloads: Vec<String>,
}

// ---------- CurseForge manifest ----------

#[derive(Debug, Clone, Deserialize)]
pub struct CurseManifest {
    #[serde(rename = "manifestType")]
    pub manifest_type: String,
    #[serde(rename = "manifestVersion")]
    pub manifest_version: u64,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub files: Vec<CurseFile>,
    #[serde(default)]
    pub overrides: String,
    pub minecraft: CurseMinecraft,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseMinecraft {
    pub version: String,
    #[serde(default)]
    pub mod_loaders: Vec<CurseLoader>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurseLoader {
    pub id: String,
}

// ---------- 读取 zip 内文本 ----------

fn read_zip_entry(pack: &Path, name: &str) -> Result<String, RmclError> {
    let file = std::fs::File::open(pack)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = archive
        .by_name(name)
        .map_err(|_| RmclError::other(format!("整合包缺少条目: {name}")))?;
    let mut s = String::new();
    entry.read_to_string(&mut s)?;
    Ok(s)
}

fn has_entry(pack: &Path, name: &str) -> bool {
    std::fs::File::open(pack)
        .ok()
        .and_then(|f| zip::ZipArchive::new(f).ok())
        .map(|mut z| z.by_name(name).is_ok())
        .unwrap_or(false)
}

/// 识别整合包类型
pub fn detect(pack: &Path) -> Option<PackSource> {
    if has_entry(pack, "modrinth.index.json") {
        Some(PackSource::Modrinth)
    } else if has_entry(pack, "manifest.json") {
        Some(PackSource::CurseForge)
    } else {
        None
    }
}

/// 解析整合包(按类型)
pub fn parse(pack: &Path) -> Result<ModpackInfo, RmclError> {
    match detect(pack) {
        Some(PackSource::Modrinth) => parse_mrpack(pack),
        Some(PackSource::CurseForge) => parse_curseforge(pack),
        None => Err(RmclError::other(
            "无法识别整合包类型:缺少 modrinth.index.json 或 manifest.json",
        )),
    }
}

fn parse_mrpack(pack: &Path) -> Result<ModpackInfo, RmclError> {
    let raw = read_zip_entry(pack, "modrinth.index.json")?;
    let index: MrpackIndex = serde_json::from_str(&raw)?;
    let mc_version = index.dependencies.get("minecraft").cloned().unwrap_or_default();
    let (loader, loader_version) = loader_from_deps(&index.dependencies);
    let downloads: Vec<ModDownload> = index
        .files
        .iter()
        .filter_map(|f| {
            let url = f.downloads.first()?.clone();
            let sha1 = f.hashes.get("sha1").cloned().unwrap_or_default();
            Some(ModDownload {
                rel_path: f.path.clone(),
                url,
                sha1,
            })
        })
        .collect();
    Ok(ModpackInfo {
        source: PackSource::Modrinth,
        name: index.name,
        mc_version,
        loader,
        loader_version,
        overrides_relative: "overrides".into(),
        downloads,
        curse_files: Vec::new(),
    })
}

fn parse_curseforge(pack: &Path) -> Result<ModpackInfo, RmclError> {
    let raw = read_zip_entry(pack, "manifest.json")?;
    let man: CurseManifest = serde_json::from_str(&raw)?;
    let mc = man.minecraft.version.clone();
    let (loader, loader_version) = man
        .minecraft
        .mod_loaders
        .first()
        .map(|l| split_loader_id(&l.id))
        .unwrap_or(("vanilla".into(), String::new()));
    Ok(ModpackInfo {
        source: PackSource::CurseForge,
        name: man.name.clone(),
        mc_version: mc,
        loader,
        loader_version,
        overrides_relative: if man.overrides.trim().is_empty() {
            "overrides".into()
        } else {
            man.overrides.clone()
        },
        downloads: Vec::new(),
        curse_files: man.files,
    })
}

/// 从 mrpack dependencies 中提取加载器,如 fabric-loader -> ("fabric", "0.16.9")
fn loader_from_deps(deps: &HashMap<String, String>) -> (String, String) {
    for (k, v) in deps {
        match k.as_str() {
            "fabric-loader" => return ("fabric".into(), v.clone()),
            "quilt-loader" => return ("quilt".into(), v.clone()),
            "forge" => return ("forge".into(), v.clone()),
            "neoforge" => return ("neoforge".into(), v.clone()),
            _ => {}
        }
    }
    ("vanilla".into(), String::new())
}

/// 解析 CurseForge modLoaders 的 id,如 "fabric-0.16.9" / "forge-47.2.0"
fn split_loader_id(id: &str) -> (String, String) {
    let id = id.trim();
    if let Some((loader, ver)) = id.split_once('-') {
        if !ver.is_empty() {
            return (loader.to_string(), ver.to_string());
        }
    }
    (id.to_string(), String::new())
}

/// 校验整合包与目标实例的 loader/mc 兼容性;不匹配返回 Err(提前提示,避免装到一半失败)
pub fn validate(info: &ModpackInfo, mc_version: &str, loader: &str) -> Result<(), RmclError> {
    if !info.mc_version.is_empty() && info.mc_version != mc_version {
        return Err(RmclError::other(format!(
            "整合包要求 MC {} ,当前实例为 {}",
            info.mc_version, mc_version
        )));
    }
    if info.loader != "vanilla" && info.loader != loader {
        return Err(RmclError::other(format!(
            "整合包需要加载器 {} ,当前实例为 {}",
            info.loader, loader
        )));
    }
    Ok(())
}

/// 安装结果:成功安装的文件 + 失败清单
#[derive(Debug, Clone, Serialize)]
pub struct PackInstallResult {
    pub installed: Vec<String>,
    pub failures: Vec<String>,
}

/// 把整合包安装到实例 game_dir:下载文件 -> 复制 overrides
pub async fn install_pack<F>(
    client: &reqwest::Client,
    mirror: &Mirror,
    info: &ModpackInfo,
    pack_path: &Path,
    game_dir: &Path,
    retry_times: u32,
    max_concurrent: usize,
    curseforge_key: Option<&str>,
    on_progress: F,
) -> Result<PackInstallResult, RmclError>
where
    F: Fn(usize, usize, String) + Send + Sync + 'static,
{
    // 导出/安装均为一次性操作,此参数保留接口一致性;当前实现串行下载以便逐文件收集失败
    let _ = max_concurrent;

    // 1. 解析下载列表(mrpack 直接用;CurseForge 走 API)
    let mut downloads = info.downloads.clone();
    if info.source == PackSource::CurseForge {
        if curseforge_key.is_none() {
            return Err(RmclError::other(
                "CurseForge 整合包需要配置 API Key(设置页 → 网络)才能自动解析下载地址",
            ));
        }
        let key = curseforge_key.unwrap();
        for f in &info.curse_files {
            match resolve_curseforge_file(client, key, f.project_id, f.file_id).await {
                Ok(Some(d)) => downloads.push(d),
                Ok(None) => continue,
                Err(e) => return Err(e),
            }
        }
    }

    // 2. 逐文件下载(失败不中断,汇入 failures 清单,便于用户手动补齐)
    let mut installed: Vec<String> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let total = downloads.len();
    let mut done = 0usize;
    for d in &downloads {
        let rel = sanitize_rel(&d.rel_path);
        if rel.is_empty() {
            continue;
        }
        let dest = game_dir.join(&rel);
        let item = DownloadItem {
            url: d.url.clone(),
            sha1: d.sha1.clone(),
            size: 0,
            dest: dest.clone(),
        };
        let ok = crate::core::downloader::download_one(client, mirror, &item, retry_times).await;
        done += 1;
        let file_label = rel.rsplit('/').next().unwrap_or(&rel).to_string();
        on_progress(done, total, file_label.clone());
        match ok {
            Ok(_) => {
                if rel.starts_with("mods/") {
                    installed.push(file_label);
                }
            }
            Err(e) => failures.push(format!("{file_label}: {e}")),
        }
    }

    // 3. 复制 overrides(config/、resourcepacks/ 等)
    apply_overrides(pack_path, &info.overrides_relative, game_dir)?;

    Ok(PackInstallResult {
        installed,
        failures,
    })
}

/// 从 zip 中把 `<overrides>/...` 完整复制到 game_dir(含子目录,已做 zip-slip 防护)
pub fn apply_overrides(pack_path: &Path, overrides_rel: &str, game_dir: &Path) -> Result<(), RmclError> {
    let file = std::fs::File::open(pack_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let prefix = overrides_rel.trim_end_matches('/');
    let mut copied = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(name) = entry.enclosed_name() else { continue };
        let name_str = name.to_string_lossy().replace('\\', "/");
        let Some(rel) = name_str.strip_prefix(&format!("{prefix}/")) else {
            continue;
        };
        let out = game_dir.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out_file = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut out_file)?;
        copied += 1;
    }
    if copied == 0 {
        // overrides 目录不存在或为空时静默成功(很多整合包没有 overrides)
        return Ok(());
    }
    Ok(())
}

/// 通过 CurseForge API 解析单个文件的下载地址;无 downloadUrl 时返回 None
async fn resolve_curseforge_file(
    client: &reqwest::Client,
    api_key: &str,
    project_id: u64,
    file_id: u64,
) -> Result<Option<ModDownload>, RmclError> {
    let url = format!("{FORGE_API_BASE}/mods/{project_id}/files/{file_id}");
    let resp = client
        .get(&url)
        .header("x-api-key", api_key)
        .send()
        .await?
        .error_for_status()?;
    let body: serde_json::Value = resp.json().await?;
    let data = body.get("data").ok_or_else(|| {
        RmclError::other(format!("CurseForge 返回异常(project {project_id}, file {file_id})"))
    })?;
    let file_name = data.get("fileName").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let download_url = data.get("downloadUrl").and_then(|v| v.as_str());
    let sha1 = data
        .get("hashes")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|h| h.get("algo").and_then(|a| a.as_i64()) == Some(1))
                .or_else(|| arr.first())
        })
        .and_then(|h| h.get("value").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();
    let url = match download_url {
        Some(u) if !u.is_empty() => u.to_string(),
        _ => {
            let id = data.get("id").and_then(|v| v.as_i64()).unwrap_or(file_id as i64);
            format!("https://mediafilez.forgecdn.net/files/{}/{}", id / 1000, id)
        }
    };
    if file_name.is_empty() {
        return Ok(None);
    }
    Ok(Some(ModDownload {
        rel_path: format!("mods/{file_name}"),
        url,
        sha1,
    }))
}

// ---------- 导出 ----------

/// 导出打包:写入 modrinth.index.json(含本机 sha1)与实现,产出一个 mrpack zip
pub fn export_mrpack(
    game_dir: &Path,
    mc_version: &str,
    loader: &str,
    loader_version: &str,
    name: &str,
    mods: &[String],
    dest: &Path,
) -> Result<(), RmclError> {
    let mut files: Vec<MrpackFile> = Vec::new();
    for mod_name in mods {
        let path = game_dir.join("mods").join(mod_name);
        if !path.exists() {
            continue;
        }
        let sha1 = sha1_of(&path)?;
        files.push(MrpackFile {
            path: format!("mods/{mod_name}"),
            hashes: HashMap::from([("sha1".into(), sha1)]),
            downloads: Vec::new(),
        });
    }
    let mut dependencies = HashMap::new();
    dependencies.insert("minecraft".into(), mc_version.to_string());
    match loader {
        "fabric" => {
            dependencies.insert("fabric-loader".into(), loader_version.to_string());
        }
        "quilt" => {
            dependencies.insert("quilt-loader".into(), loader_version.to_string());
        }
        "forge" => {
            dependencies.insert("forge".into(), loader_version.to_string());
        }
        _ => {}
    }
    let index = MrpackIndex {
        format_version: 1,
        game: "minecraft".into(),
        name: name.to_string(),
        summary: format!("由 RustMCL 导出 (MC {mc_version})"),
        files,
        dependencies,
    };

    let file = std::fs::File::create(dest)?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip_writer
        .start_file("modrinth.index.json", options)
        .map_err(|e| RmclError::Zip(e))?;
    zip_writer
        .write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;
    zip_writer.finish().map_err(|e| RmclError::Zip(e))?;
    Ok(())
}

/// 规范化相对路径:去除前导 /、"."、".." 等危险段,空/越界返回空串
fn sanitize_rel(rel: &str) -> String {
    let normalized = rel.replace('\\', "/");
    let mut safe = PathBuf::new();
    for seg in normalized.split('/') {
        match seg {
            "" | "." | ".." => {}
            s => safe.push(s),
        }
    }
    safe.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_loader_id() {
        assert_eq!(split_loader_id("fabric-0.16.9"), ("fabric".into(), "0.16.9".into()));
        assert_eq!(split_loader_id("forge-47.2.0"), ("forge".into(), "47.2.0".into()));
        assert_eq!(split_loader_id("vanilla"), ("vanilla".into(), String::new()));
    }

    #[test]
    fn loader_from_deps_prefers_fabric() {
        let mut deps = HashMap::new();
        deps.insert("minecraft".into(), "1.21.1".into());
        deps.insert("fabric-loader".into(), "0.16.9".into());
        assert_eq!(loader_from_deps(&deps), ("fabric".into(), "0.16.9".into()));
    }

    #[test]
    fn validate_mismatch_rejected() {
        let info = ModpackInfo {
            source: PackSource::Modrinth,
            name: "x".into(),
            mc_version: "1.20.1".into(),
            loader: "fabric".into(),
            loader_version: "0.16".into(),
            overrides_relative: "overrides".into(),
            downloads: vec![],
            curse_files: vec![],
        };
        assert!(validate(&info, "1.21.1", "fabric").is_err());
        assert!(validate(&info, "1.20.1", "forge").is_err());
        assert!(validate(&info, "1.20.1", "fabric").is_ok());
    }

    #[test]
    fn sanitize_rel_removes_dangerous_segments() {
        assert_eq!(sanitize_rel("mods/sodium.jar"), "mods/sodium.jar");
        assert_eq!(sanitize_rel("../evil.jar"), "evil.jar");
        assert_eq!(sanitize_rel("config/a/b.txt"), "config/a/b.txt");
    }

    #[test]
    fn parses_mrpack_index() {
        let raw = r#"{
          "formatVersion": 1,
          "game": "minecraft",
          "name": "Test Pack",
          "summary": "s",
          "files": [
            {"path": "mods/sodium.jar", "hashes": {"sha1": "abc", "sha512": "def"}, "downloads": ["https://cdn/x.jar"]},
            {"path": "resourcepacks/rp.zip", "hashes": {"sha1": "zzz"}, "downloads": ["https://cdn/rp.zip"]}
          ],
          "dependencies": {"minecraft": "1.21.1", "fabric-loader": "0.16.9"}
        }"#;
        let idx: MrpackIndex = serde_json::from_str(raw).unwrap();
        assert_eq!(idx.dependencies.get("minecraft").unwrap(), "1.21.1");
        assert_eq!(idx.files.len(), 2);
        assert_eq!(idx.files[0].hashes.get("sha1").unwrap(), "abc");
    }

    #[test]
    fn parses_curse_manifest() {
        let raw = r#"{
          "manifestType": "minecraftModpack",
          "manifestVersion": 1,
          "name": "Cursed",
          "version": "1.0",
          "files": [{"projectID": 123, "fileID": 456, "required": true}],
          "overrides": "overrides",
          "minecraft": {"version": "1.20.1", "modLoaders": [{"id": "fabric-0.16.9", "primary": true}]}
        }"#;
        let man: CurseManifest = serde_json::from_str(raw).unwrap();
        assert_eq!(man.minecraft.version, "1.20.1");
        assert_eq!(man.files.len(), 1);
        assert_eq!(man.files[0].project_id, 123);
        assert_eq!(man.minecraft.mod_loaders[0].id, "fabric-0.16.9");
    }

    #[test]
    fn apply_overrides_copies_files() {
        // 构建一个含 overrides/ 的 zip
        let root = std::env::temp_dir().join(format!("rmcl_pack_{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&root).unwrap();
        let pack = root.join("pack.mrpack");
        let f = std::fs::File::create(&pack).unwrap();
        let mut zw = zip::ZipWriter::new(f);
        let opt = zip::write::SimpleFileOptions::default();
        // overrides/config/example.toml
        zw.start_file("overrides/config/example.toml", opt).unwrap();
        zw.write_all(b"key=value").unwrap();
        zw.start_file("overrides/resourcepacks/rp.zip", opt).unwrap();
        zw.write_all(b"zip").unwrap();
        // mods 文件(不应该被当作 overrides 复制)
        zw.start_file("mods/x.jar", opt).unwrap();
        zw.write_all(b"x").unwrap();
        zw.finish().unwrap();

        let game_dir = root.join("game");
        std::fs::create_dir_all(&game_dir).unwrap();
        apply_overrides(&pack, "overrides", &game_dir).unwrap();
        assert!(game_dir.join("config/example.toml").exists());
        assert!(game_dir.join("resourcepacks/rp.zip").exists());
        assert!(!game_dir.join("mods/x.jar").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
