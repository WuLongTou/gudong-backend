use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, patch, post, put},
};
use backend::{
    AppState, api,
    config::Config,
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

    // 认证相关路由（公开）
    let auth_routes = Router::new()
        .route("/register", post(api::operations::user::register))
        .route(
            "/register/temporary",
            post(api::operations::user::create_temporary),
        )
        .route("/login", post(api::operations::user::login))
        .route("/token/refresh", get(api::operations::user::refresh_token))
        .route("/token/verify", post(api::operations::user::check_token));

    // 用户相关路由（需要认证）
    let user_routes = Router::new()
        .route("/profile", put(api::operations::user::update_nickname))
        .route(
            "/profile/password",
            patch(api::operations::user::update_password),
        )
        .route(
            "/profile/password/reset",
            post(api::operations::user::reset_password),
        )
        .route(
            "/location/nearby",
            get(api::operations::activity::find_nearby_users),
        )
        .route(
            "/activities",
            get(api::operations::activity::find_user_activities),
        )
        .route(
            "/{user_id}/activities",
            get(api::operations::activity::find_user_activities),
        );

    // 群组相关路由（需要认证）
    let group_routes = Router::new()
        .route("/", post(api::operations::group::create_group))
        .route(
            "/search/name/{name}",
            get(api::operations::group::search_groups_by_name),
        )
        .route(
            "/search/location",
            post(api::operations::group::search_groups_by_location),
        )
        .route("/my", get(api::operations::group::get_user_groups))
        .route("/{group_id}", get(api::operations::group::get_group_info))
        .route(
            "/{group_id}/heartbeat",
            put(api::operations::group::keep_alive),
        )
        .route(
            "/{group_id}/activities",
            get(api::operations::activity::get_group_activities),
        )
        .route(
            "/{group_id}/members",
            get(api::operations::group::get_group_members),
        )
        .route(
            "/{group_id}/members",
            post(api::operations::group::join_group),
        )
        .route(
            "/{group_id}/members/my",
            delete(api::operations::group::leave_group),
        )
        .route(
            "/{group_id}/members/{user_id}",
            delete(api::operations::group::remove_group_member),
        )
        .route(
            "/{group_id}/members/{user_id}/role",
            put(api::operations::group::update_user_role),
        );

    // 消息相关路由（需要认证）
    let message_routes = Router::new()
        .route("/", post(api::operations::message::send_message))
        .route(
            "/groups/{group_id}",
            get(api::operations::message::get_message_history),
        )
        .route(
            "/{message_id}",
            delete(api::operations::message::delete_message),
        );

    // 活动相关路由（需要认证）
    let activity_routes = Router::new()
        .route("/", post(api::operations::activity::create_user_activity))
        .route("/", get(api::operations::activity::get_all_activities))
        .route(
            "/nearby",
            get(api::operations::activity::get_nearby_activities),
        );

    // 系统健康检查路由（公开）
    let health_routes = Router::new().route("/ping", get(api::operations::test::ping));

    // 将需要认证的路由组织到一起并应用认证中间件
    let authenticated_routes = Router::new()
        .nest("/users", user_routes)
        .nest("/groups", group_routes)
        .nest("/messages", message_routes)
        .nest("/activities", activity_routes)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // 将公开路由组织到一起
    let public_routes = Router::new()
        .nest("/auth", auth_routes)
        .nest("/health", health_routes);

    // 合并所有路由
    let api_routes = Router::new()
        .merge(public_routes)
        .merge(authenticated_routes);

    // 创建基础路由
    let router = Router::new().nest(&config.api_base_uri.clone(), api_routes);

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
