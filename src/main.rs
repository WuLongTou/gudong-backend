use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use backend::{
    AppState,
    config::Config,
    handlers,
    middleware::{RateLimiter, auth_middleware, log_errors, rate_limit},
};
use sqlx::Executor;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
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

    // 设置应用状态
    let state = AppState {
        pool,
        config: config.clone(),
    };

    // 设置限流器
    let rate_limiter = Arc::new(RateLimiter::new(redis_client, config));

    // 设置 CORS
    let cors = CorsLayer::permissive();

    // 将路由分为公开路由和受保护路由
    let public_routes = Router::new()
        // 用户公开路由
        .route("/users/register", post(handlers::register))
        .route("/users/temporary", post(handlers::create_temporary))
        .route("/users/login", post(handlers::login));

    let protected_routes = Router::new()
        // 需要认证的用户路由
        .route("/users/update", post(handlers::update_user))
        .route("/users/reset-password", post(handlers::reset_password))
        // 群组路由
        .route("/groups/create", post(handlers::create_group))
        .route("/groups/by-id", get(handlers::find_by_id))
        .route("/groups/by-name", get(handlers::find_by_name))
        .route("/groups/by-location", get(handlers::find_by_location))
        .route("/groups/join", post(handlers::join_group))
        .route("/groups/leave", post(handlers::leave_group))
        .route("/groups/keep-alive", post(handlers::keep_alive))
        // 消息路由
        .route("/messages/create", post(handlers::create_message))
        .route("/messages/get", post(handlers::get_messages))
        // 应用认证中间件
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new().merge(public_routes).merge(protected_routes),
        )
        .layer(cors)
        .layer(axum::middleware::from_fn(log_errors))
        // 其他公共中间件
        .layer(axum::middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit,
        ))
        .with_state(state.clone());

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
