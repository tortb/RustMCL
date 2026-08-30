use thiserror::Error;

/// 统一错误类型:所有可失败路径都收敛到该枚举,避免散落 unwrap()
#[derive(Debug, Error)]
pub enum RmclError {
    #[error("数据库错误: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("配置错误: {0}")]
    Config(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("网络错误: {0}")]
    Network(#[from] reqwest::Error),
    #[error("TOML 反序列化错误: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("TOML 序列化错误: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("压缩包错误: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// 用户主动中止的下载(非网络/校验失败),用于区分"取消"与"失败"
    #[error("下载已取消")]
    Cancelled,
    #[error("{0}")]
    Other(String),
}

impl RmclError {
    pub fn other(msg: impl Into<String>) -> Self {
        RmclError::Other(msg.into())
    }
}

/// Tauri command 层统一把 RmclError 转成 String 返回给前端
impl From<RmclError> for String {
    fn from(e: RmclError) -> Self {
        e.to_string()
    }
}
