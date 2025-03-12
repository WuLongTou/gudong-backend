use config::Config;
use sqlx::PgPool;

pub mod config;
pub mod middleware;
pub mod utils;

pub mod routes;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
}
