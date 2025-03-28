/// 群组缓存键前缀
const GROUP_ID_PREFIX: &str = "group:id:";

/// 群组名称缓存键前缀
const GROUP_NAME_PREFIX: &str = "group:name:";

/// 群组位置缓存键前缀
const GROUP_LOCATION_PREFIX: &str = "group:loc:";

/// 群组GEO索引键
pub const GROUP_GEO_KEY: &str = "groups:geo";

/// 生成群组ID缓存键
pub fn group_id_key(group_id: &str) -> String {
    format!("{}{}", GROUP_ID_PREFIX, group_id)
}

/// 生成群组名称缓存键
pub fn group_name_key(name: &str) -> String {
    format!("{}{}", GROUP_NAME_PREFIX, name)
}

/// 生成附近群组缓存键
pub fn nearby_groups_key(lat: f64, lon: f64, radius: f64) -> String {
    // 精确到小数点后两位的坐标
    let lat_rounded = (lat * 100.0).round() / 100.0;
    let lon_rounded = (lon * 100.0).round() / 100.0;
    format!("{}{}:{}:{}", GROUP_LOCATION_PREFIX, lat_rounded, lon_rounded, radius)
}

/// 生成群组成员缓存键
pub fn group_members_key(group_id: &str) -> String {
    format!("{}{}:members", GROUP_ID_PREFIX, group_id)
} 