use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::PgPool;
use std::sync::Arc;
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

// 缓存相关常量
const GROUP_CACHE_EXPIRE: u64 = 600; // 群组缓存过期时间，单位秒
const GROUP_ID_CACHE_PREFIX: &str = "group:id:"; // 群组ID缓存前缀
const GROUP_NAME_CACHE_PREFIX: &str = "group:name:"; // 群组名称缓存前缀
const GROUP_LOCATION_CACHE_PREFIX: &str = "group:loc:"; // 群组位置缓存前缀

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

    pub async fn find_by_id(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        group_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        // 尝试从缓存读取
        let cache_key = format!("{}{}", GROUP_ID_CACHE_PREFIX, group_id);

        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            let cached: redis::RedisResult<String> = conn.get(&cache_key).await;

            if let Ok(json_str) = cached {
                if let Ok(group) = serde_json::from_str::<Group>(&json_str) {
                    tracing::debug!("Get group from cache: {}", cache_key);
                    return Ok(Some(group));
                }
            }
        }

        // 从数据库查询
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

        // 缓存结果
        if let Some(ref g) = group {
            if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
                if let Ok(json_str) = serde_json::to_string(g) {
                    let _: Result<(), redis::RedisError> =
                        conn.set_ex(&cache_key, json_str, GROUP_CACHE_EXPIRE).await;
                    tracing::debug!("Set group to cache: {}", cache_key);
                }
            }
        }

        Ok(group)
    }

    pub async fn find_by_name(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        name: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        // 对于模糊查询，只在名称非常具体（至少5个字符）时使用缓存
        if name.len() >= 5 {
            let cache_key = format!("{}{}", GROUP_NAME_CACHE_PREFIX, name);

            if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
                let cached: redis::RedisResult<String> = conn.get(&cache_key).await;

                if let Ok(json_str) = cached {
                    if let Ok(groups) = serde_json::from_str::<Vec<Group>>(&json_str) {
                        tracing::debug!("Get groups by name from cache: {}", cache_key);
                        return Ok(groups);
                    }
                }
            }
        }

        // 从数据库查询
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

        // 如果名称具体且结果适合缓存，则缓存结果
        if name.len() >= 5 && groups.len() < 50 {
            if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
                let cache_key = format!("{}{}", GROUP_NAME_CACHE_PREFIX, name);
                if let Ok(json_str) = serde_json::to_string(&groups) {
                    let _: Result<(), redis::RedisError> =
                        conn.set_ex(&cache_key, json_str, GROUP_CACHE_EXPIRE).await;
                    tracing::debug!("Set groups by name to cache: {}", cache_key);
                }
            }
        }

        Ok(groups)
    }

    pub async fn find_by_location(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        latitude: f64,
        longitude: f64,
        radius: f64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        // 对于位置查询，将坐标精确到小数点后两位作为缓存key
        let lat_rounded = (latitude * 100.0).round() / 100.0;
        let lon_rounded = (longitude * 100.0).round() / 100.0;
        let cache_key = format!(
            "{}{}:{}:{}",
            GROUP_LOCATION_CACHE_PREFIX, lat_rounded, lon_rounded, radius
        );

        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            let cached: redis::RedisResult<String> = conn.get(&cache_key).await;

            if let Ok(json_str) = cached {
                if let Ok(groups) = serde_json::from_str::<Vec<Group>>(&json_str) {
                    tracing::debug!("Get groups by location from cache: {}", cache_key);
                    return Ok(groups);
                }
            }
        }

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

        // 缓存结果，时间较短，因为位置查询结果变化较快
        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            if let Ok(json_str) = serde_json::to_string(&filtered_groups) {
                let _: Result<(), redis::RedisError> = conn.set_ex(&cache_key, json_str, 120).await; // 仅缓存2分钟
                tracing::debug!("Set groups by location to cache: {}", cache_key);
            }
        }

        Ok(filtered_groups)
    }

    pub async fn join(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        group_id: &str,
        user_id: &str,
        password: Option<String>,
    ) -> Result<(), sqlx::Error> {
        let group = Self::find_by_id(pool, redis, group_id)
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

        // 更新群组成员数后，清除相关缓存
        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            let cache_key = format!("{}{}", GROUP_ID_CACHE_PREFIX, group_id);
            let _: Result<(), redis::RedisError> = conn.del(&cache_key).await;
        }

        Ok(())
    }

    pub async fn leave(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        group_id: &str,
        user_id: &str,
    ) -> Result<(), sqlx::Error> {
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

        // 更新群组成员数后，清除相关缓存
        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            let cache_key = format!("{}{}", GROUP_ID_CACHE_PREFIX, group_id);
            let _: Result<(), redis::RedisError> = conn.del(&cache_key).await;
        }

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
