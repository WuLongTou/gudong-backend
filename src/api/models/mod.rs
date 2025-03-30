// API 数据传输对象模块
// 包含所有与前端交互的数据结构

pub mod activity;
pub mod common;
pub mod group;
pub mod message;
pub mod user;

// 重新导出常用类型
pub use activity::*;
pub use common::*;
pub use group::*;
pub use message::*;
pub use user::*;
