//! 下载镜像层:把官方下载 URL 按资源类型重写到镜像源,并提供测速。
//! 内置官方 + BMCLAPI + MCBBS 预设,支持自定义源。
//! 重写规则依据 URL 主机推断资源类型(官方源不做任何改动)。

use std::time::Instant;

use serde::{Deserialize, Serialize};

/// 镜像源基址
pub const BMCLAPI_BASE: &str = "https://bmclapi2.bangbang93.com";
pub const MCBBS_BASE: &str = "https://download.mcbbs.net";

/// 一个可选的镜像源(用于前端展示)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorSpec {
    pub id: String,
    pub name: String,
    /// 基址;官方源为空串
    pub base: String,
}

/// 当前生效的镜像
#[derive(Debug, Clone, Serialize)]
pub struct Mirror {
    pub id: String,
    pub base: String,
}

impl Mirror {
    /// 由配置取值构造;`selected` 可为 "official" / "bmclapi" / "mcbbs" / "custom" / 其他(回退官方)
    pub fn from_config(selected: &str, custom_base: Option<&str>) -> Self {
        match selected {
            "bmclapi" => Self {
                id: "bmclapi".into(),
                base: BMCLAPI_BASE.into(),
            },
            "mcbbs" => Self {
                id: "mcbbs".into(),
                base: MCBBS_BASE.into(),
            },
            "custom" => Self {
                id: "custom".into(),
                base: custom_base.unwrap_or("").trim_end_matches('/').to_string(),
            },
            _ => Self {
                id: "official".into(),
                base: String::new(),
            },
        }
    }

    pub fn is_official(&self) -> bool {
        self.base.trim().is_empty()
    }

    /// 依据 URL 主机推断资源类型并重写;未匹配到镜像规则时原样返回(交由 SHA1 校验兜底)。
    pub fn rewrite(&self, url: &str) -> String {
        if self.is_official() {
            return url.to_string();
        }
        let base = self.base.trim_end_matches('/');

        // 1) 仅换主机的类型:版本清单 / 版本 JSON / AssetsIndex / client jar / java runtime / logging
        for host in [
            "piston-meta.mojang.com",
            "launchermeta.mojang.com",
            "launcher.mojang.com",
        ] {
            if let Some(m) = map_keep_path(url, host, base) {
                return m;
            }
        }

        // 2) 资源对象:resources.download.minecraft.net/<pre>/<hash> -> base/assets/<hash>
        if let Some(m) = map_asset(url, base) {
            return m;
        }

        // 3) maven 库:libraries / forge / (neo)forge / fabric maven -> base/maven/<path>
        for host in [
            "libraries.minecraft.net",
            "files.minecraftforge.net",
            "maven.minecraftforge.net",
            "maven.neoforged.net",
            "maven.fabricmc.net",
        ] {
            if let Some(m) = map_maven(url, host, base) {
                return m;
            }
        }

        // 4) 加载器 meta:fabric / quilt
        if let Some(m) = map_keep_path(url, "meta.fabricmc.net", &format!("{base}/fabric-meta")) {
            return m;
        }
        if let Some(m) = map_keep_path(url, "meta.quiltmc.org", &format!("{base}/quilt-meta")) {
            return m;
        }

        url.to_string()
    }
}

/// 仅替换主机、保留完整路径。
fn map_keep_path(url: &str, official_host: &str, replacement: &str) -> Option<String> {
    let len = host_marker(url, official_host)?;
    let rest = &url[len..]; // 以 '/' 开头,原样保留
    Some(format!("{}{}", replacement.trim_end_matches('/'), rest))
}

/// 主机标记的字节长度(用于截取原始 URL 路径);未命中返回 None
fn host_marker(url: &str, official_host: &str) -> Option<usize> {
    let https = format!("https://{official_host}");
    let http = format!("http://{official_host}");
    if url.starts_with(&https) {
        Some(https.len())
    } else if url.starts_with(&http) {
        Some(http.len())
    } else {
        None
    }
}

/// maven 库:libraries.minecraft.net/<path> 或 files.minecraftforge.net/maven/<path>
/// 统一去重前缀后落到 base/maven/<path>
fn map_maven(url: &str, official_host: &str, base: &str) -> Option<String> {
    let len = host_marker(url, official_host)?;
    let mut rest = &url[len..];
    if let Some(stripped) = rest.strip_prefix('/') {
        rest = stripped;
    }
    if let Some(stripped) = rest.strip_prefix("maven/") {
        rest = stripped;
    }
    Some(format!("{}/maven/{}", base.trim_end_matches('/'), rest))
}

/// 资源对象:取 URL 最后一个路径段作为 hash
fn map_asset(url: &str, base: &str) -> Option<String> {
    let len = host_marker(url, "resources.download.minecraft.net")?;
    let hash = url[len..]
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("");
    if hash.is_empty() {
        return None;
    }
    Some(format!("{}/assets/{}", base.trim_end_matches('/'), hash))
}

/// 镜像源预设列表
pub fn presets() -> Vec<MirrorSpec> {
    vec![
        MirrorSpec {
            id: "official".into(),
            name: "官方源".into(),
            base: String::new(),
        },
        MirrorSpec {
            id: "bmclapi".into(),
            name: "BMCLAPI".into(),
            base: BMCLAPI_BASE.into(),
        },
        MirrorSpec {
            id: "mcbbs".into(),
            name: "MCBBS".into(),
            base: MCBBS_BASE.into(),
        },
    ]
}

/// 测速结果
#[derive(Debug, Clone, Serialize)]
pub struct SpeedResult {
    pub id: String,
    pub name: String,
    pub base: String,
    pub latency_ms: u64,
    /// 吞吐量 KB/s
    pub throughput: f64,
    pub ok: bool,
    #[serde(default)]
    pub error: String,
}

/// 测速:拉取版本清单(轻量小文件),测延迟与吞吐。
/// 任一源失败不 panic,返回 ok=false 供前端展示降级。
pub async fn speed_test(client: &reqwest::Client, spec: &MirrorSpec) -> SpeedResult {
    let mirror = Mirror {
        id: spec.id.clone(),
        base: spec.base.clone(),
    };
    let url = mirror.rewrite(crate::core::version::manifest::MANIFEST_URL);
    let start = Instant::now();
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return failed(spec, e.to_string());
        }
    };
    let resp = match resp.error_for_status() {
        Ok(r) => r,
        Err(e) => return failed(spec, e.to_string()),
    };
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => return failed(spec, e.to_string()),
    };
    let elapsed = start.elapsed();
    let latency_ms = elapsed.as_millis() as u64;
    let throughput = (bytes.len() as f64 / 1024.0) / elapsed.as_secs_f64().max(1e-6);
    SpeedResult {
        id: spec.id.clone(),
        name: spec.name.clone(),
        base: spec.base.clone(),
        latency_ms,
        throughput,
        ok: true,
        error: String::new(),
    }
}

fn failed(spec: &MirrorSpec, error: String) -> SpeedResult {
    SpeedResult {
        id: spec.id.clone(),
        name: spec.name.clone(),
        base: spec.base.clone(),
        latency_ms: 0,
        throughput: 0.0,
        ok: false,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_is_noop() {
        let m = Mirror::from_config("official", None);
        assert!(m.is_official());
        assert_eq!(
            m.rewrite("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"),
            "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
        );
    }

    #[test]
    fn map_keep_path_rewrites_manifest() {
        let m = Mirror::from_config("bmclapi", None);
        assert_eq!(
            m.rewrite("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"),
            "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json"
        );
    }

    #[test]
    fn map_keep_path_rewrites_version_json_and_client() {
        let m = Mirror::from_config("bmclapi", None);
        assert_eq!(
            m.rewrite("https://launcher.mojang.com/v1/objects/abc/client.jar"),
            "https://bmclapi2.bangbang93.com/v1/objects/abc/client.jar"
        );
        assert_eq!(
            m.rewrite("https://piston-meta.mojang.com/version/1.21.1.json"),
            "https://bmclapi2.bangbang93.com/version/1.21.1.json"
        );
    }

    #[test]
    fn map_asset_uses_full_hash() {
        let m = Mirror::from_config("bmclapi", None);
        assert_eq!(
            m.rewrite("https://resources.download.minecraft.net/ab/abcdef0123456789"),
            "https://bmclapi2.bangbang93.com/assets/abcdef0123456789"
        );
    }

    #[test]
    fn map_maven_dedups_prefix() {
        let m = Mirror::from_config("bmclapi", None);
        // libraries.minecraft.net 无 maven 前缀
        assert_eq!(
            m.rewrite(
                "https://libraries.minecraft.net/com/mojang/brigadier/1.1.8/brigadier-1.1.8.jar"
            ),
            "https://bmclapi2.bangbang93.com/maven/com/mojang/brigadier/1.1.8/brigadier-1.1.8.jar"
        );
        // forge 带 maven 前缀,应去掉避免双 maven
        assert_eq!(
            m.rewrite("https://files.minecraftforge.net/maven/net/minecraftforge/forge/promotions_slim.json"),
            "https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/promotions_slim.json"
        );
    }

    #[test]
    fn fabric_meta_rewritten() {
        let m = Mirror::from_config("bmclapi", None);
        assert_eq!(
            m.rewrite("https://meta.fabricmc.net/v2/versions/loader/1.21.1"),
            "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader/1.21.1"
        );
    }

    #[test]
    fn unmapped_host_left_unchanged() {
        let m = Mirror::from_config("bmclapi", None);
        // Modrinth CDN 不在镜像规则内,原样返回
        assert_eq!(
            m.rewrite("https://cdn.modrinth.com/data/A/versions/v1/x.jar"),
            "https://cdn.modrinth.com/data/A/versions/v1/x.jar"
        );
    }

    #[test]
    fn custom_base_used() {
        let m = Mirror::from_config("custom", Some("https://my.mirror.example.com/"));
        assert_eq!(
            m.rewrite("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"),
            "https://my.mirror.example.com/mc/game/version_manifest_v2.json"
        );
    }
}
