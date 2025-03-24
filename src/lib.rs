use config::Config;
use sqlx::PgPool;
use std::sync::Arc;
use redis::Client as RedisClient;

pub mod config;
pub mod middleware;
pub mod utils;

pub mod routes;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub redis: Arc<RedisClient>,
}
