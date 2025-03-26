use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// 缓存相关常量
const USER_LOCATION_CACHE_PREFIX: &str = "user:loc:"; // 用户位置缓存前缀
const ACTIVITY_CACHE_PREFIX: &str = "activity:"; // 活动缓存前缀
const CACHE_EXPIRE: u64 = 120; // 缓存过期时间，单位秒

#[derive(Debug, Serialize, Deserialize)]
pub struct NearbyUser {
    pub user_id: String,
    pub nickname: String,
    pub last_active: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub distance: f64,
    pub avatar: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserActivity {
    pub activity_id: String,
    pub user_id: String,
    pub nickname: String,
    pub activity_type: String,
    pub activity_details: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub created_at: DateTime<Utc>,
    pub distance: f64,
    pub avatar: Option<String>,
}

// 计算球面距离的函数（基于经纬度）
fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    // 使用Haversine公式计算距离
    let r = 6371000.0; // 地球半径（米）
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let delta_phi = (lat2 - lat1).to_radians();
    let delta_lambda = (lon2 - lon1).to_radians();

    let a = (delta_phi / 2.0).sin().powi(2) + 
            phi1.cos() * phi2.cos() * (delta_lambda / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    r * c // 返回距离（米）
}

impl NearbyUser {
    pub async fn find_by_location(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        latitude: f64,
        longitude: f64,
        radius: f64,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let limit = limit.unwrap_or(20).min(50); // 最多返回50条记录

        // 缓存键：精确到小数点后两位的坐标和半径
        let lat_rounded = (latitude * 100.0).round() / 100.0;
        let lon_rounded = (longitude * 100.0).round() / 100.0;
        let cache_key = format!(
            "{}{}:{}:{}:{}",
            USER_LOCATION_CACHE_PREFIX, lat_rounded, lon_rounded, radius, limit
        );

        // 尝试从缓存获取
        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            let cached: redis::RedisResult<String> = conn.get(&cache_key).await;
            if let Ok(json_str) = cached {
                if let Ok(users) = serde_json::from_str::<Vec<NearbyUser>>(&json_str) {
                    tracing::debug!("Get nearby users from cache: {}", cache_key);
                    return Ok(users);
                }
            }
        }

        // 数据库查询，使用近似范围先过滤
        let lat_range = radius / 111000.0; // 1度纬度约111km
        let lon_range = radius / (111000.0 * latitude.to_radians().cos());

        // 获取用户位置记录
        // 注意：需要在数据库中创建user_locations表
        let users = sqlx::query_as!(
            RawNearbyUser,
            r#"
            WITH recent_locations AS (
                SELECT DISTINCT ON (ul.user_id)
                    ul.user_id,
                    ul.latitude,
                    ul.longitude,
                    ul.updated_at,
                    u.nickname
                FROM user_locations ul
                JOIN users u ON ul.user_id = u.user_id
                WHERE 
                    ul.latitude BETWEEN ($1 - $3::float8) AND ($1 + $3::float8)
                    AND ul.longitude BETWEEN ($2 - $4::float8) AND ($2 + $4::float8)
                    AND ul.updated_at > NOW() - INTERVAL '3 days'
                ORDER BY ul.user_id, ul.updated_at DESC
            )
            SELECT 
                rl.user_id,
                rl.nickname,
                rl.latitude,
                rl.longitude,
                rl.updated_at as last_active,
                COALESCE(ul.status, '在线') as status,
                ul.avatar
            FROM recent_locations rl
            LEFT JOIN user_profiles ul ON rl.user_id = ul.user_id
            ORDER BY rl.updated_at DESC
            LIMIT $5
            "#,
            latitude,
            longitude,
            lat_range,
            lon_range,
            limit
        )
        .fetch_all(pool)
        .await?;

        // 计算距离并过滤
        let mut nearby_users = Vec::new();
        for user in users {
            let distance = calculate_distance(
                latitude,
                longitude,
                user.latitude,
                user.longitude,
            );

            // 仅包含在指定半径内的用户
            if distance <= radius {
                nearby_users.push(NearbyUser {
                    user_id: user.user_id,
                    nickname: user.nickname,
                    last_active: user.last_active,
                    latitude: user.latitude,
                    longitude: user.longitude,
                    distance,
                    avatar: user.avatar,
                    status: user.status.unwrap_or_else(|| "在线".to_string()),
                });
            }
        }

        // 按距离排序
        nearby_users.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        // 缓存结果（2分钟）
        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            if let Ok(json_str) = serde_json::to_string(&nearby_users) {
                let _: Result<(), redis::RedisError> = conn.set_ex(&cache_key, json_str, CACHE_EXPIRE).await;
                tracing::debug!("Set nearby users to cache: {}", cache_key);
            }
        }

        Ok(nearby_users)
    }
}

impl UserActivity {
    pub async fn find_recent_activities(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        latitude: f64,
        longitude: f64,
        radius: f64,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let limit = limit.unwrap_or(20).min(50); // 最多返回50条记录

        // 缓存键
        let lat_rounded = (latitude * 100.0).round() / 100.0;
        let lon_rounded = (longitude * 100.0).round() / 100.0;
        let cache_key = format!(
            "{}{}:{}:{}:{}",
            ACTIVITY_CACHE_PREFIX, lat_rounded, lon_rounded, radius, limit
        );

        // 尝试从缓存获取
        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            let cached: redis::RedisResult<String> = conn.get(&cache_key).await;
            if let Ok(json_str) = cached {
                if let Ok(activities) = serde_json::from_str::<Vec<UserActivity>>(&json_str) {
                    tracing::debug!("Get activities from cache: {}", cache_key);
                    return Ok(activities);
                }
            }
        }

        // 数据库查询范围近似
        let lat_range = radius / 111000.0;
        let lon_range = radius / (111000.0 * latitude.to_radians().cos());

        // 获取用户活动记录
        let activities = sqlx::query_as!(
            RawUserActivity,
            r#"
            SELECT 
                a.activity_id,
                a.user_id,
                u.nickname,
                a.activity_type,
                a.activity_details,
                a.latitude,
                a.longitude,
                a.created_at,
                up.avatar
            FROM user_activities a
            JOIN users u ON a.user_id = u.user_id
            LEFT JOIN user_profiles up ON a.user_id = up.user_id
            WHERE 
                a.latitude BETWEEN ($1 - $3::float8) AND ($1 + $3::float8)
                AND a.longitude BETWEEN ($2 - $4::float8) AND ($2 + $4::float8)
                AND a.created_at > NOW() - INTERVAL '3 days'
            ORDER BY a.created_at DESC
            LIMIT $5
            "#,
            latitude,
            longitude,
            lat_range,
            lon_range,
            limit
        )
        .fetch_all(pool)
        .await?;

        // 计算距离并过滤
        let mut recent_activities = Vec::new();
        for activity in activities {
            let distance = calculate_distance(
                latitude,
                longitude,
                activity.latitude,
                activity.longitude,
            );

            // 仅包含在指定半径内的活动
            if distance <= radius {
                recent_activities.push(UserActivity {
                    activity_id: activity.activity_id,
                    user_id: activity.user_id,
                    nickname: activity.nickname,
                    activity_type: activity.activity_type,
                    activity_details: activity.activity_details,
                    latitude: activity.latitude,
                    longitude: activity.longitude,
                    created_at: activity.created_at,
                    distance,
                    avatar: activity.avatar,
                });
            }
        }

        // 按时间排序（最新的在前）
        recent_activities.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // 缓存结果（2分钟）
        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            if let Ok(json_str) = serde_json::to_string(&recent_activities) {
                let _: Result<(), redis::RedisError> = conn.set_ex(&cache_key, json_str, CACHE_EXPIRE).await;
                tracing::debug!("Set activities to cache: {}", cache_key);
            }
        }

        Ok(recent_activities)
    }

    // 创建用户活动
    pub async fn create(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        user_id: &str,
        activity_type: &str,
        activity_details: Option<&str>,
        latitude: f64,
        longitude: f64,
    ) -> Result<String, sqlx::Error> {
        // 生成唯一活动ID
        let activity_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // 插入活动记录到数据库
        sqlx::query!(
            r#"
            INSERT INTO user_activities
                (activity_id, user_id, activity_type, activity_details, latitude, longitude, created_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7)
            "#,
            activity_id,
            user_id,
            activity_type,
            activity_details,
            latitude,
            longitude,
            now
        )
        .execute(pool)
        .await?;

        // 同时更新用户位置记录
        sqlx::query!(
            r#"
            INSERT INTO user_locations
                (user_id, latitude, longitude, updated_at)
            VALUES
                ($1, $2, $3, $4)
            ON CONFLICT (user_id)
            DO UPDATE SET
                latitude = EXCLUDED.latitude,
                longitude = EXCLUDED.longitude,
                updated_at = EXCLUDED.updated_at
            "#,
            user_id,
            latitude,
            longitude,
            now
        )
        .execute(pool)
        .await?;

        // 获取用户所在区域的缓存键
        let lat_rounded = (latitude * 100.0).round() / 100.0;
        let lon_rounded = (longitude * 100.0).round() / 100.0;
        let user_location_pattern = format!("{}{:.*}:{:.*}:*", USER_LOCATION_CACHE_PREFIX, 2, lat_rounded, 2, lon_rounded);
        let activity_pattern = format!("{}{:.*}:{:.*}:*", ACTIVITY_CACHE_PREFIX, 2, lat_rounded, 2, lon_rounded);

        // 清除相关区域的缓存
        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            // 查找并删除匹配的缓存键
            let user_keys: Vec<String> = redis::cmd("KEYS")
                .arg(&user_location_pattern)
                .query_async(&mut conn)
                .await
                .unwrap_or_default();

            let activity_keys: Vec<String> = redis::cmd("KEYS")
                .arg(&activity_pattern)
                .query_async(&mut conn)
                .await
                .unwrap_or_default();

            // 删除查找到的缓存键
            for key in user_keys {
                let _: Result<(), redis::RedisError> = conn.del(&key).await;
            }

            for key in activity_keys {
                let _: Result<(), redis::RedisError> = conn.del(&key).await;
            }
        }

        Ok(activity_id)
    }

    // 根据用户ID查找活动
    pub async fn find_by_user_id(
        pool: &PgPool,
        user_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let limit = limit.unwrap_or(20).min(50); // 最多返回50条记录

        // 获取用户活动记录
        let activities = sqlx::query_as!(
            RawUserActivity,
            r#"
            SELECT 
                a.activity_id,
                a.user_id,
                u.nickname,
                a.activity_type,
                a.activity_details,
                a.latitude,
                a.longitude,
                a.created_at,
                up.avatar
            FROM user_activities a
            JOIN users u ON a.user_id = u.user_id
            LEFT JOIN user_profiles up ON a.user_id = up.user_id
            WHERE a.user_id = $1
            ORDER BY a.created_at DESC
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(pool)
        .await?;

        // 转换为UserActivity
        let user_activities = activities
            .into_iter()
            .map(|a| UserActivity {
                activity_id: a.activity_id,
                user_id: a.user_id,
                nickname: a.nickname,
                activity_type: a.activity_type,
                activity_details: a.activity_details,
                latitude: a.latitude,
                longitude: a.longitude,
                created_at: a.created_at,
                distance: 0.0, // 不需要计算距离
                avatar: a.avatar,
            })
            .collect();

        Ok(user_activities)
    }

    // 查找附近的活动
    pub async fn find_nearby_activities(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        latitude: f64,
        longitude: f64,
        radius: f64,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        // 直接复用find_recent_activities函数逻辑
        Self::find_recent_activities(pool, redis, latitude, longitude, radius, limit).await
    }
}

// 定义原始数据结构，用于从数据库查询
struct RawNearbyUser {
    user_id: String,
    nickname: String,
    last_active: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    status: Option<String>,
    avatar: Option<String>,
}

// 定义原始数据结构，用于从数据库查询
struct RawUserActivity {
    activity_id: String,
    user_id: String,
    nickname: String,
    activity_type: String,
    activity_details: Option<String>,
    latitude: f64,
    longitude: f64,
    created_at: DateTime<Utc>,
    avatar: Option<String>,
} 