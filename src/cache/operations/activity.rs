use crate::cache::keys::{ACTIVITY_GEO_KEY, USER_GEO_KEY, activity_cache_key, user_cache_key};
use crate::cache::models::activity::{
    CachedNearbyUser, CachedUserActivity, NearbyUser, UserActivity,
};
use redis::{AsyncCommands, Client as RedisClient};
use std::sync::Arc;

/// 缓存过期时间（秒）
pub const CACHE_EXPIRE: u64 = 120;

/// 活动缓存操作
pub struct ActivityCacheOperations {
    redis_client: Arc<RedisClient>,
}

impl ActivityCacheOperations {
    /// 创建新的活动缓存操作实例
    pub fn new(redis_client: Arc<RedisClient>) -> Self {
        Self { redis_client }
    }

    /// 缓存附近用户
    pub async fn cache_nearby_users(&self, users: &Vec<NearbyUser>) -> redis::RedisResult<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        for user in users {
            // 将用户位置添加到GEO索引
            let _: () = redis::cmd("GEOADD")
                .arg(USER_GEO_KEY)
                .arg(user.longitude)
                .arg(user.latitude)
                .arg(&user.user_id)
                .query_async(&mut conn)
                .await?;

            // 缓存用户详情
            let cached_user = CachedNearbyUser::from(user);
            let user_json = serde_json::to_string(&cached_user).unwrap_or_default();

            let _: () = conn
                .set_ex(user_cache_key(&user.user_id), user_json, CACHE_EXPIRE)
                .await?;
        }

        Ok(())
    }

    /// 基于地理坐标查找附近用户
    pub async fn find_nearby_users_geo(
        &self,
        latitude: f64,
        longitude: f64,
        radius: f64,
    ) -> Vec<NearbyUser> {
        let mut conn = match self.redis_client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Redis connection error: {}", e);
                return Vec::new();
            }
        };

        // 使用GEORADIUS命令查找附近用户
        let users_result: redis::RedisResult<Vec<(String, f64)>> = redis::cmd("GEORADIUS")
            .arg(USER_GEO_KEY)
            .arg(longitude)
            .arg(latitude)
            .arg(radius)
            .arg("km")
            .arg("WITHDIST")
            .query_async(&mut conn)
            .await;

        let mut nearby_users = Vec::new();

        match users_result {
            Ok(users) => {
                for (user_id, distance) in users {
                    // 获取用户详情
                    let user_data: redis::RedisResult<String> =
                        conn.get(user_cache_key(&user_id)).await;

                    if let Ok(user_data) = user_data {
                        if let Ok(mut cached_user) =
                            serde_json::from_str::<CachedNearbyUser>(&user_data)
                        {
                            // 更新距离
                            cached_user.distance = distance;

                            // 转换为API模型
                            nearby_users.push(NearbyUser::from(cached_user));
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error searching nearby users: {}", e);
            }
        }

        nearby_users
    }

    /// 缓存附近活动
    pub async fn cache_nearby_activities(
        &self,
        activities: &Vec<UserActivity>,
    ) -> redis::RedisResult<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        for activity in activities {
            // 将活动位置添加到GEO索引
            let _: () = redis::cmd("GEOADD")
                .arg(ACTIVITY_GEO_KEY)
                .arg(activity.longitude)
                .arg(activity.latitude)
                .arg(&activity.activity_id)
                .query_async(&mut conn)
                .await?;

            // 缓存活动详情
            let cached_activity = CachedUserActivity::from(activity);
            let activity_json = serde_json::to_string(&cached_activity).unwrap_or_default();

            let _: () = conn
                .set_ex(
                    activity_cache_key(&activity.activity_id),
                    activity_json,
                    CACHE_EXPIRE,
                )
                .await?;
        }

        Ok(())
    }

    /// 基于地理坐标查找附近活动
    pub async fn find_nearby_activities_geo(
        &self,
        latitude: f64,
        longitude: f64,
        radius: f64,
    ) -> Vec<UserActivity> {
        let mut conn = match self.redis_client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Redis connection error: {}", e);
                return Vec::new();
            }
        };

        // 使用GEORADIUS命令查找附近活动
        let activities_result: redis::RedisResult<Vec<(String, f64)>> = redis::cmd("GEORADIUS")
            .arg(ACTIVITY_GEO_KEY)
            .arg(longitude)
            .arg(latitude)
            .arg(radius)
            .arg("km")
            .arg("WITHDIST")
            .query_async(&mut conn)
            .await;

        let mut nearby_activities = Vec::new();

        match activities_result {
            Ok(activities) => {
                for (activity_id, distance) in activities {
                    // 获取活动详情
                    let activity_data: redis::RedisResult<String> =
                        conn.get(activity_cache_key(&activity_id)).await;

                    if let Ok(activity_data) = activity_data {
                        if let Ok(mut cached_activity) =
                            serde_json::from_str::<CachedUserActivity>(&activity_data)
                        {
                            // 更新距离
                            cached_activity.distance = distance;

                            // 转换为API模型
                            nearby_activities.push(UserActivity::from(cached_activity));
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error searching nearby activities: {}", e);
            }
        }

        nearby_activities
    }

    /// 更新用户位置
    pub async fn update_user_location(
        &self,
        user_id: &str,
        latitude: f64,
        longitude: f64,
    ) -> redis::RedisResult<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        // 更新GEO索引中的用户位置
        let _: () = redis::cmd("GEOADD")
            .arg(USER_GEO_KEY)
            .arg(longitude)
            .arg(latitude)
            .arg(user_id)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    /// 创建活动同时更新缓存
    pub async fn create_activity(&self, activity: &UserActivity) -> redis::RedisResult<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        // 将活动位置添加到GEO索引
        let _: () = redis::cmd("GEOADD")
            .arg(ACTIVITY_GEO_KEY)
            .arg(activity.longitude)
            .arg(activity.latitude)
            .arg(&activity.activity_id)
            .query_async(&mut conn)
            .await?;

        // 缓存活动详情
        let cached_activity = CachedUserActivity::from(activity);
        let activity_json = serde_json::to_string(&cached_activity).unwrap_or_default();

        let _: () = conn
            .set_ex(
                activity_cache_key(&activity.activity_id),
                activity_json,
                CACHE_EXPIRE,
            )
            .await?;

        Ok(())
    }
}
