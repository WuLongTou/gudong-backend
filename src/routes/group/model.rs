use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::utils::{calculate_distance, hash_password, verify_password};

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub group_id: String,
    pub name: String,
    pub location_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub creator_id: String,
    pub created_at: DateTime<Utc>,
    pub member_count: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub location_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: String,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JoinGroupRequest {
    pub group_id: String,
    pub password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GroupMember {
    pub group_id: String,
    pub user_id: String,
    pub joined_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct KeepAliveRequest {
    pub group_id: String,
}

#[derive(Debug, Serialize)]
pub struct GroupInfo {
    pub group_id: String,
    pub name: String,
    pub location_name: String,
    pub description: String,
    pub member_count: i32,
    pub is_need_password: bool,
}

impl From<Group> for GroupInfo {
    fn from(group: Group) -> Self {
        Self {
            group_id: group.group_id,
            name: group.name,
            location_name: group.location_name,
            description: group.description,
            member_count: group.member_count,
            is_need_password: group.password_hash.is_some(),
        }
    }
}

impl Group {
    pub async fn create(
        pool: &PgPool,
        req: CreateGroupRequest,
        creator_id: String,
    ) -> Result<Self, sqlx::Error> {
        let group_id = Uuid::new_v4().to_string();
        let password_hash = req
            .password
            .map(|pwd| hash_password(&pwd))
            .transpose()
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to hash password: {}", e)))?;

        let group = sqlx::query_as!(
            Group,
            r#"
            INSERT INTO groups (
                group_id, name, location_name, latitude, longitude,
                description, password_hash, creator_id, created_at, member_count
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), 1)
            RETURNING 
                group_id, name, location_name, latitude, longitude,
                description, password_hash, creator_id, created_at, member_count
            "#,
            group_id,
            req.name,
            req.location_name,
            req.latitude,
            req.longitude,
            req.description,
            password_hash,
            creator_id,
        )
        .fetch_one(pool)
        .await?;

        // 创建群组的同时把创建者加入群组
        sqlx::query!(
            r#"
            INSERT INTO group_members (group_id, user_id, joined_at)
            VALUES ($1, $2, NOW())
            "#,
            group_id,
            creator_id,
        )
        .execute(pool)
        .await?;

        Ok(group)
    }

    pub async fn find_by_id(pool: &PgPool, group_id: &str) -> Result<Option<Self>, sqlx::Error> {
        let group = sqlx::query_as!(
            Group,
            r#"
            SELECT 
                group_id, name, location_name, latitude, longitude,
                description, password_hash, creator_id, created_at, member_count
            FROM groups
            WHERE group_id = $1
            "#,
            group_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(group)
    }

    pub async fn find_by_name(pool: &PgPool, name: &str) -> Result<Vec<Self>, sqlx::Error> {
        let groups = sqlx::query_as!(
            Group,
            r#"
            SELECT 
                group_id, name, location_name, latitude, longitude,
                description, password_hash, creator_id, created_at, member_count
            FROM groups
            WHERE name LIKE $1
            "#,
            format!("%{}%", name)
        )
        .fetch_all(pool)
        .await?;

        Ok(groups)
    }

    pub async fn find_by_location(
        pool: &PgPool,
        latitude: f64,
        longitude: f64,
        radius: f64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        // 使用近似计算方法，先用经纬度范围过滤，再精确计算距离
        let lat_range = radius / 111000.0; // 1度纬度约111km
        let lon_range = radius / (111000.0 * latitude.to_radians().cos());

        let groups = sqlx::query_as!(
            Group,
            r#"
            SELECT 
                group_id, name, location_name, latitude, longitude,
                description, password_hash, creator_id, created_at, member_count
            FROM groups
            WHERE 
                latitude BETWEEN ($1::DOUBLE PRECISION - $3::DOUBLE PRECISION) 
                AND ($1::DOUBLE PRECISION + $3::DOUBLE PRECISION)
                AND longitude BETWEEN ($2::DOUBLE PRECISION - $4::DOUBLE PRECISION) 
                AND ($2::DOUBLE PRECISION + $4::DOUBLE PRECISION)
            "#,
            latitude,
            longitude,
            lat_range,
            lon_range
        )
        .fetch_all(pool)
        .await?;

        // 精确计算距离并过滤
        let filtered_groups = groups
            .into_iter()
            .filter(|group| {
                calculate_distance(latitude, longitude, group.latitude, group.longitude) <= radius
            })
            .collect();

        Ok(filtered_groups)
    }

    pub async fn join(
        pool: &PgPool,
        group_id: &str,
        user_id: &str,
        password: Option<String>,
    ) -> Result<(), sqlx::Error> {
        let group = Self::find_by_id(pool, group_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // 检查密码
        if let Some(hash) = group.password_hash {
            let password = password.ok_or_else(|| {
                sqlx::Error::Protocol("Password required to join this group".into())
            })?;

            if !verify_password(&password, &hash).map_err(|e| {
                sqlx::Error::Protocol(format!("Password verification failed: {}", e))
            })? {
                return Err(sqlx::Error::Protocol("Invalid password".into()));
            }
        }

        // 检查用户是否已经在群组中
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
        .fetch_one(pool)
        .await?
        .exists;

        if exists {
            return Ok(());
        }

        // 开启事务
        let mut tx = pool.begin().await?;

        // 添加用户到群组
        sqlx::query!(
            r#"
            INSERT INTO group_members (group_id, user_id, joined_at)
            VALUES ($1, $2, NOW())
            "#,
            group_id,
            user_id
        )
        .execute(&mut *tx)
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
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn leave(pool: &PgPool, group_id: &str, user_id: &str) -> Result<(), sqlx::Error> {
        // 检查用户是否在群组中
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
        .fetch_one(pool)
        .await?
        .exists;

        if !exists {
            return Err(sqlx::Error::Protocol("User not in group".into()));
        }

        // 开启事务
        let mut tx = pool.begin().await?;

        // 从群组中移除用户
        sqlx::query!(
            r#"
            DELETE FROM group_members
            WHERE group_id = $1 AND user_id = $2
            "#,
            group_id,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        // 更新群组成员数
        sqlx::query!(
            r#"
            UPDATE groups
            SET member_count = member_count - 1
            WHERE group_id = $1
            "#,
            group_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn keep_alive(
        pool: &PgPool,
        group_id: &str,
        user_id: &str,
    ) -> Result<DateTime<Utc>, sqlx::Error> {
        // 更新用户最后活跃时间
        let updated = sqlx::query!(
            r#"
            UPDATE group_members 
            SET last_active = NOW()
            WHERE group_id = $1 AND user_id = $2
            RETURNING last_active
            "#,
            group_id,
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(updated.last_active)
    }
}
