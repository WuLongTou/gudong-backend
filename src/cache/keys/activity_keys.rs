/// 用户位置缓存键前缀
const USER_LOCATION_PREFIX: &str = "user:loc:";

/// 活动缓存键前缀
const ACTIVITY_PREFIX: &str = "activity:";

/// 活动历史缓存键前缀
const USER_ACTIVITIES_PREFIX: &str = "user:activities:";

/// 用户GEO索引键
pub const USER_GEO_KEY: &str = "users:geo";

/// 活动GEO索引键
pub const ACTIVITY_GEO_KEY: &str = "activities:geo";

/// 生成用户缓存键
pub fn user_cache_key(user_id: &str) -> String {
    format!("user:{}", user_id)
}

/// 生成活动缓存键
pub fn activity_cache_key(activity_id: &str) -> String {
    format!("activity:{}", activity_id)
}

/// 生成附近用户缓存键
pub fn nearby_users_key(lat: f64, lon: f64, radius: f64, limit: i64) -> String {
    // 精确到小数点后两位的坐标
    let lat_rounded = (lat * 100.0).round() / 100.0;
    let lon_rounded = (lon * 100.0).round() / 100.0;
    format!(
        "{}{}:{}:{}:{}",
        USER_LOCATION_PREFIX, lat_rounded, lon_rounded, radius, limit
    )
}

/// 生成附近活动缓存键
pub fn nearby_activities_key(lat: f64, lon: f64, radius: f64, limit: i64) -> String {
    // 精确到小数点后两位的坐标
    let lat_rounded = (lat * 100.0).round() / 100.0;
    let lon_rounded = (lon * 100.0).round() / 100.0;
    format!(
        "{}{}:{}:{}:{}",
        ACTIVITY_PREFIX, lat_rounded, lon_rounded, radius, limit
    )
}

/// 生成用户活动历史缓存键
pub fn user_activities_key(user_id: &str) -> String {
    format!("{}{}", USER_ACTIVITIES_PREFIX, user_id)
}
