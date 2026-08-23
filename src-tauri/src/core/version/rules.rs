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
    pub is_demo_user: bool,
    pub has_custom_resolution: bool,
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

/// 目前 Mojang 只用这两个 feature,其余字段缺失时该规则视为不匹配
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeaturesRule {
    #[serde(default)]
    pub is_demo_user: Option<bool>,
    #[serde(default)]
    pub has_custom_resolution: Option<bool>,
}

impl FeaturesRule {
    fn matches(&self, ctx: &RuleContext) -> bool {
        if let Some(need) = self.is_demo_user {
            if ctx.features.is_demo_user != need {
                return false;
            }
        }
        if let Some(need) = self.has_custom_resolution {
            if ctx.features.has_custom_resolution != need {
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
                has_custom_resolution: Some(true),
                ..Default::default()
            }),
        }];
        let mut c = ctx();
        c.features.has_custom_resolution = true;
        assert!(rules_allow(Some(&rules), &c));
        c.features.has_custom_resolution = false;
        assert!(!rules_allow(Some(&rules), &c));
    }
}
