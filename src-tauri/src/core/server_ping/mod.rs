//! Minecraft Server List Ping 协议:Handshake → Status Request → Status Response。
//! 不依赖游戏本体,直接用 tokio TcpStream 手写协议帧(VarInt 编解码可单测)。
//! 带超时(默认 4s),超时/连接失败返回错误而非无限阻塞。

use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::db::schema::ServerStatus;
use crate::error::RmclError;

const TIMEOUT: Duration = Duration::from_secs(4);

/// 对指定地址端口做 ping,成功返回 ServerStatus(ok=true),失败返回 Err
pub async fn ping(address: &str, port: u16) -> Result<ServerStatus, RmclError> {
    let start = Instant::now();
    let status = tokio::time::timeout(TIMEOUT, ping_inner(address, port)).await;
    match status {
        Ok(Ok(mut s)) => {
            s.latency_ms = start.elapsed().as_millis() as u64;
            Ok(s)
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err(RmclError::other("ping 超时(>4s)")),
    }
}

async fn ping_inner(address: &str, port: u16) -> Result<ServerStatus, RmclError> {
    let mut stream = TcpStream::connect((address, port)).await?;
    let _ = stream.set_nodelay(true);

    // 1. Handshake(packet id 0x00, protocol -1, server addr, port, next state 0x01)
    let mut hs: Vec<u8> = vec![0x00];
    hs.extend(encode_varint(-1));
    hs.extend(encode_varint(address.len() as i32));
    hs.extend(address.as_bytes());
    hs.extend(&port.to_be_bytes());
    hs.push(0x01);
    let mut frame = encode_varint(hs.len() as i32);
    frame.extend(hs);
    stream.write_all(&frame).await?;

    // 2. Status Request(packet id 0x00, 无 payload)
    let mut req = encode_varint(1);
    req.push(0x00);
    stream.write_all(&req).await?;

    // 3. 读响应:VarInt 总长度 + 包内数据(packet id 0x00 + VarInt 字符串长度 + JSON)
    let total_len = read_varint(&mut stream).await? as usize;
    if total_len > 1_048_576 {
        return Err(RmclError::other("ping 响应过大"));
    }
    let mut body = vec![0u8; total_len];
    stream.read_exact(&mut body).await?;
    // 跳过 packet id
    let (_, after_id) = read_varint_from_slice(&body);
    let json = read_string_from_slice(&after_id)?;

    parse_status(&json)
}

/// 解析 status JSON;任何字段缺失都优雅降级(不 panic)
fn parse_status(json: &str) -> Result<ServerStatus, RmclError> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| RmclError::other(format!("status 解析失败: {e}")))?;

    let motd = v
        .get("description")
        .map(extract_motd)
        .unwrap_or_else(|| String::new());

    let (online, max) = v
        .get("players")
        .map(|p| {
            (
                p.get("online").and_then(|n| n.as_i64()).unwrap_or(0),
                p.get("max").and_then(|n| n.as_i64()).unwrap_or(0),
            )
        })
        .unwrap_or((0, 0));

    let favicon = v.get("favicon").and_then(|f| f.as_str()).map(|s| s.to_string());

    Ok(ServerStatus {
        id: String::new(),
        motd,
        players_online: online,
        players_max: max,
        latency_ms: 0,
        favicon,
        ok: true,
    })
}

/// 提取 MOTD 文本:字符串直接返回;对象(聊天组件)取 text + extra(部分用 with)递归。
fn extract_motd(desc: &serde_json::Value) -> String {
    match desc {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(obj) => {
            let mut parts = Vec::new();
            if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
                if !text.is_empty() {
                    parts.push(text.to_string());
                }
            }
            // 子组件:标准字段为 extra,部分服务器用 with;均按数组递归
            let arr = obj
                .get("extra")
                .and_then(|e| e.as_array())
                .or_else(|| obj.get("with").and_then(|w| w.as_array()));
            if let Some(arr) = arr {
                for item in arr {
                    if let Some(c) = extract_motd_opt(item) {
                        parts.push(c);
                    }
                }
            }
            parts.join("")
        }
        serde_json::Value::Array(arr) => arr.iter().filter_map(extract_motd_opt).collect(),
        _ => String::new(),
    }
}

fn extract_motd_opt(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(_) => {
            let s = extract_motd(v);
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        serde_json::Value::Array(arr) => {
            let joined: String = arr.iter().filter_map(extract_motd_opt).collect();
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        }
        _ => None,
    }
}

/// VarInt 编码(i32 -> 最多 5 字节)
pub fn encode_varint(mut value: i32) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
    out
}

/// 从字节切片解码首个 VarInt,返回(值, 剩余切片)
fn read_varint_from_slice(buf: &[u8]) -> (i32, &[u8]) {
    let mut value: i32 = 0;
    let mut shift = 0;
    let mut i = 0;
    loop {
        let byte = buf[i];
        value |= ((byte & 0x7F) as i32) << shift;
        i += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    (value, &buf[i..])
}

/// 从流中读一个 VarInt
async fn read_varint(stream: &mut TcpStream) -> Result<i32, RmclError> {
    let mut value: i32 = 0;
    let mut shift = 0;
    loop {
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte).await?;
        value |= ((byte[0] & 0x7F) as i32) << shift;
        if byte[0] & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift > 35 {
            return Err(RmclError::other("VarInt 过长"));
        }
    }
    Ok(value)
}

/// 从切片中读 VarInt 长度的字符串
fn read_string_from_slice(buf: &[u8]) -> Result<String, RmclError> {
    let (len, rest) = read_varint_from_slice(buf);
    let len = len as usize;
    if len > rest.len() {
        return Err(RmclError::other("status 字符串长度越界"));
    }
    Ok(String::from_utf8_lossy(&rest[..len]).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_roundtrip_small() {
        for v in [0i32, 1, 127, 128, 255, 300, 25565, -1] {
            let enc = encode_varint(v);
            let (dec, rest) = read_varint_from_slice(&enc);
            assert_eq!(dec, v, "v={v}");
            assert!(rest.is_empty());
        }
    }

    #[test]
    fn varint_zero() {
        assert_eq!(encode_varint(0), vec![0x00]);
    }

    #[test]
    fn motd_string() {
        let json = r#"{"description":{"text":"Hello World"},"players":{"online":3,"max":20}}"#;
        let s = parse_status(json).unwrap();
        assert_eq!(s.motd, "Hello World");
        assert_eq!(s.players_online, 3);
        assert_eq!(s.players_max, 20);
        assert!(s.ok);
    }

    #[test]
    fn motd_legacy_plain_string() {
        let json = r#"{"description":"A plain motd","players":{"online":0,"max":10}}"#;
        let s = parse_status(json).unwrap();
        assert_eq!(s.motd, "A plain motd");
    }

    #[test]
    fn motd_object_with_translate() {
        let json = r#"{"description":{"text":"Hi","extra":[{"text":" there"}]},"players":{"online":1,"max":2}}"#;
        let s = parse_status(json).unwrap();
        assert_eq!(s.motd, "Hi there");
    }

    #[test]
    fn missing_players_defaults_zero() {
        let json = r#"{"description":{"text":"x"}}"#;
        let s = parse_status(json).unwrap();
        assert_eq!(s.players_online, 0);
        assert_eq!(s.players_max, 0);
    }

    #[test]
    fn invalid_json_errors() {
        assert!(parse_status("not json").is_err());
    }
}
