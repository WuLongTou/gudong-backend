use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::AsyncCommands;

use crate::{
    config::Config,
    utils::{error_codes, error_to_api_response},
};

#[derive(Clone)]
pub struct RateLimiter {
    redis: Arc<redis::Client>,
    config: Arc<Config>,
}

use axum::extract::ConnectInfo; // 新增导入
use std::net::SocketAddr;

impl RateLimiter {
    pub fn new(redis: redis::Client, config: Config) -> Self {
        Self {
            redis: Arc::new(redis),
            config: Arc::new(config),
        }
    }

    pub async fn check_rate_limit(
        self: Arc<Self>,
        req: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        // 从连接信息获取原始IP
        let remote_ip = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip().to_string());
        tracing::info!("remote_ip: {:?}", remote_ip);
        // 从请求头中获取IP，或者使用连接信息中的IP作为默认值
        let ip = req
            .headers()
            .get("x-real-ip")
            .and_then(|h| h.to_str().ok())
            .or_else(|| {
                req.headers()
                    .get("x-forwarded-for")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.split(',').find(|ip| !ip.trim().is_empty()))
            })
            .or_else(|| remote_ip.as_deref()) // 降级使用连接IP
            .unwrap_or("unknown")
            .trim()
            .to_string();
        tracing::info!("ip: {:?}", ip);

        let key = format!("rate_limit:{}", ip);
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // 使用 Redis 的 INCR 和 EXPIRE 命令实现计数器
        let count: i32 = conn
            .incr(&key, 1)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if count == 1 {
            // 如果是第一次请求，设置过期时间
            let _: () = conn
                .expire(&key, self.config.rate_limit_window().as_secs() as i64)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }

        if count > self.config.rate_limit_requests as i32 {
            return Ok((
                StatusCode::OK,
                error_to_api_response::<()>(
                    error_codes::RATE_LIMIT,
                    format!(
                        "请求过于频繁，请在{}秒后重试",
                        self.config.rate_limit_window().as_secs()
                    ),
                ),
            )
                .into_response());
        }

        Ok(next.run(req).await)
    }
}

pub async fn rate_limit(
    State(limiter): State<Arc<RateLimiter>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    limiter.check_rate_limit(req, next).await
}
