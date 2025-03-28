// 数据库模块
// 包含数据库实体定义和存储库操作

pub mod entities;    // 数据库实体定义
pub mod repositories; // 数据库操作实现

// 重新导出常用类型和函数，方便其他模块使用
pub use entities::user::UserEntity;
pub use repositories::user::UserRepository; 