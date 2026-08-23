use thiserror::Error;

/// 统一错误类型:所有可失败路径都收敛到该枚举,避免散落 unwrap()
#[derive(Debug, Error)]
pub enum RunaError {
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
    #[error("{0}")]
    Other(String),
}

impl RunaError {
    pub fn other(msg: impl Into<String>) -> Self {
        RunaError::Other(msg.into())
    }
}

/// Tauri command 层统一把 RunaError 转成 String 返回给前端
impl From<RunaError> for String {
    fn from(e: RunaError) -> Self {
        e.to_string()
    }
}
