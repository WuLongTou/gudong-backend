use axum::Router;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

mod error;
mod group;
mod infrastructure;
mod result;
mod user;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    // 初始化Redis连接池
    let redis_manager = RedisConnectionManager::new("redis://127.0.0.1/").unwrap();
    let redis_pool = Pool::builder().build(redis_manager).await.unwrap();

    let cors = CorsLayer::permissive();
    let app = Router::new()
        .nest("/user", user::router())
        .nest("/group", group::router())
        .layer(cors)
        .with_state(redis_pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Server is running on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
