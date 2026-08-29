//! 崩溃日志分析命令:定位最新 crash report 并给出诊断

use tauri::State;

use crate::core::diagnostics::{analyze, find_latest_crash_report, CrashDiagnosis};
use crate::db::repository::Repository;
use crate::AppState;

/// 分析指定实例(或默认共享游戏目录)最新的崩溃报告。
/// - 传入 instance_id:定位到实例专属 game_dir 下的 crash-reports/
/// - 不传:定位到共享 game_dir(~/.rustmcl/game)
#[tauri::command]
pub fn analyze_crash_report(
    state: State<'_, AppState>,
    instance_id: Option<String>,
) -> Result<CrashDiagnosis, String> {
    let game_dir = match instance_id.as_deref().unwrap_or("") {
        "" => state.data_dir.join("game"),
        id => {
            let conn = state
                .db
                .lock()
                .map_err(|e| format!("数据库锁获取失败: {e}"))?;
            Repository::get_instance(&conn, id)
                .map_err(|e| e.to_string())?
                .map(|inst| std::path::PathBuf::from(inst.game_dir))
                .ok_or_else(|| format!("实例不存在: {id}"))?
        }
    };

    let Some(path) = find_latest_crash_report(&game_dir) else {
        return Ok(CrashDiagnosis::not_found());
    };
    let content = std::fs::read_to_string(&path).map_err(|e| format!("读取崩溃报告失败: {e}"))?;

    let rules = crate::core::diagnostics::load_rules(&state.data_dir);
    let mut diag = analyze(&content, &rules);
    diag.path = path.to_string_lossy().to_string();
    Ok(diag)
}

/// 列出实例下全部崩溃报告(供前端展示历史)
#[tauri::command]
pub fn list_crash_reports(
    state: State<'_, AppState>,
    instance_id: Option<String>,
) -> Result<Vec<String>, String> {
    let game_dir = match instance_id.as_deref().unwrap_or("") {
        "" => state.data_dir.join("game"),
        id => {
            let conn = state
                .db
                .lock()
                .map_err(|e| format!("数据库锁获取失败: {e}"))?;
            Repository::get_instance(&conn, id)
                .map_err(|e| e.to_string())?
                .map(|inst| std::path::PathBuf::from(inst.game_dir))
                .ok_or_else(|| format!("实例不存在: {id}"))?
        }
    };
    let dir = game_dir.join("crash-reports");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut files: Vec<String> = entries
        .flatten()
        .map(|e| e.path().to_string_lossy().to_string())
        .filter(|p| {
            let name = std::path::Path::new(p)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            name.starts_with("crash-") && name.ends_with(".txt")
        })
        .collect();
    files.sort();
    Ok(files)
}
