//! 微软账号 OAuth 全链路:Device Code Flow → Xbox Live → XSTS → Minecraft Token → Profile
//!
//! 链路(每步失败返回中文可读错误):
//!   1. MSA Device Code 授权(用户在浏览器输入 code)
//!   2. 轮询换取 MSA access_token + refresh_token
//!   3. XBL token(携带 MSA token)
//!   4. XSTS token(携带 XBL token,同时拿到 user hash)
//!   5. Minecraft 登录(携带 XSTS + uhs)→ access_token
//!   6. 拉取 Profile 得到正版 id / name
//!
//! refresh token 通过 keyring(系统钥匙串)保存,DB 只存账号元数据(T3.2)。

use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;

use crate::error::RmclError;

/// 独立注册的 Azure AD 应用 client id(非官方共享,已注册于 login.microsoftonline.com)
pub const CLIENT_ID: &str = "0f8c2c12-fcd8-4d56-8c5e-59bd13e78ee8";

const SCOPE: &str = "XboxLive.signin offline_access";

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/// keyring 条目:服务名 "rustmcl",条目名固定,保存微软 refresh token
const KEYRING_SERVICE: &str = "rustmcl";
const KEYRING_ENTRY: &str = "microsoft-refresh-token";

/// Device Code 响应(展示给用户 + 轮询用)
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeInfo {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: Option<String>,
}

/// OAuth token 端点成功响应
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    #[allow(dead_code)]
    pub expires_in: u64,
}

/// 轮询一次设备码授权的结果
pub enum PollResult {
    /// 授权完成,拿到 MSA token
    Success(OAuthToken),
    /// 用户尚未完成授权(authorization_pending / slow_down),继续轮询
    Pending,
    /// 明确失败(declined / expired / 校验码错误),停止轮询
    Failed(String),
}

/// Minecraft 正版 Profile
#[derive(Debug, Clone)]
pub struct McProfile {
    pub id: String,
    pub name: String,
}

/// 登录完成后的完整账号数据
#[derive(Debug, Clone)]
pub struct MicrosoftAccount {
    pub refresh_token: String,
    pub profile: McProfile,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
    #[allow(dead_code)]
    error_description: Option<String>,
}

/// 请求 Device Code,展示给用户在浏览器完成授权
pub async fn request_device_code(client: &Client) -> Result<DeviceCodeInfo, RmclError> {
    eprintln!("[rmcl-ms] POST {DEVICE_CODE_URL} client_id={CLIENT_ID} scope={SCOPE}");
    let resp = client
        .post(DEVICE_CODE_URL)
        .form(&[("client_id", CLIENT_ID), ("scope", SCOPE)])
        .send()
        .await?;
    let status = resp.status();
    eprintln!("[rmcl-ms] device_code HTTP {status}");
    if resp.status().is_success() {
        Ok(resp.json::<DeviceCodeInfo>().await?)
    } else {
        let e: TokenError = resp.json().await.unwrap_or_else(|_| TokenError {
            error: "unknown".into(),
            error_description: None,
        });
        Err(RmclError::other(format!("获取设备码失败: {}", e.error)))
    }
}

/// 轮询一次 Device Code 授权结果;调用方需按 interval 间隔重复调用
pub async fn poll_device_token(client: &Client, device_code: &str) -> PollResult {
    let resp = match client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return PollResult::Failed(format!("网络错误: {e}")),
    };
    if resp.status().is_success() {
        return match resp.json::<OAuthToken>().await {
            Ok(tok) => PollResult::Success(tok),
            Err(e) => PollResult::Failed(format!("解析令牌失败: {e}")),
        };
    }
    let status = resp.status();
    let err: TokenError = resp
        .json()
        .await
        .unwrap_or_else(|_| TokenError {
            error: "unknown".into(),
            error_description: None,
        });
    eprintln!("[rmcl-ms] poll_token HTTP {status}: {}", err.error);
    match err.error.as_str() {
        "authorization_pending" | "slow_down" => PollResult::Pending,
        "authorization_declined" => PollResult::Failed("用户拒绝了授权".into()),
        "expired_token" => PollResult::Failed("设备码已过期,请重新开始".into()),
        "bad_verification_code" => PollResult::Failed("校验码错误,请重新开始".into()),
        other => PollResult::Failed(format!("授权失败: {other}")),
    }
}

/// XBL / XSTS 共用的请求体结构
#[derive(Serialize)]
struct XboxAuthRequest<'a> {
    #[serde(rename = "Properties")]
    properties: XboxAuthProperties<'a>,
    #[serde(rename = "RelyingParty")]
    relying_party: &'a str,
    #[serde(rename = "TokenType")]
    token_type: &'a str,
}

#[derive(Serialize)]
struct XboxAuthProperties<'a> {
    /// 仅 XBL 使用(官方值为 "RPS");XSTS 不携带
    #[serde(rename = "AuthMethod", skip_serializing_if = "Option::is_none")]
    auth_method: Option<&'a str>,
    /// 仅 XBL 使用
    #[serde(rename = "SiteName", skip_serializing_if = "Option::is_none")]
    site_name: Option<&'a str>,
    /// "d=<msa_token>";XSTS 不携带
    #[serde(rename = "RpsTicket", skip_serializing_if = "Option::is_none")]
    rps_ticket: Option<String>,
    #[serde(rename = "SandboxId", skip_serializing_if = "Option::is_none")]
    sandbox_id: Option<&'a str>,
    #[serde(rename = "UserTokens", skip_serializing_if = "Option::is_none")]
    user_tokens: Option<Vec<String>>,
}

/// 调用 XBL / XSTS 认证端点,返回 (token, user_hash)
/// 二者对 Properties 的字段要求不同,由调用方分别构造。
async fn xbox_auth(
    client: &Client,
    url: &str,
    relying_party: &str,
    properties: XboxAuthProperties<'_>,
) -> Result<(String, String), RmclError> {
    let req = XboxAuthRequest {
        properties,
        relying_party,
        token_type: "JWT",
    };
    eprintln!("[rmcl-ms] POST {url} relying_party={relying_party}");
    let resp = client
        .post(url)
        .header("x-xbl-contract-version", "1")
        .header("Accept", "application/json")
        .json(&req)
        .send()
        .await?;
    let status = resp.status();
    let status_code = status.as_u16();
    // 先取原始响应体文本并打印,避免解码失败时丢失 status/body;再尝试解析为 JSON
    let raw = resp.text().await.unwrap_or_default();
    eprintln!("[rmcl-ms] {url} HTTP {status_code} body: {}", raw.chars().take(400).collect::<String>());
    let body: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => serde_json::Value::String(raw),
    };
    if status.is_success() {
        let token = body["Token"]
            .as_str()
            .ok_or_else(|| RmclError::other("Xbox 响应缺少 Token"))?
            .to_string();
        let uhs = body["DisplayClaims"]["xui"][0]["uhs"]
            .as_str()
            .ok_or_else(|| RmclError::other("Xbox 响应缺少 uhs"))?
            .to_string();
        Ok((token, uhs))
    } else {
        eprintln!("[rmcl-ms] {url} HTTP {status}: {body}");
        Err(xbox_error(&body, url))
    }
}

/// 把 XSTS 的错误码(XErr)映射为中文可读信息
fn xbox_error(body: &serde_json::Value, url: &str) -> RmclError {
    let msg = match body["XErr"].as_i64() {
        Some(2148916233) => "该微软账号未关联 Xbox 账号,请先到 xbox.com 创建".into(),
        Some(2148916235) => "该地区暂不支持 Xbox Live,请检查账号所在地".into(),
        Some(2148916236) => "Xbox Live 服务暂时不可用,请稍后重试".into(),
        Some(2148916237) => "需要先在 Xbox 页面同意服务条款".into(),
        Some(2148916238) => "该账号为未成年人,需由监护人完成认证后使用".into(),
        Some(2148916258) => "该账号不满足使用条件(可能未拥有 Minecraft 正版)".into(),
        _ => format!(
            "Xbox 认证失败({url}): {}",
            body["Message"].as_str().unwrap_or("未知错误")
        ),
    };
    RmclError::other(msg)
}

/// 使用 MSA access/refresh token 完成 XBL → XSTS → Minecraft 全链路
pub async fn exchange_tokens(
    client: &Client,
    msa_access_token: &str,
    msa_refresh_token: Option<&str>,
) -> Result<MicrosoftAccount, RmclError> {
    // 日志 token 类型前缀,便于排查(MSA access token 可能是 opaque 或 JWT,均属正常)
    eprintln!(
        "[rmcl-ms] MSA access_token 前缀: {}(token 长度 {})",
        msa_access_token.chars().take(8).collect::<String>(),
        msa_access_token.len()
    );
    // 1. XBL token(携带 MSA access token)
    let (xbl_token, _uhs) = xbox_auth(
        client,
        XBL_AUTH_URL,
        "http://auth.xboxlive.com",
        XboxAuthProperties {
            auth_method: Some("RPS"),
            site_name: Some("user.auth.xboxlive.com"),
            rps_ticket: Some(format!("d={msa_access_token}")),
            sandbox_id: None,
            user_tokens: None,
        },
    )
    .await?;
    // 2. XSTS token(仅携带 SandboxId + UserTokens,拿到 user hash)
    let (xsts_token, uhs) = xbox_auth(
        client,
        XSTS_AUTH_URL,
        "rp://api.minecraftservices.com/",
        XboxAuthProperties {
            auth_method: None,
            site_name: None,
            rps_ticket: None,
            sandbox_id: Some("RETAIL"),
            user_tokens: Some(vec![xbl_token]),
        },
    )
    .await?;
    // 3. Minecraft 登录
    eprintln!("[rmcl-ms] POST {MC_LOGIN_URL}");
    let resp = client
        .post(MC_LOGIN_URL)
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={uhs};{xsts_token}")
        }))
        .send()
        .await?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;
    if !status.is_success() {
        eprintln!("[rmcl-ms] {MC_LOGIN_URL} HTTP {status}: {body}");
        return Err(RmclError::other(format!(
            "Minecraft 登录失败: {}",
            body["errorMessage"].as_str().unwrap_or("未知错误")
        )));
    }
    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| RmclError::other("Minecraft 响应缺少 access_token"))?
        .to_string();
    // 4. 拉取 Profile
    let profile = fetch_profile(client, &access_token).await?;
    Ok(MicrosoftAccount {
        refresh_token: msa_refresh_token.unwrap_or_default().to_string(),
        profile,
    })
}

/// 用 access_token 拉取 Minecraft Profile(id + name)
pub async fn fetch_profile(client: &Client, access_token: &str) -> Result<McProfile, RmclError> {
    eprintln!("[rmcl-ms] GET {MC_PROFILE_URL}");
    let resp = client
        .get(MC_PROFILE_URL)
        .bearer_auth(access_token)
        .send()
        .await?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;
    if status.is_success() {
        let id = body["id"]
            .as_str()
            .ok_or_else(|| RmclError::other("Profile 缺少 id"))?
            .to_string();
        let name = body["name"]
            .as_str()
            .ok_or_else(|| RmclError::other("Profile 缺少 name"))?
            .to_string();
        Ok(McProfile { id, name })
    } else if status.as_u16() == 404 {
        eprintln!("[rmcl-ms] profile HTTP 404: 没有 Minecraft Java 版");
        Err(RmclError::other("该微软账号没有 Minecraft Java 版,请先购买游戏"))
    } else {
        eprintln!("[rmcl-ms] {MC_PROFILE_URL} HTTP {status}: {body}");
        Err(RmclError::other(format!(
            "获取 Profile 失败: HTTP {}",
            status.as_u16()
        )))
    }
}

/// 用 refresh token 静默续期,返回 (access_token, 新 refresh_token)
pub async fn refresh_access_token(
    client: &Client,
    refresh_token: &str,
) -> Result<(String, String), RmclError> {
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ])
        .send()
        .await?;
    if resp.status().is_success() {
        let t: OAuthToken = resp.json().await?;
        let new_refresh = t
            .refresh_token
            .unwrap_or_else(|| refresh_token.to_string());
        Ok((t.access_token, new_refresh))
    } else {
        let e: TokenError = resp.json().await.unwrap_or_else(|_| TokenError {
            error: "unknown".into(),
            error_description: None,
        });
        Err(RmclError::other(format!("刷新令牌失败: {}", e.error)))
    }
}

// ---------- keyring 存储(T3.2) ----------

pub fn save_refresh_token(token: &str) -> Result<(), RmclError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ENTRY)
        .map_err(|e| RmclError::other(format!("打开系统钥匙串失败: {e}")))?;
    entry
        .set_password(token)
        .map_err(|e| RmclError::other(format!("保存令牌失败: {e}")))?;
    Ok(())
}

pub fn load_refresh_token() -> Result<String, RmclError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ENTRY)
        .map_err(|e| RmclError::other(format!("打开系统钥匙串失败: {e}")))?;
    entry
        .get_password()
        .map_err(|e| RmclError::other(format!("读取令牌失败: {e}")))
}

pub fn delete_refresh_token() -> Result<(), RmclError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ENTRY)
        .map_err(|e| RmclError::other(format!("打开系统钥匙串失败: {e}")))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(RmclError::other(format!("删除令牌失败: {e}"))),
    }
}

/// 静默续期并返回当前账号的 (name, uuid, access_token);未登录或无有效令牌时返回 None
pub async fn resolve_active_account(
    client: &Client,
) -> Result<Option<(String, String, String)>, RmclError> {
    let refresh = match load_refresh_token() {
        Ok(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };
    let (access, new_refresh) = refresh_access_token(client, &refresh).await?;
    if new_refresh != refresh {
        let _ = save_refresh_token(&new_refresh);
    }
    let profile = fetch_profile(client, &access).await?;
    Ok(Some((
        profile.name,
        format_uuid(&profile.id),
        access,
    )))
}

/// 将不带横杠的 32 位 uuid 格式化为标准 8-4-4-4-12(游戏参数要求)
pub fn format_uuid(raw: &str) -> String {
    let clean: String = raw.chars().filter(|c| *c != '-').collect();
    if clean.len() == 32 {
        format!(
            "{}-{}-{}-{}-{}",
            &clean[0..8],
            &clean[8..12],
            &clean[12..16],
            &clean[16..20],
            &clean[20..32]
        )
    } else {
        clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_uuid() {
        assert_eq!(
            format_uuid("0123456789abcdef0123456789abcdef"),
            "01234567-89ab-cdef-0123-456789abcdef"
        );
        // 带横杠输入幂等
        assert_eq!(
            format_uuid("01234567-89ab-cdef-0123-456789abcdef"),
            "01234567-89ab-cdef-0123-456789abcdef"
        );
        // 非法长度原样返回
        assert_eq!(format_uuid("abc"), "abc");
    }

    #[test]
    fn maps_xbox_err_codes() {
        let json = serde_json::json!({"XErr": 2148916233i64, "Message": "no"});
        let err = xbox_error(&json, "xsts");
        assert!(err.to_string().contains("未关联 Xbox"));
        let json2 = serde_json::json!({"XErr": 2148916238i64, "Message": "minor"});
        let err2 = xbox_error(&json2, "xsts");
        assert!(err2.to_string().contains("未成年人"));
        let json3 = serde_json::json!({"XErr": 12345, "Message": "boom"});
        let err3 = xbox_error(&json3, "xsts");
        assert!(err3.to_string().contains("boom"));
    }
}
