//! 启动相关:参数拼装、natives 解压、子进程管理

pub mod args_builder;
pub mod process;

use std::path::{Path, PathBuf};

use crate::core::version::rules::{rules_allow, RuleContext};
use crate::core::version::version_json::VersionJson;
use crate::error::RmclError;

/// 单个 native 库的解压计划
pub struct NativePlan {
    pub jar_path: PathBuf,
    pub exclude: Vec<String>,
}

/// 收集本机需要解压的 native 库
pub fn native_plan(version: &VersionJson, ctx: &RuleContext, libraries_dir: &PathBuf) -> Vec<NativePlan> {
    let mut plans = Vec::new();
    for lib in &version.libraries {
        if !rules_allow(lib.rules.as_deref(), ctx) {
            continue;
        }
        let Some(natives) = &lib.natives else { continue };
        let Some(classifier) = natives.get(ctx.os_name) else { continue };
        let Some(downloads) = &lib.downloads else { continue };
        let Some(classifiers) = &downloads.classifiers else { continue };
        let Some(dl) = classifiers.get(classifier) else { continue };
        plans.push(NativePlan {
            jar_path: libraries_dir.join(dl.path.clone().unwrap_or_default()),
            exclude: lib
                .extract
                .as_ref()
                .map(|e| e.exclude.clone())
                .unwrap_or_default(),
        });
    }
    plans
}

/// 将 native jar 解压到 natives 目录(排除 extract.exclude 指定项)
pub fn extract_natives(plans: &[NativePlan], natives_dir: &Path) -> Result<(), RmclError> {
    std::fs::create_dir_all(natives_dir)?;
    for plan in plans {
        let file = std::fs::File::open(&plan.jar_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if entry.is_dir() {
                continue;
            }
            if plan.exclude.iter().any(|e| name.starts_with(e)) {
                continue;
            }
            // 防路径穿越
            if name.split('/').any(|seg| seg == "..") {
                continue;
            }
            let out = natives_dir.join(&name);
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut f = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut f)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::core::version::rules::FeaturesCtx;

    #[test]
    fn extract_native_jar_skips_excludes() {
        let dir = std::env::temp_dir().join(format!("rmcl_natives_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // 手工构造一个含两个文件的 zip
        let jar_path = dir.join("fake-natives.jar");
        let file = std::fs::File::create(&jar_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        writer.start_file("libfake.so", opts).unwrap();
        writer.write_all(b"binary").unwrap();
        writer.start_file("META-INF/MANIFEST.MF", opts).unwrap();
        writer.write_all(b"manifest").unwrap();
        writer.finish().unwrap();

        let plans = vec![NativePlan {
            jar_path,
            exclude: vec!["META-INF/".into()],
        }];
        let out = dir.join("out");
        extract_natives(&plans, &out).unwrap();

        assert!(out.join("libfake.so").exists());
        assert!(!out.join("META-INF/MANIFEST.MF").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn native_plan_picks_current_os() {
        let ctx = RuleContext::current(FeaturesCtx::default());
        let version: VersionJson = serde_json::from_str(
            r#"{
                "id": "t",
                "assetIndex": {"id": "18", "sha1": "a", "size": 1, "url": "u"},
                "downloads": {"client": {"sha1": "b", "size": 1, "url": "u"}},
                "libraries": [{
                    "name": "x",
                    "natives": {"linux": "natives-linux", "osx": "natives-macos", "windows": "natives-windows"},
                    "extract": {"exclude": ["META-INF/"]},
                    "downloads": {"classifiers": {
                        "natives-linux": {"path": "x/natives-linux.jar", "sha1": "c", "size": 1, "url": "u"},
                        "natives-macos": {"path": "x/natives-macos.jar", "sha1": "d", "size": 1, "url": "u"},
                        "natives-windows": {"path": "x/natives-windows.jar", "sha1": "e", "size": 1, "url": "u"}
                    }}
                }],
                "mainClass": "M"
            }"#,
        )
        .unwrap();
        let plans = native_plan(&version, &ctx, &PathBuf::from("/libraries"));
        assert_eq!(plans.len(), 1);
        let expected = format!("natives-{}.jar", ctx.os_name);
        assert!(plans[0].jar_path.to_string_lossy().ends_with(&expected));
    }
}
