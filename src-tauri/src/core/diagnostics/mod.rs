//! 崩溃日志分析:定位 crash-reports,用可扩展的规则库(json 规则文件)给出诊断建议。
//! 规则文件默认内嵌,也可在数据目录放 crash_rules.json 覆盖/扩展。

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use regex::Regex;
use serde::{Deserialize, Serialize};

/// 内嵌的默认规则(可在数据目录放 crash_rules.json 覆盖)
const DEFAULT_RULES: &str = include_str!("rules.json");

/// 数据目录下的用户规则文件
pub fn user_rules_path(data_dir: &Path) -> PathBuf {
    data_dir.join("crash_rules.json")
}

/// 一条崩溃规则:匹配任一 pattern(正则,若非法则退化为子串匹配)即命中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashRule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub patterns: Vec<String>,
    pub summary: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuleSet {
    #[serde(default)]
    rules: Vec<CrashRule>,
}

impl RuleSet {
    fn load_default() -> Self {
        serde_json::from_str(DEFAULT_RULES).unwrap_or(RuleSet { rules: Vec::new() })
    }

    /// 优先读数据目录下的用户规则文件(用于扩展/覆盖),否则用内嵌默认
    pub fn load(data_dir: &Path) -> Self {
        let path = user_rules_path(data_dir);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(s) = serde_json::from_str(&content) {
                return s;
            }
        }
        Self::load_default()
    }

    pub fn into_rules(self) -> Vec<CrashRule> {
        self.rules
    }
}

/// 加载规则列表:优先数据目录下的用户规则文件,否则用内嵌默认
pub fn load_rules(data_dir: &Path) -> Vec<CrashRule> {
    RuleSet::load(data_dir).into_rules()
}

/// 崩溃诊断结果(同时作为 Tauri command 返回类型)
#[derive(Debug, Clone, Serialize)]
pub struct CrashDiagnosis {
    /// 是否找到了崩溃报告
    pub found: bool,
    /// 崩溃报告文件路径(未找到时为空)
    pub path: String,
    /// 主诊断说明
    pub summary: String,
    /// 具体可执行的下一步建议(去重)
    pub suggestions: Vec<String>,
    /// 命中的规则名
    pub matched: Vec<String>,
    /// 原始报告全文(前端用于“复制完整日志”;超长会被截断)
    pub raw_content: String,
    pub truncated: bool,
}

impl CrashDiagnosis {
    pub fn not_found() -> Self {
        Self {
            found: false,
            path: String::new(),
            summary: String::new(),
            suggestions: Vec::new(),
            matched: Vec::new(),
            raw_content: String::new(),
            truncated: false,
        }
    }
}

/// 扫描 <game_dir>/crash-reports/,返回修改时间最新的 crash-*.txt
pub fn find_latest_crash_report(game_dir: &Path) -> Option<PathBuf> {
    let dir = game_dir.join("crash-reports");
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("crash-") && name.ends_with(".txt") {
            let mtime = entry.metadata().ok().and_then(|m| m.modified().ok());
            if let Some(mtime) = mtime {
                if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
                    best = Some((mtime, entry.path()));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// 分析崩溃报告文本;返回(主诊断, 建议列表, 命中规则名)
pub fn analyze(content: &str, rules: &[CrashRule]) -> CrashDiagnosis {
    let mut matched: Vec<String> = Vec::new();
    let mut suggestions: Vec<String> = Vec::new();
    let mut summaries: Vec<String> = Vec::new();

    for rule in rules {
        let hit = rule.patterns.iter().any(|p| {
            Regex::new(p)
                .map(|re| re.is_match(content))
                .unwrap_or_else(|_| content.contains(p))
        });
        if hit {
            matched.push(rule.name.clone());
            if !summaries.contains(&rule.summary) {
                summaries.push(rule.summary.clone());
            }
            for s in &rule.suggestions {
                if !suggestions.contains(s) {
                    suggestions.push(s.clone());
                }
            }
        }
    }

    // 尽力定位具体 mod(建议性,不阻断)
    if let Some(mod_id) = find_mod_id(content) {
        let msg = format!("崩溃可能与 mod「{mod_id}」相关,尝试更新、禁用或删除该 mod。");
        if !suggestions.contains(&msg) {
            suggestions.push(msg);
        }
    }

    if matched.is_empty() && suggestions.is_empty() {
        suggestions.push("未匹配到已知崩溃特征,请查看完整日志或向启动器反馈。".into());
    }

    let summary = if summaries.is_empty() {
        "未识别的崩溃类型".to_string()
    } else {
        summaries.join(" ")
    };

    let (raw_content, truncated) = cap(content, 200_000);

    CrashDiagnosis {
        found: true,
        path: String::new(),
        summary,
        suggestions,
        matched,
        raw_content,
        truncated,
    }
}

/// 从崩溃文本中尽力提取可能导致崩溃的 mod id
fn find_mod_id(content: &str) -> Option<String> {
    // 常见的 mod 标识写法
    let patterns = [
        r#"(?m)Mod id\s+'([A-Za-z0-9_.-]+)'"#,
        r#"(?m)Mod\s+id\s*[:=]\s*'([A-Za-z0-9_.-]+)'"#,
        r#"(?m)mod\s+'([A-Za-z0-9_.-]+)'"#,
        r#"(?m)Missing mods?[:\s]+([A-Za-z0-9_.-]+)"#,
    ];
    for pat in patterns {
        if let Ok(re) = Regex::new(pat) {
            if let Some(c) = re.captures(content) {
                if let Some(m) = c.get(1) {
                    let id = m.as_str().trim();
                    if !id.is_empty() && id.len() < 64 {
                        return Some(id.to_string());
                    }
                }
            }
        }
    }
    None
}

/// 截断超长内容,避免 IPC 负载过大
fn cap(content: &str, limit: usize) -> (String, bool) {
    if content.len() <= limit {
        (content.to_string(), false)
    } else {
        (content.chars().take(limit).collect(), true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_oom() -> &'static str {
        r#"---- Minecraft Crash Report ----
Description: Unexpected error

java.lang.OutOfMemoryError: Java heap space
	at net.minecraft.client.Minecraft.run(Minecraft.java:1234)

-- System Details --
	Memory: 2 GB
"#
    }

    fn sample_unknown() -> &'static str {
        r#"---- Minecraft Crash Report ----
Description: Unexpected error

java.lang.IllegalStateException: something weird
	at net.minecraft.client.Minecraft.run(Minecraft.java:1)
"#
    }

    #[test]
    fn detects_oom_and_ignores_unmatched_words() {
        let rules = RuleSet::load_default().into_rules();
        let d = analyze(sample_oom(), &rules);
        assert!(d.found);
        assert!(d.matched.contains(&"内存不足".to_string()));
        assert!(d.suggestions.iter().any(|s| s.contains("最大内存")));
    }

    #[test]
    fn unmatched_returns_unknown_type() {
        let rules = RuleSet::load_default().into_rules();
        let d = analyze(sample_unknown(), &rules);
        assert!(d.matched.is_empty());
        assert_eq!(d.summary, "未识别的崩溃类型");
        assert!(!d.suggestions.is_empty());
    }

    #[test]
    fn finds_mod_id_in_resolution_error() {
        let content = r#"
net.minecraftforge.fml.ModLoadingException: ...
	Mod id 'sodium'
	Missing mods: lithium
"#;
        assert_eq!(find_mod_id(content).as_deref(), Some("sodium"));
    }

    #[test]
    fn missing_mod_line_matched() {
        let content = "Description: Loading mods\nMissing mods: lithium requires ...";
        assert_eq!(find_mod_id(content).as_deref(), Some("lithium"));
    }

    #[test]
    fn find_latest_picks_newest() {
        let dir = std::env::temp_dir().join(format!("rmcl_crash_{}", uuid::Uuid::new_v4().simple()));
        let crash = dir.join("crash-reports");
        std::fs::create_dir_all(&crash).unwrap();
        std::fs::write(crash.join("crash-2024-03-03_03.03.03-server.txt"), "server").unwrap();
        std::fs::write(crash.join("crash-2024-01-01_01.01.01-client.txt"), "old").unwrap();
        std::fs::write(crash.join("crash-2024-02-02_02.02.02-client.txt"), "new").unwrap();
        let found = find_latest_crash_report(&dir);
        assert_eq!(
            found.as_ref().map(|p| p.file_name().unwrap().to_string_lossy().to_string()),
            Some("crash-2024-02-02_02.02.02-client.txt".into())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
