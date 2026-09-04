//! 下载镜像命令:列出可用镜像、测速、切换镜像源

use tauri::State;

use crate::config::app_config::AppConfig;
use crate::core::mirror::{self, Mirror, MirrorSpec, SpeedResult};
use crate::AppState;

/// 返回可用镜像源列表(预设 + 当前自定义基址)
#[tauri::command]
pub fn list_mirrors(state: State<'_, AppState>) -> Result<Vec<MirrorSpec>, String> {
    let cfg = AppConfig::load_or_create(&state.config_path).map_err(|e| e.to_string())?;
    let mut specs = mirror::presets();
    // 当前若是自定义源,把自定义基址补进列表供前端展示
    if let Some(base) = cfg.download.mirror_custom_base.as_deref() {
        if !base.trim().is_empty() {
            specs.retain(|s| s.id != "custom");
            specs.push(MirrorSpec {
                id: "custom".into(),
                name: "自定义".into(),
                base: base.to_string(),
            });
        }
    }
    Ok(specs)
}

/// 测速单个镜像源
#[tauri::command]
pub async fn test_mirror_speed(
    state: State<'_, AppState>,
    id: String,
    base: String,
) -> Result<SpeedResult, String> {
    let spec = MirrorSpec {
        id,
        name: String::new(),
        base,
    };
    Ok(mirror::speed_test(&state.client, &spec).await)
}

/// 对全部候选镜像并发测速,返回按延迟升序的结果
#[tauri::command]
pub async fn test_all_mirror_speed(state: State<'_, AppState>) -> Result<Vec<SpeedResult>, String> {
    let client = state.client.clone();
    let specs = list_mirrors(state)?;
    let mut handles = Vec::with_capacity(specs.len());
    for spec in specs {
        let client = client.clone();
        handles.push(tauri::async_runtime::spawn(async move {
            mirror::speed_test(&client, &spec).await
        }));
    }
    let mut out = Vec::with_capacity(handles.len());
    for h in handles {
        out.push(h.await.map_err(|e| format!("测速任务异常: {e}"))?);
    }
    out.sort_by_key(|r| r.latency_ms);
    Ok(out)
}

/// 切换镜像源:persist 到 config.toml,并同步更新 AppState 中的 active mirror
#[tauri::command]
pub fn set_mirror(
    state: State<'_, AppState>,
    mirror: String,
    custom_base: Option<String>,
) -> Result<Mirror, String> {
    let active = Mirror::from_config(&mirror, custom_base.as_deref());
    if !active.is_official() {
        // 校验自定义基址合法性
        let base = active.base.trim();
        if !base.is_empty() && !base.starts_with("https://") && !base.starts_with("http://") {
            return Err("镜像基址必须以 http(s):// 开头".into());
        }
    }

    let mut cfg = AppConfig::load_or_create(&state.config_path).map_err(|e| e.to_string())?;
    cfg.download.mirror = mirror;
    cfg.download.mirror_custom_base = custom_base.filter(|b| !b.trim().is_empty());
    cfg.save(&state.config_path).map_err(|e| e.to_string())?;

    {
        let mut cur = state
            .mirror
            .lock()
            .map_err(|e| format!("镜像状态锁获取失败: {e}"))?;
        *cur = active.clone();
    }
    Ok(active)
}
