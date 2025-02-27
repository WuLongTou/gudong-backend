use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ApiResult<T: Serialize> {
    pub code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<T>,
}

impl<T: Serialize> ApiResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            error_message: None,
            content: Some(data),
        }
    }

    pub fn error(code: i32, message: &str) -> Self {
        Self {
            code,
            error_message: Some(message.to_string()),
            content: None,
        }
    }
}
