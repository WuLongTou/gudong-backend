use axum::{routing::post, Router, routing::get};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;

mod handlers;
mod types;

pub fn router() -> Router<Pool<RedisConnectionManager>> {
    Router::new()
        .route("/", get(|| async { "user endpoint" }))
        .route("/register", post(handlers::register_user))
}
