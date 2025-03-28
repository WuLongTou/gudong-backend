// 活动存储库
// 包含活动相关的数据库操作

use crate::database::entities::activity::ActivityEntity;
use sqlx::{PgPool, Error as SqlxError};
use std::sync::Arc;
use uuid::Uuid;

/// 近期用户活动信息
pub struct NearbyUserActivity {
    pub user_id: String,
    pub nickname: String,
    pub last_activity_id: Option<String>,
    pub last_activity_type: Option<String>,
    pub last_activity_description: Option<String>,
    pub last_activity_time: Option<chrono::DateTime<chrono::Utc>>,
    pub distance: Option<f64>,
}

/// 活动存储库，处理所有与活动相关的数据库操作
pub struct ActivityRepository {
    db: Arc<PgPool>,
}

impl ActivityRepository {
    /// 创建新的活动存储库实例
    pub fn new(db: Arc<PgPool>) -> Self {
        Self { db }
    }
    
    /// 创建活动
    pub async fn create_activity(
        &self,
        user_id: &str,
        activity_type: &str, 
        activity_details: Option<&str>,
        latitude: f64,
        longitude: f64
    ) -> Result<String, SqlxError> {
        let activity_id = Uuid::new_v4().to_string();
        
        // 根据user_activities表的实际结构
        sqlx::query!(
            r#"
            INSERT INTO user_activities (
                activity_id, user_id, activity_type, activity_details, latitude, longitude
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            activity_id,
            user_id,
            activity_type,
            activity_details,
            latitude,
            longitude
        )
        .execute(&*self.db)
        .await?;
        
        Ok(activity_id)
    }
    
    /// 查找附近活动
    pub async fn find_nearby_activities(
        &self, 
        latitude: f64, 
        longitude: f64,
        radius: f64,
        limit: i64
    ) -> Result<Vec<ActivityEntity>, SqlxError> {
        let actual_limit = if limit <= 0 { 20 } else { limit };
        
        // 使用近似范围和Haversine公式计算距离
        let lat_range = radius / 111000.0; // 1度纬度约111km
        let lon_range = radius / (111000.0 * latitude.to_radians().cos());
        
        // 将user_activities表中的数据映射到ActivityEntity
        let activities = sqlx::query!(
            r#"
            SELECT 
                a.activity_id as "id!",
                CASE 
                    WHEN a.activity_type = 'USER_CHECKIN' THEN 2
                    WHEN a.activity_type = 'GROUP_CREATE' THEN 10
                    WHEN a.activity_type = 'USER_JOINED' THEN 11
                    WHEN a.activity_type = 'MESSAGE_SENT' THEN 20
                    ELSE 1
                END as "activity_type!",
                a.user_id as "user_id!",
                NULL as "group_id",
                a.activity_details as "content",
                COALESCE(a.activity_details, a.activity_type) as "description!",
                a.longitude as "longitude!",
                a.latitude as "latitude!",
                a.created_at as "created_at!",
                -- 使用Haversine公式计算距离（米）
                2.0 * 6371000.0 * asin(sqrt(
                    power(sin(radians(a.latitude - $1::float8) / 2.0), 2.0) + 
                    cos(radians($1::float8)) * cos(radians(a.latitude)) * 
                    power(sin(radians(a.longitude - $2::float8) / 2.0), 2.0)
                )) AS "distance"
            FROM user_activities a
            WHERE 
                a.latitude BETWEEN $1 - $3 AND $1 + $3
                AND a.longitude BETWEEN $2 - $4 AND $2 + $4
            ORDER BY a.created_at DESC
            LIMIT $5
            "#,
            latitude,
            longitude,
            lat_range,
            lon_range,
            actual_limit
        )
        .fetch_all(&*self.db)
        .await?;
        
        // 转换为ActivityEntity结构
        let entities = activities.into_iter()
            .filter(|a| a.distance.unwrap_or(f64::MAX) <= radius) // 过滤真正在半径内的活动
            .map(|a| ActivityEntity {
                id: a.id,
                activity_type: a.activity_type,
                user_id: a.user_id,
                group_id: a.group_id,
                content: a.content,
                description: a.description,
                longitude: a.longitude,
                latitude: a.latitude,
                created_at: a.created_at,
            })
            .collect();
        
        Ok(entities)
    }
    
    /// 获取用户活动
    pub async fn find_user_activities(
        &self,
        user_id: &str,
        limit: i64
    ) -> Result<Vec<ActivityEntity>, SqlxError> {
        let actual_limit = if limit <= 0 { 20 } else { limit };
        
        // 查询用户的活动记录
        let activities = sqlx::query!(
            r#"
            SELECT 
                a.activity_id as "id!",
                CASE 
                    WHEN a.activity_type = 'USER_CHECKIN' THEN 2
                    WHEN a.activity_type = 'GROUP_CREATE' THEN 10
                    WHEN a.activity_type = 'USER_JOINED' THEN 11
                    WHEN a.activity_type = 'MESSAGE_SENT' THEN 20
                    ELSE 1
                END as "activity_type!",
                a.user_id as "user_id!",
                NULL as "group_id",
                a.activity_details as "content",
                COALESCE(a.activity_details, a.activity_type) as "description!",
                a.longitude as "longitude!",
                a.latitude as "latitude!",
                a.created_at as "created_at!"
            FROM user_activities a
            WHERE a.user_id = $1
            ORDER BY a.created_at DESC
            LIMIT $2
            "#,
            user_id,
            actual_limit
        )
        .fetch_all(&*self.db)
        .await?;
        
        // 转换为ActivityEntity结构
        let entities = activities.into_iter()
            .map(|a| ActivityEntity {
                id: a.id,
                activity_type: a.activity_type,
                user_id: a.user_id,
                group_id: a.group_id,
                content: a.content,
                description: a.description,
                longitude: a.longitude,
                latitude: a.latitude,
                created_at: a.created_at,
            })
            .collect();
        
        Ok(entities)
    }
    
    /// 获取群组活动
    pub async fn find_group_activities(
        &self,
        group_id: &str,
        limit: i64
    ) -> Result<Vec<ActivityEntity>, SqlxError> {
        let actual_limit = if limit <= 0 { 20 } else { limit };
        
        // 查询与群组相关的活动
        // 注意：user_activities表中没有直接的group_id字段
        // 但我们可以查询group相关的活动类型
        let activities = sqlx::query!(
            r#"
            SELECT 
                a.activity_id as "id!",
                CASE 
                    WHEN a.activity_type = 'USER_CHECKIN' THEN 2
                    WHEN a.activity_type = 'GROUP_CREATE' THEN 10
                    WHEN a.activity_type = 'USER_JOINED' THEN 11
                    WHEN a.activity_type = 'MESSAGE_SENT' THEN 20
                    ELSE 1
                END as "activity_type!",
                a.user_id as "user_id!",
                $1::varchar as "group_id",
                a.activity_details as "content",
                COALESCE(a.activity_details, a.activity_type) as "description!",
                a.longitude as "longitude!",
                a.latitude as "latitude!",
                a.created_at as "created_at!"
            FROM user_activities a
            JOIN group_members gm ON a.user_id = gm.user_id
            WHERE gm.group_id = $1
              AND a.activity_type IN ('GROUP_CREATE', 'USER_JOINED', 'MESSAGE_SENT')
            ORDER BY a.created_at DESC
            LIMIT $2
            "#,
            group_id,
            actual_limit
        )
        .fetch_all(&*self.db)
        .await?;
        
        // 转换为ActivityEntity结构
        let entities = activities.into_iter()
            .map(|a| ActivityEntity {
                id: a.id,
                activity_type: a.activity_type,
                user_id: a.user_id,
                group_id: a.group_id.map(|id| id),
                content: a.content,
                description: a.description,
                longitude: a.longitude,
                latitude: a.latitude,
                created_at: a.created_at,
            })
            .collect();
        
        Ok(entities)
    }
    
    /// 查找附近用户
    pub async fn find_nearby_users(
        &self,
        latitude: f64,
        longitude: f64,
        radius: f64,
        limit: i64
    ) -> Result<Vec<NearbyUserActivity>, SqlxError> {
        let actual_limit = if limit <= 0 { 20 } else { limit };
        
        // 使用近似范围和Haversine公式计算距离
        let lat_range = radius / 111000.0; // 1度纬度约111km
        let lon_range = radius / (111000.0 * latitude.to_radians().cos());
        
        // 查询附近的用户活动
        let nearby_users = sqlx::query!(
            r#"
            WITH recent_activities AS (
                SELECT DISTINCT ON (user_id)
                    user_id,
                    activity_id,
                    activity_type,
                    activity_details,
                    created_at,
                    latitude,
                    longitude
                FROM user_activities
                ORDER BY user_id, created_at DESC
            )
            SELECT 
                u.user_id,
                u.nickname,
                ra.activity_id as last_activity_id,
                ra.activity_type as last_activity_type,
                ra.activity_details as last_activity_description,
                ra.created_at as last_activity_time,
                ra.latitude,
                ra.longitude,
                -- 使用Haversine公式计算距离（米）
                2.0 * 6371000.0 * asin(sqrt(
                    power(sin(radians(ra.latitude - $1::float8) / 2.0), 2.0) + 
                    cos(radians($1::float8)) * cos(radians(ra.latitude)) * 
                    power(sin(radians(ra.longitude - $2::float8) / 2.0), 2.0)
                )) AS distance
            FROM users u
            JOIN recent_activities ra ON u.user_id = ra.user_id
            WHERE 
                ra.latitude BETWEEN $1 - $3 AND $1 + $3
                AND ra.longitude BETWEEN $2 - $4 AND $2 + $4
            ORDER BY distance
            LIMIT $5
            "#,
            latitude,
            longitude,
            lat_range,
            lon_range,
            actual_limit
        )
        .fetch_all(&*self.db)
        .await?;
        
        // 转换为NearbyUserActivity结构并过滤真正在范围内的用户
        let result = nearby_users.into_iter()
            .filter(|u| u.distance.unwrap_or(f64::MAX) <= radius)
            .map(|u| NearbyUserActivity {
                user_id: u.user_id,
                nickname: u.nickname,
                last_activity_id: Some(u.last_activity_id),
                last_activity_type: Some(u.last_activity_type),
                last_activity_description: u.last_activity_description,
                last_activity_time: Some(u.last_activity_time),
                distance: u.distance,
            })
            .collect();
        
        Ok(result)
    }
    
    /// 删除活动
    pub async fn delete_activity(&self, activity_id: &str) -> Result<bool, SqlxError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM user_activities
            WHERE activity_id = $1
            "#,
            activity_id
        )
        .execute(&*self.db)
        .await?;
        
        Ok(result.rows_affected() > 0)
    }
} 