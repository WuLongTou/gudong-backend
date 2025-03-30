// API 处理器模块
// 包含所有 API 请求处理逻辑

pub mod activity;
pub mod group;
pub mod message;
pub mod test;
pub mod user;

// 重新导出常用处理器
pub use activity::*;
pub use group::*;
pub use message::*;
pub use test::*;
pub use user::*;
