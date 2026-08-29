//! T5.1 Forge 版本清单获取(promotions_slim.json)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::core::mirror::Mirror;
use crate::error::RmclError;

use super::{fetch_body, PROMOTIONS_URL};

/// 单个候选 Forge 版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeVersionInfo {
    pub version: String,
    pub is_recommended: bool,
    pub is_latest: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct Promotions {
    #[serde(default)]
    promos: BTreeMap<String, String>,
}

/// 解析 promotions_slim.json,返回指定 MC 版本可用的 Forge 版本(按 recommended/最新标记)
pub fn parse_promotions(promos: &BTreeMap<String, String>, mc_version: &str) -> Vec<ForgeVersionInfo> {
    let mut map: BTreeMap<String, (bool, bool)> = BTreeMap::new();
    for (key, version) in promos {
        // key 形如 "1.20.1-recommended" / "1.20.1-latest"
        let Some((mc, flag)) = key.rsplit_once('-') else { continue };
        if mc != mc_version {
            continue;
        }
        let entry = map.entry(version.clone()).or_insert((false, false));
        match flag {
            "recommended" => entry.0 = true,
            "latest" => entry.1 = true,
            _ => {}
        }
    }
    // 推荐版本排前,其次按版本号降序
    let mut list: Vec<ForgeVersionInfo> = map
        .into_iter()
        .map(|(version, (is_recommended, is_latest))| ForgeVersionInfo {
            version,
            is_recommended,
            is_latest,
        })
        .collect();
    list.sort_by(|a, b| {
        b.is_recommended
            .cmp(&a.is_recommended)
            .then_with(|| b.version.cmp(&a.version))
    });
    list
}

/// 拉取并解析 Forge 版本清单
pub async fn list_forge_versions(
    client: &reqwest::Client,
    mirror: &Mirror,
    mc_version: &str,
    retry_times: u32,
) -> Result<Vec<ForgeVersionInfo>, RmclError> {
    let body = fetch_body(client, mirror, PROMOTIONS_URL, retry_times).await?;
    let promos: Promotions = serde_json::from_str(&body)?;
    Ok(parse_promotions(&promos.promos, mc_version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn sample() -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("1.20.1-recommended".into(), "47.2.0".into());
        m.insert("1.20.1-latest".into(), "47.3.0".into());
        m.insert("1.21.1-recommended".into(), "51.0.4".into());
        m.insert("1.12.2-recommended".into(), "14.23.5.2860".into());
        m.insert("1.20.1-xyz".into(), "47.2.0".into()); // 非标 flag,忽略
        m
    }

    #[test]
    fn extracts_recommended_marked_version() {
        let list = parse_promotions(&sample(), "1.20.1");
        // 47.2.0 既是 recommended;47.3.0 是 latest
        let rec = list.iter().find(|v| v.version == "47.2.0").unwrap();
        assert!(rec.is_recommended);
        let latest = list.iter().find(|v| v.version == "47.3.0").unwrap();
        assert!(latest.is_latest);
        // recommended 排在 latest 之前
        assert_eq!(list[0].version, "47.2.0");
    }

    #[test]
    fn no_version_for_unknown_mc() {
        assert!(parse_promotions(&sample(), "1.20.2").is_empty());
    }

    #[test]
    fn parses_promotions_map() {
        let body = r#"{"promos": {"1.20.1-recommended": "47.2.0", "1.20.1-latest": "47.3.0"}}"#;
        let p: Promotions = serde_json::from_str(body).unwrap();
        assert_eq!(p.promos.len(), 2);
        assert_eq!(p.promos.get("1.20.1-recommended").unwrap(), "47.2.0");
    }
}
