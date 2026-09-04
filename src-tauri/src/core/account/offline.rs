//! 离线(本地)账号:基于用户名生成固定 UUID
//!
//! 算法与 Java `UUID.nameUUIDFromBytes` 等价(MD5 based, UUID v3):
//!   md5("OfflinePlayer:" + username) → 16 字节,然后:
//!     byte[6] = (byte[6] & 0x0f) | 0x30  # 置为 version 3
//!     byte[8] = (byte[8] & 0x3f) | 0x80  # 置为 IETF variant
//!   → 标准 8-4-4-4-12 格式
//!
//! 相同用户名每次生成的 UUID 完全一致,与其他启动器/服务端识别保持一致。

use crate::error::RmclError;

/// MC 用户名规则:3-16 位,仅字母数字下划线
pub const USERNAME_MIN: usize = 3;
pub const USERNAME_MAX: usize = 16;

/// 校验离线用户名是否合法
pub fn validate_username(username: &str) -> Result<(), RmclError> {
    let len = username.chars().count();
    if len < USERNAME_MIN || len > USERNAME_MAX {
        return Err(RmclError::other(format!(
            "用户名长度需为 {USERNAME_MIN}-{USERNAME_MAX} 个字符"
        )));
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(RmclError::other("用户名只能包含字母、数字和下划线"));
    }
    Ok(())
}

/// 生成离线用户名对应的固定 UUID(与 Java nameUUIDFromBytes 一致)
pub fn offline_uuid(username: &str) -> String {
    use md5::Digest;
    use md5::Md5;

    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{username}").as_bytes());
    let digest = hasher.finalize(); // 16 字节

    let mut b = [0u8; 16];
    b.copy_from_slice(&digest);
    b[6] = (b[6] & 0x0f) | 0x30;
    b[8] = (b[8] & 0x3f) | 0x80;

    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steve_offline_uuid_matches_java() {
        // 通过 Java UUID.nameUUIDFromBytes 计算的参考值
        assert_eq!(
            offline_uuid("Steve"),
            "5627dd98-e6be-3c21-b8a8-e92344183641"
        );
        // 幂等:重复调用结果一致
        assert_eq!(offline_uuid("Steve"), offline_uuid("Steve"));
    }

    #[test]
    fn validates_username() {
        assert!(validate_username("Steve").is_ok());
        assert!(validate_username("a_1").is_ok());
        assert!(validate_username("ab").is_err());
        assert!(validate_username("verylongusername123").is_err());
        assert!(validate_username("bad name").is_err());
        assert!(validate_username("名字").is_err());
    }
}
