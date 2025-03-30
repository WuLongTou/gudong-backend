/// 用户信息缓存键前缀
const USER_INFO_PREFIX: &str = "user:info:";

/// 用户状态缓存键前缀
const USER_STATUS_PREFIX: &str = "user:status:";

/// 生成用户信息缓存键
pub fn user_info_key(user_id: &str) -> String {
    format!("{}{}", USER_INFO_PREFIX, user_id)
}

/// 生成用户状态缓存键
pub fn user_status_key(user_id: &str) -> String {
    format!("{}{}", USER_STATUS_PREFIX, user_id)
}

/// 生成附近用户地理位置键
pub fn nearby_users_key() -> String {
    "geo:users".to_string()
}
