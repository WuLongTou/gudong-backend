/// 缓存键模块
/// 提供各种缓存键生成函数

// 用户缓存键模块
pub mod user_keys;

// 活动缓存键模块
pub mod activity_keys;

// 群组缓存键模块
pub mod group_keys;

// 重新导出常用的键生成函数
pub use user_keys::{user_info_key, user_status_key, nearby_users_key};
pub use activity_keys::{nearby_users_key as nearby_users_location_key, nearby_activities_key, user_activities_key, USER_GEO_KEY, ACTIVITY_GEO_KEY, user_cache_key, activity_cache_key};
pub use group_keys::{group_id_key, group_name_key, nearby_groups_key, group_members_key, GROUP_GEO_KEY}; 