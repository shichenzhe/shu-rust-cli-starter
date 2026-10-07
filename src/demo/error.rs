use thiserror::Error;

/// demo 域 API 错误类型（thiserror 分层示范）
#[derive(Debug, Error)]
pub enum DemoApiError {
    #[error("网络请求失败: {0}")]
    Request(#[from] reqwest::Error),

    #[error("API 错误 ({status}): {message}")]
    Status { status: u16, message: String },

    #[error("JSON 解析失败: {0}")]
    Decode(String),
}
