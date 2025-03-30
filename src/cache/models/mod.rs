/// 缓存数据模型
/// 定义缓存数据的结构体
// 用户缓存模型
pub mod user;

// 活动缓存模型
pub mod activity;

// 群组缓存模型
pub mod group;

pub mod rate_limit;
pub mod session;
pub mod token;

// 重新导出常用类型
pub use activity::{CachedNearbyUser, CachedUserActivity};
pub use group::{CachedGroup, CachedGroupMember, CachedNearbyGroup};
pub use rate_limit::*;
pub use session::*;
pub use token::*;
pub use user::{CachedUser, CachedUserLocation, CachedUserStatus};
