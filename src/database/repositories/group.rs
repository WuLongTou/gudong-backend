// 群组存储库
// 包含群组相关的数据库操作

use crate::database::entities::group::GroupEntity;
use sqlx::{PgPool, Error as SqlxError};
use std::sync::Arc;
use uuid::Uuid;
use crate::utils::{hash_password, verify_password};
use chrono::{DateTime, Utc};

/// 群组存储库，处理所有与群组相关的数据库操作
pub struct GroupRepository {
    db: Arc<PgPool>,
}

impl GroupRepository {
    /// 创建新的群组存储库实例
    pub fn new(db: Arc<PgPool>) -> Self {
        Self { db }
    }
    
    /// 创建群组
    pub async fn create_group(
        &self, 
        name: &str, 
        location_name: &str,
        latitude: f64, 
        longitude: f64,
        description: &str,
        password: Option<&str>,
        creator_id: &str
    ) -> Result<String, SqlxError> {
        let group_id = Uuid::new_v4().to_string();
        
        // 处理可选密码
        let password_hash = match password {
            Some(pwd) => Some(hash_password(pwd)
                .map_err(|e| SqlxError::Protocol(format!("Failed to hash password: {}", e)))?),
            None => None,
        };

        // 创建群组记录
        sqlx::query!(
            r#"
            INSERT INTO groups (
                group_id, name, location_name, latitude, longitude,
                description, password_hash, creator_id, created_at, member_count
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), 1)
            "#,
            group_id,
            name,
            location_name,
            latitude,
            longitude,
            description,
            password_hash,
            creator_id,
        )
        .execute(&*self.db)
        .await?;

        // 创建者加入群组
        sqlx::query!(
            r#"
            INSERT INTO group_members (group_id, user_id, joined_at, last_active)
            VALUES ($1, $2, NOW(), NOW())
            "#,
            group_id,
            creator_id,
        )
        .execute(&*self.db)
        .await?;

        Ok(group_id)
    }
    
    /// 根据名称查找群组
    pub async fn find_groups_by_name(&self, name: &str) -> Result<Vec<GroupEntity>, SqlxError> {
        let search_pattern = format!("%{}%", name);
        
        let groups = sqlx::query_as!(
            GroupEntity,
            r#"
            SELECT 
                group_id as id, 
                name, 
                location_name, 
                latitude, 
                longitude,
                description, 
                password_hash as password, 
                creator_id, 
                created_at, 
                created_at as last_active
            FROM groups
            WHERE name ILIKE $1
            ORDER BY created_at DESC
            LIMIT 20
            "#,
            search_pattern
        )
        .fetch_all(&*self.db)
        .await?;
        
        Ok(groups)
    }
    
    /// 根据位置查找附近群组
    pub async fn find_groups_by_location(
        &self, 
        latitude: f64, 
        longitude: f64, 
        radius: f64
    ) -> Result<Vec<GroupEntity>, SqlxError> {
        // 获取所有群组并在内存中进行过滤，避免使用PostGIS扩展
        let all_groups = sqlx::query_as!(
            GroupEntity,
            r#"
            SELECT 
                group_id as id, 
                name, 
                location_name, 
                latitude, 
                longitude,
                description, 
                password_hash as password, 
                creator_id, 
                created_at, 
                created_at as last_active
            FROM groups
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&*self.db)
        .await?;
        
        // 在内存中过滤附近群组
        let nearby_groups = all_groups.into_iter()
            .filter_map(|group| {
                // 计算距离（简单的欧几里德距离，1度约等于111km）
                let dx = (group.longitude - longitude).abs() * 111.0;
                let dy = (group.latitude - latitude).abs() * 111.0;
                let distance_km = (dx * dx + dy * dy).sqrt();
                let distance_m = distance_km * 1000.0;
                
                // 如果距离小于指定半径，则保留
                if distance_m <= radius {
                    Some(group)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        
        Ok(nearby_groups)
    }
    
    /// 根据ID查找群组
    pub async fn find_group_by_id(&self, group_id: &str) -> Result<Option<GroupEntity>, SqlxError> {
        let group = sqlx::query_as!(
            GroupEntity,
            r#"
            SELECT 
                group_id as id, 
                name, 
                location_name, 
                latitude, 
                longitude,
                description, 
                password_hash as password, 
                creator_id, 
                created_at, 
                created_at as last_active
            FROM groups
            WHERE group_id = $1
            "#,
            group_id
        )
        .fetch_optional(&*self.db)
        .await?;
        
        Ok(group)
    }
    
    /// 添加用户到群组
    pub async fn add_user_to_group(
        &self, 
        group_id: &str, 
        user_id: &str,
        password: Option<&str>
    ) -> Result<(), SqlxError> {
        // 检查群组是否存在
        let group = self.find_group_by_id(group_id).await?
            .ok_or_else(|| SqlxError::RowNotFound)?;
        
        // 检查用户是否已经在群组
        let is_member = self.user_in_group(group_id, user_id).await?;
        if is_member {
            // 用户已在群组，更新最后活跃时间即可
            sqlx::query!(
                r#"
                UPDATE group_members
                SET last_active = NOW()
                WHERE group_id = $1 AND user_id = $2
                "#,
                group_id,
                user_id
            )
            .execute(&*self.db)
            .await?;
            
            return Ok(());
        }
        
        // 检查密码（如果需要）
        if let Some(hash) = group.password {
            let pwd = password.ok_or_else(|| 
                SqlxError::Protocol("Password required to join this group".into()))?;
                
            let valid = verify_password(pwd, &hash)
                .map_err(|e| SqlxError::Protocol(format!("Password verification error: {}", e)))?;
                
            if !valid {
                return Err(SqlxError::Protocol("Invalid password".into()));
            }
        }
        
        // 添加用户到群组
        sqlx::query!(
            r#"
            INSERT INTO group_members (group_id, user_id, joined_at, last_active)
            VALUES ($1, $2, NOW(), NOW())
            "#,
            group_id,
            user_id
        )
        .execute(&*self.db)
        .await?;
        
        // 更新群组成员数
        sqlx::query!(
            r#"
            UPDATE groups
            SET member_count = member_count + 1
            WHERE group_id = $1
            "#,
            group_id
        )
        .execute(&*self.db)
        .await?;
        
        Ok(())
    }
    
    /// 检查群组是否存在
    pub async fn group_exists(&self, group_id: &str) -> Result<bool, SqlxError> {
        let exists = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM groups WHERE group_id = $1
            ) as "exists!"
            "#,
            group_id
        )
        .fetch_one(&*self.db)
        .await?
        .exists;
        
        Ok(exists)
    }
    
    /// 检查用户是否已经在群组中
    pub async fn user_in_group(
        &self, 
        group_id: &str, 
        user_id: &str
    ) -> Result<bool, SqlxError> {
        let exists = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM group_members 
                WHERE group_id = $1 AND user_id = $2
            ) as "exists!"
            "#,
            group_id,
            user_id
        )
        .fetch_one(&*self.db)
        .await?
        .exists;
        
        Ok(exists)
    }
    
    /// 用户离开群组
    pub async fn remove_user_from_group(
        &self,
        group_id: &str,
        user_id: &str
    ) -> Result<(), SqlxError> {
        // 检查用户是否在群组中
        let is_member = self.user_in_group(group_id, user_id).await?;
        if !is_member {
            return Ok(()); // 用户不在群组中，无需操作
        }
        
        // 移除用户
        sqlx::query!(
            r#"
            DELETE FROM group_members
            WHERE group_id = $1 AND user_id = $2
            "#,
            group_id,
            user_id
        )
        .execute(&*self.db)
        .await?;
        
        // 更新群组成员数
        sqlx::query!(
            r#"
            UPDATE groups
            SET member_count = GREATEST(member_count - 1, 0)
            WHERE group_id = $1
            "#,
            group_id
        )
        .execute(&*self.db)
        .await?;
        
        Ok(())
    }
    
    /// 获取群组成员数量
    pub async fn get_member_count(&self, group_id: &str) -> Result<i64, SqlxError> {
        let count = sqlx::query!(
            r#"
            SELECT COUNT(*) as count FROM group_members
            WHERE group_id = $1
            "#,
            group_id
        )
        .fetch_one(&*self.db)
        .await?
        .count
        .unwrap_or(0);
        
        Ok(count)
    }
    
    /// 更新用户在群组中的活跃状态
    pub async fn update_user_activity(
        &self,
        group_id: &str,
        user_id: &str
    ) -> Result<DateTime<Utc>, SqlxError> {
        // 检查用户是否在群组中
        let is_member = self.user_in_group(group_id, user_id).await?;
        if !is_member {
            return Err(SqlxError::Protocol("User is not a member of this group".into()));
        }
        
        // 更新最后活跃时间
        let now = Utc::now();
        
        sqlx::query!(
            r#"
            UPDATE group_members
            SET last_active = $3
            WHERE group_id = $1 AND user_id = $2
            "#,
            group_id,
            user_id,
            now
        )
        .execute(&*self.db)
        .await?;
        
        Ok(now)
    }
    
    /// 获取群组成员列表
    pub async fn get_group_members(
        &self,
        group_id: &str
    ) -> Result<Vec<(String, String, DateTime<Utc>)>, SqlxError> {
        let members = sqlx::query!(
            r#"
            SELECT 
                gm.user_id,
                u.nickname,
                gm.last_active
            FROM group_members gm
            JOIN users u ON gm.user_id = u.user_id
            WHERE gm.group_id = $1
            ORDER BY gm.last_active DESC
            "#,
            group_id
        )
        .fetch_all(&*self.db)
        .await?
        .into_iter()
        .map(|row| (row.user_id, row.nickname, row.last_active))
        .collect();
        
        Ok(members)
    }

    /// 获取用户加入的所有群组
    pub async fn find_groups_by_user_id(&self, user_id: &str) -> Result<Vec<GroupEntity>, SqlxError> {
        let groups = sqlx::query_as!(
            GroupEntity,
            r#"
            SELECT 
                g.group_id as id, 
                g.name, 
                g.location_name, 
                g.latitude, 
                g.longitude,
                g.description, 
                g.password_hash as password, 
                g.creator_id, 
                g.created_at, 
                gm.last_active as "last_active!"
            FROM groups g
            JOIN group_members gm ON g.group_id = gm.group_id
            WHERE gm.user_id = $1
            ORDER BY gm.last_active DESC
            "#,
            user_id
        )
        .fetch_all(&*self.db)
        .await?;
        
        Ok(groups)
    }

    /// 检查用户是否为群组管理员
    pub async fn user_is_admin(
        &self, 
        group_id: &str, 
        user_id: &str
    ) -> Result<bool, SqlxError> {
        // 检查是否是创建者
        let is_creator = sqlx::query!(
            r#"
            SELECT creator_id
            FROM groups
            WHERE group_id = $1
            "#,
            group_id
        )
        .fetch_optional(&*self.db)
        .await?
        .map(|row| row.creator_id == user_id)
        .unwrap_or(false);
        
        if is_creator {
            return Ok(true);
        }
        
        // 检查是否被设置为管理员
        let is_admin = sqlx::query!(
            r#"
            SELECT role
            FROM group_members
            WHERE group_id = $1 AND user_id = $2
            "#,
            group_id,
            user_id
        )
        .fetch_optional(&*self.db)
        .await?
        .map(|row| row.role.unwrap_or_default() == "admin")
        .unwrap_or(false);
        
        Ok(is_admin)
    }
    
    /// 更新用户角色（设置/取消管理员权限）
    pub async fn update_user_role(
        &self,
        group_id: &str,
        user_id: &str,
        is_admin: bool
    ) -> Result<(), SqlxError> {
        // 检查群组是否存在
        let exists = self.group_exists(group_id).await?;
        if !exists {
            return Err(SqlxError::RowNotFound);
        }
        
        // 检查用户是否在群组中
        let is_member = self.user_in_group(group_id, user_id).await?;
        if !is_member {
            return Err(SqlxError::RowNotFound);
        }
        
        // 更新用户角色
        let role = if is_admin { "admin" } else { "member" };
        
        sqlx::query!(
            r#"
            UPDATE group_members
            SET role = $3
            WHERE group_id = $1 AND user_id = $2
            "#,
            group_id,
            user_id,
            role
        )
        .execute(&*self.db)
        .await?;
        
        Ok(())
    }
} 