use config::Config;
use sqlx::PgPool;

pub mod config;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod utils;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
}
