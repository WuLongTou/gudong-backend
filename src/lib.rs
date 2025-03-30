use config::Config;
use redis::Client as RedisClient;
use sqlx::PgPool;
use std::sync::Arc;

pub mod api;
pub mod cache;
pub mod config;
pub mod database;
pub mod middleware;
pub mod utils;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub redis: Arc<RedisClient>,
}
