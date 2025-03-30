use crate::cache::keys::{GROUP_GEO_KEY, group_id_key};
use crate::cache::models::group::{CachedGroup, Group};
use redis::{AsyncCommands, Client as RedisClient};
use std::sync::Arc;

/// 群组缓存操作的过期时间（秒）
pub const GROUP_CACHE_EXPIRE: u64 = 600; // 10分钟

/// 群组缓存操作
pub struct GroupCacheOperations {
    redis_client: Arc<RedisClient>,
}

impl GroupCacheOperations {
    /// 创建新的群组缓存操作实例
    pub fn new(redis_client: Arc<RedisClient>) -> Self {
        Self { redis_client }
    }

    /// 缓存群组信息
    pub async fn cache_group(&self, group: &Group) -> Result<(), redis::RedisError> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        // 群组ID缓存键
        let key = group_id_key(&group.group_id);

        // 将Group转换为CachedGroup
        let cached_group = CachedGroup::from(group);

        // 序列化并保存到Redis
        let json = serde_json::to_string(&cached_group).map_err(|e| {
            redis::RedisError::from((redis::ErrorKind::IoError, "序列化错误", e.to_string()))
        })?;

        // 设置缓存，过期时间10分钟
        let _: () = conn.set_ex(key, json, GROUP_CACHE_EXPIRE).await?;

        // 更新GEO索引
        let _: () = redis::cmd("GEOADD")
            .arg(GROUP_GEO_KEY)
            .arg(group.longitude)
            .arg(group.latitude)
            .arg(&group.group_id)
            .query_async(&mut conn)
            .await?;

        // 设置GEO索引的过期时间
        let _: () = conn
            .expire(GROUP_GEO_KEY, GROUP_CACHE_EXPIRE as i64)
            .await?;

        Ok(())
    }

    /// 获取缓存的群组信息
    pub async fn get_cached_group(
        &self,
        group_id: &str,
    ) -> Result<Option<Group>, redis::RedisError> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        // 群组ID缓存键
        let key = group_id_key(group_id);
        let result: Option<String> = conn.get(&key).await?;

        match result {
            Some(json) => {
                // 反序列化
                let cached_group: CachedGroup = serde_json::from_str(&json).map_err(|e| {
                    redis::RedisError::from((
                        redis::ErrorKind::IoError,
                        "反序列化错误",
                        e.to_string(),
                    ))
                })?;

                // 重置过期时间
                let _: () = conn.expire(&key, GROUP_CACHE_EXPIRE as i64).await?;

                // 转换为Group并返回
                Ok(Some(Group::from(cached_group)))
            }
            None => Ok(None),
        }
    }

    /// 查找附近的群组
    pub async fn find_nearby_groups(
        &self,
        latitude: f64,
        longitude: f64,
        radius: f64,
    ) -> Result<Vec<Group>, redis::RedisError> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let mut cached_groups = Vec::new();

        // 延长GEO索引的过期时间
        let _: Result<(), redis::RedisError> =
            conn.expire(GROUP_GEO_KEY, GROUP_CACHE_EXPIRE as i64).await;

        // 使用GEORADIUS查询给定坐标半径内的群组ID
        let geo_results: Vec<(String, (f64, f64, f64))> = redis::cmd("GEORADIUS")
            .arg(GROUP_GEO_KEY)
            .arg(longitude) // Redis的GEORADIUS命令参数顺序是 longitude, latitude
            .arg(latitude)
            .arg(radius)
            .arg("m") // 单位：米
            .arg("WITHDIST") // 返回距离
            .arg("WITHCOORD") // 返回坐标
            .query_async(&mut conn)
            .await?;

        if geo_results.is_empty() {
            return Ok(Vec::new());
        }

        // 遍历GEO结果，获取群组详情
        for (group_id, (lon, lat, _distance)) in geo_results {
            // 获取群组详情缓存
            let group_key = group_id_key(&group_id);
            let group_json: Option<String> = conn.get(&group_key).await?;

            if let Some(json) = group_json {
                if let Ok(cached_group) = serde_json::from_str::<CachedGroup>(&json) {
                    // 延长缓存过期时间
                    let _: () = conn.expire(&group_key, GROUP_CACHE_EXPIRE as i64).await?;

                    // 转换为Group并添加到结果集
                    let mut group = Group::from(cached_group);

                    // 如果坐标有变化，更新为GEO索引中的精确坐标
                    if (group.latitude - lat).abs() > 0.0001
                        || (group.longitude - lon).abs() > 0.0001
                    {
                        group.latitude = lat;
                        group.longitude = lon;
                    }

                    cached_groups.push(group);
                }
            }
        }

        Ok(cached_groups)
    }

    /// 更新群组位置
    pub async fn update_group_location(
        &self,
        group_id: &str,
        latitude: f64,
        longitude: f64,
    ) -> Result<(), redis::RedisError> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        // 1. 从GEO索引中删除旧位置
        let _: () = redis::cmd("ZREM")
            .arg(GROUP_GEO_KEY)
            .arg(group_id)
            .query_async(&mut conn)
            .await?;

        // 2. 添加新位置到GEO索引
        let _: () = redis::cmd("GEOADD")
            .arg(GROUP_GEO_KEY)
            .arg(longitude)
            .arg(latitude)
            .arg(group_id)
            .query_async(&mut conn)
            .await?;

        // 3. 更新群组缓存中的位置信息
        let group_key = group_id_key(group_id);
        let group_json: Option<String> = conn.get(&group_key).await?;

        if let Some(json) = group_json {
            if let Ok(mut cached_group) = serde_json::from_str::<CachedGroup>(&json) {
                // 更新坐标
                cached_group.latitude = latitude;
                cached_group.longitude = longitude;

                // 保存回Redis
                if let Ok(updated_json) = serde_json::to_string(&cached_group) {
                    let _: () = conn
                        .set_ex(group_key, updated_json, GROUP_CACHE_EXPIRE)
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// 清除群组缓存
    pub async fn clear_group_cache(&self, group_id: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        // 1. 从GEO索引中删除
        let _: () = redis::cmd("ZREM")
            .arg(GROUP_GEO_KEY)
            .arg(group_id)
            .query_async(&mut conn)
            .await?;

        // 2. 删除群组详情缓存
        let group_key = group_id_key(group_id);
        let _: () = conn.del(group_key).await?;

        Ok(())
    }
}
