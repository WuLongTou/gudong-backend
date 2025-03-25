use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post, put},
};
use backend::{
    AppState,
    config::Config,
    middleware::{RateLimiter, auth_middleware, log_errors, rate_limit},
    routes,
};
use sqlx::Executor;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置
    let config = Config::from_env().expect("Failed to load configuration");

    #[cfg(debug_assertions)]
    tracing::info!("Running in debug mode with CORS enabled");

    #[cfg(not(debug_assertions))]
    tracing::info!("Running in production mode with CORS disabled");

    // 设置数据库连接池
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                conn.execute("SET application_name = 'geotrack_backend';")
                    .await?;
                Ok(())
            })
        })
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to Postgres");

    // 设置 Redis 客户端
    let redis_client =
        redis::Client::open(config.redis_url.clone()).expect("Failed to create Redis client");
    let redis_arc = Arc::new(redis_client.clone());

    // 设置应用状态
    let state = AppState {
        pool,
        config: config.clone(),
        redis: redis_arc,
    };

    // 设置限流器
    let rate_limiter = Arc::new(RateLimiter::new(redis_client, config.clone()));

    // 将路由分为公开路由和受保护路由
    let public_routes = Router::new()
        // 用户公开路由
        .route("/users/register", post(routes::user::register))
        .route("/users/temporary", post(routes::user::create_temporary))
        .route("/users/login", post(routes::user::login));

    let protected_routes = Router::new()
        // 需要认证的用户路由
        .route("/users/update-nickname", put(routes::user::update_nickname))
        .route("/users/update-password", put(routes::user::update_password))
        .route("/users/reset-password", post(routes::user::reset_password))
        .route("/users/refresh-token", post(routes::user::refresh_token))
        .route("/users/check-token", get(routes::user::check_token))
        // 群组路由
        .route("/groups/create", post(routes::group::create_group))
        .route("/groups/by-id", get(routes::group::find_by_id))
        .route("/groups/by-name", get(routes::group::find_by_name))
        .route("/groups/by-location", get(routes::group::find_by_location))
        .route("/groups/join", post(routes::group::join_group))
        .route("/groups/leave", post(routes::group::leave_group))
        .route("/groups/keep-alive", post(routes::group::keep_alive))
        // 消息路由
        .route("/messages/create", post(routes::message::create_message))
        .route("/messages/get", post(routes::message::get_messages))
        // 应用认证中间件
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // 创建基础路由
    let router = Router::new().nest(
        &config.api_base_uri.clone(),
        Router::new().merge(public_routes).merge(protected_routes),
    );

    // 添加日志中间件和限流中间件
    let router = router.layer(axum::middleware::from_fn(log_errors)).layer(
        axum::middleware::from_fn_with_state(rate_limiter, rate_limit),
    );

    // 根据编译模式决定是否添加CORS
    #[cfg(debug_assertions)]
    let router = {
        tracing::debug!("Adding CORS layer for development mode");
        // 设置开发环境的CORS，允许所有来源
        let cors = CorsLayer::permissive();
        router.layer(cors)
    };

    // 添加应用状态
    let app = router.with_state(state.clone());

    // 启动服务器
    let addr = SocketAddr::new(
        state.config.server_host.parse().unwrap_or_else(|_| {
            tracing::warn!("Invalid server_host, falling back to dual-stack default");
            IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED)
        }),
        state.config.server_port,
    );
    tracing::info!("Server listening on {}", addr);
    axum::serve(
        tokio::net::TcpListener::bind(&addr)
            .await
            .expect("Failed to bind"),
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Failed to start server");
}
