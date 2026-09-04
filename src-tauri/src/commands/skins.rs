//! 皮肤管理命令(模块 9):本地皮肤库 + 微软账号上传 + 离线账号皮肤关联。
//! 上传复用账号系统的 keyring refresh token 静默续期,不另起一套 token 存储。

use std::collections::HashMap;
use std::path::Path;

use tauri::State;

use crate::core::account::microsoft_auth::resolve_active_account;
use crate::core::skin::{self, SkinEntry};
use crate::AppState;

/// Minecraft 皮肤上传端点(PUT,body 为 variant + data URL)
const SKIN_UPLOAD_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";

/// 列出本地皮肤库
#[tauri::command]
pub fn list_skins(state: State<AppState>) -> Result<Vec<SkinEntry>, String> {
    skin::list_skins(&state.data_dir).map_err(|e| e.to_string())
}

/// 导入本地皮肤(校验 64x64/64x32 PNG 并复制到皮肤库)
#[tauri::command]
pub fn import_skin(
    state: State<AppState>,
    src_path: String,
    name: String,
    model: String,
) -> Result<SkinEntry, String> {
    let model = if model == "slim" { "slim" } else { "classic" };
    skin::import_skin(
        &state.data_dir,
        std::path::Path::new(&src_path),
        &name,
        model,
    )
    .map_err(|e| e.to_string())
}

/// 删除本地皮肤
#[tauri::command]
pub fn remove_skin(state: State<AppState>, id: String) -> Result<(), String> {
    skin::remove_skin(&state.data_dir, &id).map_err(|e| e.to_string())
}

/// 读取皮肤图片的 data URL(供前端 3D 预览加载);不存在时返回 None
#[tauri::command]
pub fn get_skin_image(state: State<AppState>, id: String) -> Result<Option<String>, String> {
    Ok(skin::read_skin_png(&state.data_dir, &id)
        .ok()
        .map(|bytes| format!("data:image/png;base64,{}", skin::to_base64(&bytes))))
}

/// 将皮肤上传到当前活跃微软账号
#[tauri::command]
pub async fn upload_skin(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    let client = state.client.clone();

    // 1. 用 keyring 里的 refresh token 静默续期,拿到 access_token
    let Some((_, _, access_token)) = resolve_active_account(&client)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err("当前无有效的微软账号,请先登录微软账号后再上传皮肤".into());
    };

    // 2. 读取本地皮肤(校验尺寸 + 取 model)
    let entry = skin::list_skins(&data_dir)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "皮肤不存在".to_string())?;
    let bytes = skin::read_skin_png(&data_dir, &id).map_err(|e| e.to_string())?;

    // 3. 上传:body 为 variant + data URL(Content-Type 由 reqwest json 生成)
    let resp = client
        .put(SKIN_UPLOAD_URL)
        .bearer_auth(access_token)
        .json(&serde_json::json!({
            "variant": entry.model,
            "url": format!("data:image/png;base64,{}", skin::to_base64(&bytes)),
        }))
        .send()
        .await
        .map_err(|e| format!("上传请求失败: {e}"))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status().as_u16();
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        Err(format!(
            "上传皮肤失败(HTTP {status}): {}",
            body["errorMessage"].as_str().unwrap_or("未知错误")
        ))
    }
}

// ---------- 离线账号皮肤关联 ----------
// 离线账号无 Mojang 账号系统支撑,这里把所选本地皮肤持久化到 data_dir/offline_skins.json,
// 键为账号 uuid,值为皮肤库 id;游戏内实际渲染需基于版本的自定义方案(见 spec 9.4),此处前端会给出说明。

fn offline_skins_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("offline_skins.json")
}

fn read_offline_skins(data_dir: &Path) -> HashMap<String, String> {
    std::fs::read_to_string(offline_skins_path(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_offline_skins(data_dir: &Path, map: &HashMap<String, String>) -> Result<(), String> {
    let s = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(offline_skins_path(data_dir), s).map_err(|e| e.to_string())
}

/// 读取离线账号当前关联的皮肤 id
#[tauri::command]
pub fn get_offline_skin(
    state: State<AppState>,
    account_id: String,
) -> Result<Option<String>, String> {
    Ok(read_offline_skins(&state.data_dir)
        .get(&account_id)
        .cloned())
}

/// 设置/清除离线账号的皮肤关联(skin_id 传空则清除)
#[tauri::command]
pub fn set_offline_skin(
    state: State<AppState>,
    account_id: String,
    skin_id: Option<String>,
) -> Result<(), String> {
    let mut map = read_offline_skins(&state.data_dir);
    match skin_id.as_deref().filter(|s| !s.is_empty()) {
        Some(id) => {
            map.insert(account_id, id.to_string());
        }
        None => {
            map.remove(&account_id);
        }
    }
    write_offline_skins(&state.data_dir, &map)
}
