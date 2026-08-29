//! 微软登录命令:后台执行 Device Code 授权链路,事件驱动前端
//! 事件:
//!   - "ms-login-device"   DeviceCodeInfo(展示 user_code / verification_uri)
//!   - "ms-login-status"   stage + message(轮询/换 token/保存等阶段提示)
//!   - "ms-login-finished" { ok, error }

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, State};

use crate::core::account::microsoft_auth::{
    delete_refresh_token, exchange_tokens, poll_device_token, request_device_code,
    save_refresh_token, PollResult,
};
use crate::db::repository::Repository;
use crate::db::schema::Account;
use crate::AppState;

/// 全局取消标志:新登录开始时复位,取消时置位,轮询循环检查
static LOGIN_CANCEL: OnceLock<AtomicBool> = OnceLock::new();

fn cancel_flag() -> &'static AtomicBool {
    LOGIN_CANCEL.get_or_init(|| AtomicBool::new(false))
}

#[derive(Clone, serde::Serialize)]
pub struct MsDeviceEvent {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub message: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct MsStatusEvent {
    pub stage: String,
    pub message: String,
}

#[derive(Clone, serde::Serialize)]
pub struct MsFinishedEvent {
    pub ok: bool,
    pub error: String,
}

/// 启动微软登录:立即返回,授权过程在后台执行并通过事件上报
#[tauri::command]
pub fn start_microsoft_login(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let client = state.client.clone();
    let db_path = state.data_dir.join("rmcl.db");
    cancel_flag().store(false, Ordering::SeqCst);

    tauri::async_runtime::spawn(async move {
        let result = run_login(client, &db_path, app.clone()).await;
        let _ = app.emit(
            "ms-login-finished",
            MsFinishedEvent {
                ok: result.is_ok(),
                error: result.err().unwrap_or_default(),
            },
        );
    });
    Ok(())
}

/// 取消当前登录流程
#[tauri::command]
pub fn cancel_microsoft_login() -> Result<(), String> {
    cancel_flag().store(true, Ordering::SeqCst);
    Ok(())
}

/// 返回当前激活账号
#[tauri::command]
pub fn get_active_account(state: State<'_, AppState>) -> Result<Option<Account>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::get_active_account(&conn).map_err(|e| e.to_string())
}

/// 退出登录:清除 keyring 中的 refresh token,并将账号置为非激活
#[tauri::command]
pub fn logout_account(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let _ = delete_refresh_token();
    let conn = state
        .db
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    Repository::deactivate_account(&conn, &id).map_err(|e| e.to_string())
}

async fn run_login(
    client: reqwest::Client,
    db_path: &std::path::Path,
    app: AppHandle,
) -> Result<(), String> {
    let _ = app.emit(
        "ms-login-status",
        MsStatusEvent {
            stage: "device".into(),
            message: "正在获取设备码...".into(),
        },
    );

    // 1. Device Code
    let info = request_device_code(&client)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit(
        "ms-login-device",
        MsDeviceEvent {
            user_code: info.user_code.clone(),
            verification_uri: info.verification_uri.clone(),
            expires_in: info.expires_in,
            message: info.message.clone(),
        },
    );

    // 2. 轮询授权结果
    let oauth = loop {
        if cancel_flag().load(Ordering::SeqCst) {
            return Err("用户取消了登录".into());
        }
        match poll_device_token(&client, &info.device_code).await {
            PollResult::Success(tok) => break tok,
            PollResult::Pending => {
                tokio::time::sleep(Duration::from_secs(info.interval.max(5))).await;
            }
            PollResult::Failed(msg) => return Err(msg),
        }
    };

    let _ = app.emit(
        "ms-login-status",
        MsStatusEvent {
            stage: "exchange".into(),
            message: "授权成功,正在验证 Xbox 账号...".into(),
        },
    );

    // 3. XBL → XSTS → Minecraft token → Profile
    let account = exchange_tokens(&client, &oauth.access_token, oauth.refresh_token.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "ms-login-status",
        MsStatusEvent {
            stage: "save".into(),
            message: "正在保存账号信息...".into(),
        },
    );

    // 4. refresh token 存入系统钥匙串,DB 只存元数据
    if !account.refresh_token.is_empty() {
        save_refresh_token(&account.refresh_token).map_err(|e| e.to_string())?;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let db_account = Account {
        id: account.profile.id.clone(),
        username: account.profile.name,
        uuid: account.profile.id,
        account_type: "microsoft".into(),
        is_active: true,
        refreshed_at: Some(now),
    };
    // 后台任务无法跨线程持有 state 的锁,这里打开独立连接写入(WAL 模式支持多连接)
    let conn = rusqlite::Connection::open(db_path).map_err(|e| format!("打开数据库失败: {e}"))?;
    Repository::upsert_account(&conn, &db_account).map_err(|e| e.to_string())?;
    Ok(())
}
