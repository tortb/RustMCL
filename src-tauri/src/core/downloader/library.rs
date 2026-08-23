//! client.jar 与 libraries 下载计划构建

use std::path::PathBuf;

use super::{DownloadItem};
use crate::core::version::rules::{rules_allow, RuleContext};
use crate::core::version::version_json::VersionJson;

/// client.jar → versions/<id>/<id>.jar
pub fn client_download_item(version: &VersionJson, version_dir: &PathBuf) -> DownloadItem {
    let dl = &version.downloads.client;
    DownloadItem {
        url: dl.url.clone(),
        sha1: dl.sha1.clone(),
        size: dl.size,
        dest: version_dir.join(format!("{}.jar", version.id)),
    }
}

/// 本机需要运行的 libraries 的 artifact 下载列表(natives classifier 的 jar 也包含在 classifiers 中,
/// 但这里只取其 artifact;native jar 由 launch 阶段按需下载解压)
pub fn library_items(version: &VersionJson, ctx: &RuleContext, libraries_dir: &PathBuf) -> Vec<DownloadItem> {
    let mut items = Vec::new();
    for lib in &version.libraries {
        if !rules_allow(lib.rules.as_deref(), ctx) {
            continue;
        }
        let Some(downloads) = &lib.downloads else { continue };
        let Some(artifact) = &downloads.artifact else { continue };
        let path = artifact
            .path
            .clone()
            .unwrap_or_else(|| lib.name.replace(':', "/") + ".jar");
        items.push(DownloadItem {
            url: artifact.url.clone(),
            sha1: artifact.sha1.clone(),
            size: artifact.size,
            dest: libraries_dir.join(path),
        });
    }
    items
}

/// 本机需要的 native 库 jar(如 natives-linux),供启动时解压
pub fn native_items(version: &VersionJson, ctx: &RuleContext, libraries_dir: &PathBuf) -> Vec<DownloadItem> {
    let mut items = Vec::new();
    for lib in &version.libraries {
        if !rules_allow(lib.rules.as_deref(), ctx) {
            continue;
        }
        let Some(natives) = &lib.natives else { continue };
        let Some(classifier_name) = natives.get(ctx.os_name) else { continue };
        let Some(downloads) = &lib.downloads else { continue };
        let Some(classifiers) = &downloads.classifiers else { continue };
        let Some(dl) = classifiers.get(classifier_name) else { continue };
        let path = dl.path.clone().unwrap_or_default();
        items.push(DownloadItem {
            url: dl.url.clone(),
            sha1: dl.sha1.clone(),
            size: dl.size,
            dest: libraries_dir.join(path),
        });
    }
    items
}
