//! mod 依赖冲突检测(模块 5):安装前分析目标版本的依赖声明,
//! 找出缺失的 required 依赖(建议自动补装)与 incompatible 冲突。
//! 语义为"建议性":前后端都不强制阻断,用户可忽略提示继续安装。

use serde::Serialize;

use crate::core::mods::modrinth::{ModrinthDependency, ModrinthVersion};

/// 一条缺失的 required 依赖(可据此自动补装)
#[derive(Debug, Clone, Serialize)]
pub struct DepHint {
    pub project_id: String,
    pub version_id: String,
    pub file_name: String,
}

/// 依赖检测结果
#[derive(Debug, Clone, Serialize)]
pub struct DepCheckResult {
    pub missing_required: Vec<DepHint>,
    pub conflicts: Vec<String>,
    pub ok: bool,
}

/// 依据目标版本的依赖声明,与已安装的 (project_id, version_id) 列表比对。
/// `installed` 为"当前实例已安装 mod 的 project_id → version_id"。
pub fn check(target: &ModrinthVersion, installed: &[(String, String)]) -> DepCheckResult {
    let mut missing: Vec<DepHint> = Vec::new();
    let mut conflicts: Vec<String> = Vec::new();

    for dep in &target.dependencies {
        let pid = dep.project_id.clone().unwrap_or_default();
        let installed_version = installed.iter().find(|(p, _)| p == &pid).map(|(_, v)| v.clone());

        match dep.dependency_type.as_str() {
            "required" => match installed_version {
                Some(v) => {
                    // 已装,但指向的特定版本不同 → 提示版本冲突(不阻断)
                    if dep.version_id.as_deref().map(|dv| dv != v.as_str()).unwrap_or(false) {
                        conflicts.push(format!("依赖「{}」要求版本不同(已装 {v})", dep_file_name(dep)));
                    }
                }
                None => missing.push(dep_hint(dep)),
            },
            "incompatible" => {
                if installed_version.is_some() {
                    conflicts.push(format!("已安装 mod 被目标版本声明为不兼容"));
                }
            }
            _ => {}
        }
    }

    // 循环依赖不会在这里发生(只做单层静态提示);自动补装由前端逐条发起
    let ok = missing.is_empty() && conflicts.is_empty();
    DepCheckResult {
        missing_required: missing,
        conflicts,
        ok,
    }
}

fn dep_hint(dep: &ModrinthDependency) -> DepHint {
    DepHint {
        project_id: dep.project_id.clone().unwrap_or_default(),
        version_id: dep.version_id.clone().unwrap_or_default(),
        file_name: dep.file_name.clone().unwrap_or_default(),
    }
}

fn dep_file_name(dep: &ModrinthDependency) -> String {
    dep.file_name
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| dep.project_id.clone().unwrap_or_else(|| "未知依赖".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mods::modrinth::ModrinthDependency;

    fn dep(dep_type: &str, project_id: &str, version_id: &str, file_name: &str) -> ModrinthDependency {
        ModrinthDependency {
            version_id: Some(version_id.to_string()),
            project_id: Some(project_id.to_string()),
            dependency_type: dep_type.to_string(),
            file_name: Some(file_name.to_string()),
        }
    }

    fn target(deps: Vec<ModrinthDependency>) -> ModrinthVersion {
        ModrinthVersion {
            id: "v".into(),
            project_id: "p".into(),
            name: "T".into(),
            version_number: "1.0".into(),
            game_versions: vec![],
            loaders: vec![],
            files: vec![],
            dependencies: deps,
        }
    }

    #[test]
    fn missing_required_reported() {
        let t = target(vec![dep("required", "sodium", "sod-v1", "sodium.jar")]);
        let r = check(&t, &[]);
        assert_eq!(r.missing_required.len(), 1);
        assert_eq!(r.missing_required[0].project_id, "sodium");
        assert!(!r.ok);
    }

    #[test]
    fn satisfied_required_ok() {
        let t = target(vec![dep("required", "sodium", "sod-v1", "sodium.jar")]);
        let r = check(&t, &[("sodium".to_string(), "sod-v1".to_string())]);
        assert!(r.missing_required.is_empty());
        assert!(r.ok);
    }

    #[test]
    fn incompatible_installed_conflict() {
        let t = target(vec![dep("incompatible", "badmod", "bv", "bad.jar")]);
        let r = check(&t, &[("badmod".to_string(), "bv".to_string())]);
        assert!(!r.conflicts.is_empty());
        assert!(!r.ok);
    }

    #[test]
    fn optional_dependency_ignored() {
        let t = target(vec![dep("optional", "extras", "ev", "extras.jar")]);
        let r = check(&t, &[]);
        assert!(r.missing_required.is_empty());
        assert!(r.ok);
    }
}
