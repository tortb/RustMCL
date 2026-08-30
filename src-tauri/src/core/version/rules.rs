//! Mojang version.json 中的 rules(OS/arch/features 条件)解析与判断

use serde::{Deserialize, Serialize};

/// 条件上下文:当前系统信息 + 特性开关
#[derive(Debug, Clone)]
pub struct RuleContext {
    /// "linux" | "windows" | "osx"
    pub os_name: &'static str,
    /// "x86" | "amd64" | "arm64" ...
    pub os_arch: &'static str,
    /// Windows 版本号(如 "10.0");非 Windows 或未知时为空串
    pub os_version: String,
    pub features: FeaturesCtx,
}

#[derive(Debug, Clone, Default)]
pub struct FeaturesCtx {
    /// 当前启动上下文中开启的 feature 集合。
    /// 用集合而非固定字段,避免 version.json 新增 feature(如 quick play 系列)时
    /// 因字段缺失而被误判为"已启用",导致参数被错误注入。
    pub enabled: std::collections::HashSet<String>,
}

impl FeaturesCtx {
    pub fn custom_resolution(mut self, yes: bool) -> Self {
        set_feature(&mut self, "has_custom_resolution", yes);
        self
    }

    pub fn demo_user(mut self, yes: bool) -> Self {
        set_feature(&mut self, "is_demo_user", yes);
        self
    }
}

fn set_feature(ctx: &mut FeaturesCtx, name: &str, on: bool) {
    if on {
        ctx.enabled.insert(name.to_string());
    } else {
        ctx.enabled.remove(name);
    }
}

impl RuleContext {
    pub fn current(features: FeaturesCtx) -> Self {
        let os_name = match std::env::consts::OS {
            "macos" => "osx",
            other => other,
        };
        let os_arch = match std::env::consts::ARCH {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            other => other,
        };
        Self {
            os_name,
            os_arch,
            os_version: String::new(),
            features,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<FeaturesRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

/// 捕获 version.json 规则中声明的任意 feature 组合。
/// 不再只固定 demo_user / custom_resolution:若只识别这两个字段,
/// quick play(is_quick_play_* 等)会被当成"已填 false"而非"未启用",
/// 从而被误判为匹配。这里用 Map 保留全部字段,便于对未知 feature 做兜底判定。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeaturesRule {
    #[serde(flatten)]
    pub fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl FeaturesRule {
    /// 一条 rule 是否匹配当前 feature 上下文:
    /// 规则声明要求的每个 feature,都必须**处于开启状态**才匹配;
    /// 未在上下文启用的 feature(含完全未知的)一律视为不匹配。
    /// 例如 quick play 规则要求 is_quick_play_singleplayer: true,
    /// 但默认未启用该 feature => 规则不匹配 => 参数不会被注入。
    fn matches(&self, ctx: &RuleContext) -> bool {
        for (name, need) in &self.fields {
            let need_true = need.as_bool().unwrap_or(false);
            let enabled = ctx.features.enabled.contains(name);
            // 规则要求该 feature 处于某一状态,而当前上下文不满足时即整体不匹配。
            // 注意:未声明的 feature(如 quick play 系列)默认不启用,因此"要求启用"
            // 的规则不会命中 —— 这是修复 MC 1.21.11 崩溃的关键。
            // 同时兼容 `{"is_demo_user": false}` 这类"要求关闭"的规则。
            if enabled != need_true {
                return false;
            }
        }
        true
    }
}

impl Rule {
    pub fn applies(&self, ctx: &RuleContext) -> bool {
        if let Some(os) = &self.os {
            if let Some(name) = &os.name {
                if name != ctx.os_name {
                    return false;
                }
            }
            if let Some(arch) = &os.arch {
                if arch != ctx.os_arch {
                    return false;
                }
            }
            if os.version.is_some() {
                // 无法可靠获取 windows 版本号时,带 version 条件的规则一律不匹配
                if ctx.os_version.is_empty() {
                    return false;
                }
            }
        }
        if let Some(features) = &self.features {
            if !features.matches(ctx) {
                return false;
            }
        }
        true
    }
}

/// 判断一组规则是否允许:无规则视为允许;命中任意 disallow 则禁止;
/// 命中 allow 视为允许(存在 allow 规则时,不匹配任何 allow 视为禁止)
pub fn rules_allow(rules: Option<&[Rule]>, ctx: &RuleContext) -> bool {
    let rules = match rules {
        Some(r) if !r.is_empty() => r,
        _ => return true,
    };
    let mut allowed = false;
    for rule in rules {
        if rule.applies(ctx) {
            match rule.action {
                RuleAction::Allow => allowed = true,
                RuleAction::Disallow => return false,
            }
        }
    }
    allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> RuleContext {
        RuleContext::current(FeaturesCtx::default())
    }

    #[test]
    fn no_rules_means_allow() {
        assert!(rules_allow(None, &ctx()));
        assert!(rules_allow(Some(&[]), &ctx()));
    }

    #[test]
    fn os_name_matches_current() {
        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: Some(OsRule {
                name: Some(std::env::consts::OS.to_string()),
                arch: None,
                version: None,
            }),
            features: None,
        }];
        assert!(rules_allow(Some(&rules), &ctx()));
    }

    #[test]
    fn wrong_os_disallowed() {
        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: Some(OsRule {
                name: Some("windows".into()),
                arch: None,
                version: None,
            }),
            features: None,
        }];
        let allowed = rules_allow(Some(&rules), &ctx());
        assert_eq!(allowed, cfg!(target_os = "windows"));
    }

    #[test]
    fn disallow_wins() {
        let rules = vec![
            Rule {
                action: RuleAction::Allow,
                os: None,
                features: None,
            },
            Rule {
                action: RuleAction::Disallow,
                os: Some(OsRule {
                    name: Some(std::env::consts::OS.to_string()),
                    arch: None,
                    version: None,
                }),
                features: None,
            },
        ];
        assert!(!rules_allow(Some(&rules), &ctx()));
    }

    #[test]
    fn feature_resolution() {
        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: None,
            features: Some(FeaturesRule {
                fields: [("has_custom_resolution".into(), serde_json::json!(true))]
                    .into_iter()
                    .collect(),
            }),
        }];
        // 未启用该 feature => 规则不匹配
        let c = ctx();
        assert!(!rules_allow(Some(&rules), &c));
        // 启用后 => 规则匹配
        let c = RuleContext::current(FeaturesCtx::default().custom_resolution(true));
        assert!(rules_allow(Some(&rules), &c));
    }

    #[test]
    fn quick_play_features_not_enabled_are_not_matched() {
        // 复现 crash 场景:MC 1.21.11 的 quick play 规则引用了多个 feature,
        // 未启用时应全部不匹配,而不是都被注入(否则 MC 抛 "Only one quick play option")。
        let make = |name: &str| {
            vec![Rule {
                action: RuleAction::Allow,
                os: None,
                features: Some(FeaturesRule {
                    fields: [(name.to_string(), serde_json::json!(true))].into_iter().collect(),
                }),
            }]
        };
        for name in [
            "has_quick_plays_support",
            "is_quick_play_singleplayer",
            "is_quick_play_multiplayer",
            "is_quick_play_realms",
        ] {
            let rules = make(name);
            assert!(
                !rules_allow(Some(&rules), &ctx()),
                "{name} 未启用时不应匹配"
            );
        }
    }

    #[test]
    fn feature_requires_false_matches_when_not_enabled() {
        // 兼容 Mojang 的 `{"features":{"is_demo_user":false}}` 规则:
        // "当该 feature 处于关闭状态时允许"。默认未启用 => 应匹配(而非被误判为永不匹配)。
        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: None,
            features: Some(FeaturesRule {
                fields: [("is_demo_user".into(), serde_json::json!(false))]
                    .into_iter()
                    .collect(),
            }),
        }];
        // 未启用 demo => 规则命中
        assert!(rules_allow(Some(&rules), &ctx()));
        // 启用 demo => 规则不命中
        let c = RuleContext::current(FeaturesCtx::default().demo_user(true));
        assert!(!rules_allow(Some(&rules), &c));
    }
}
