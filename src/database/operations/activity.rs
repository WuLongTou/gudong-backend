// 活动存储库
// 包含活动相关的数据库操作

use crate::database::models::activity::{ActivityEntity, NearbyUserActivity};
use sqlx::{Error as SqlxError, PgPool, FromRow};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 带距离的活动实体，用于 SQL 查询
#[derive(Debug, FromRow)]
pub struct ActivityEntityWithDistance {
    pub id: String,
    pub activity_type: i32,
    pub user_id: String,
    pub group_id: Option<String>,
    pub content: Option<String>,
    pub description: String,
    pub longitude: f64,
    pub latitude: f64,
    pub created_at: DateTime<Utc>,
    pub distance: Option<f64>,
}

/// 活动存储库，处理所有与活动相关的数据库操作
pub struct ActivityOperation {
    db: Arc<PgPool>,
}

impl ActivityOperation {
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
        longitude: f64,
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
        limit: i64,
    ) -> Result<Vec<ActivityEntity>, SqlxError> {
        let actual_limit = if limit <= 0 { 20 } else { limit };

        // 使用PostGIS的ST_DWithin和ST_Distance函数进行精确的地理空间查询
        // 将经纬度转换为地理坐标点，并使用球面计算距离
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
                -- 使用PostGIS计算精确的球面距离（米）
                ST_Distance(
                    ST_SetSRID(ST_MakePoint(a.longitude, a.latitude), 4326)::geography,
                    ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
                ) as "distance"
            FROM user_activities a
            WHERE ST_DWithin(
                ST_SetSRID(ST_MakePoint(a.longitude, a.latitude), 4326)::geography,
                ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
                $3
            )
            ORDER BY a.created_at DESC
            LIMIT $4
            "#,
            latitude,
            longitude,
            radius,  // 以米为单位的半径
            actual_limit
        )
        .fetch_all(&*self.db)
        .await?;

        // 转换为ActivityEntity结构
        let entities = activities
            .into_iter()
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

    /// 按类型查找附近活动
    pub async fn find_nearby_activities_by_type(
        &self,
        latitude: f64,
        longitude: f64,
        radius: f64,
        limit: i64,
        activity_types: &[&str],
    ) -> Result<Vec<ActivityEntity>, SqlxError> {
        let actual_limit = if limit <= 0 { 20 } else { limit };

        // 构建查询条件，按活动类型过滤
        let types_condition = if activity_types.is_empty() {
            "".to_string()
        } else {
            let types = activity_types
                .iter()
                .map(|t| format!("'{}'", t))
                .collect::<Vec<_>>()
                .join(", ");
            format!("AND a.activity_type IN ({})", types)
        };

        // 动态构建SQL查询
        let query = format!(
            r#"
            SELECT 
                a.activity_id as "id",
                CASE 
                    WHEN a.activity_type = 'USER_CHECKIN' THEN 2
                    WHEN a.activity_type = 'GROUP_CREATE' THEN 10
                    WHEN a.activity_type = 'USER_JOINED' THEN 11
                    WHEN a.activity_type = 'MESSAGE_SENT' THEN 20
                    ELSE 1
                END as "activity_type",
                a.user_id as "user_id",
                NULL as "group_id",
                a.activity_details as "content",
                COALESCE(a.activity_details, a.activity_type) as "description",
                a.longitude as "longitude",
                a.latitude as "latitude",
                a.created_at as "created_at",
                -- 使用PostGIS计算精确的球面距离（米）
                ST_Distance(
                    ST_SetSRID(ST_MakePoint(a.longitude, a.latitude), 4326)::geography,
                    ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
                ) as "distance"
            FROM user_activities a
            WHERE ST_DWithin(
                ST_SetSRID(ST_MakePoint(a.longitude, a.latitude), 4326)::geography,
                ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
                $3
            )
            {}
            ORDER BY a.created_at DESC
            LIMIT $4
            "#,
            types_condition
        );

        // 执行动态SQL查询
        let activities = sqlx::query_as::<_, ActivityEntityWithDistance>(&query)
            .bind(latitude)
            .bind(longitude)
            .bind(radius)
            .bind(actual_limit)
            .fetch_all(&*self.db)
            .await?;

        // 转换为ActivityEntity结构
        let entities = activities
            .into_iter()
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
        limit: i64,
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
        let entities = activities
            .into_iter()
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
        limit: i64,
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
        let entities = activities
            .into_iter()
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
        limit: i64,
    ) -> Result<Vec<NearbyUserActivity>, SqlxError> {
        let actual_limit = if limit <= 0 { 20 } else { limit };

        // 使用PostGIS进行精确的地理空间查询
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
                -- 使用PostGIS计算精确的球面距离（米）
                ST_Distance(
                    ST_SetSRID(ST_MakePoint(ra.longitude, ra.latitude), 4326)::geography,
                    ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
                ) AS distance
            FROM users u
            JOIN recent_activities ra ON u.user_id = ra.user_id
            WHERE ST_DWithin(
                ST_SetSRID(ST_MakePoint(ra.longitude, ra.latitude), 4326)::geography,
                ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
                $3
            )
            ORDER BY distance
            LIMIT $4
            "#,
            latitude,
            longitude,
            radius,  // 以米为单位的半径
            actual_limit
        )
        .fetch_all(&*self.db)
        .await?;

        // 转换为NearbyUserActivity结构
        let result = nearby_users
            .into_iter()
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
