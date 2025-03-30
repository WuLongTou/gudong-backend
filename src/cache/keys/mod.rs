/// 缓存键模块
/// 提供各种缓存键生成函数
// 用户缓存键模块
pub mod user_keys;

// 活动缓存键模块
pub mod activity_keys;

// 群组缓存键模块
pub mod group_keys;

// 重新导出常用的键生成函数
pub use activity_keys::{
    ACTIVITY_GEO_KEY, USER_GEO_KEY, activity_cache_key, nearby_activities_key,
    nearby_users_key as nearby_users_location_key, user_activities_key, user_cache_key,
};
pub use group_keys::{
    GROUP_GEO_KEY, group_id_key, group_members_key, group_name_key, nearby_groups_key,
};
pub use user_keys::{nearby_users_key, user_info_key, user_status_key};
