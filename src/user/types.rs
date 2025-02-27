use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct RegisterUserRequest {
    pub security_code: String,
    pub public_key: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RegisterUserResponse {
    pub session_token: String,
    pub server_public_key: serde_json::Value,
    pub expires_at: i64,
}
