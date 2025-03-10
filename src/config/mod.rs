use std::env;
use std::time::Duration;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_secs: u64,
    pub temp_token_expiration_secs: u64,
    pub rate_limit_window_secs: u64,
    pub rate_limit_requests: u32,
    pub server_host: String,
    pub server_port: u16,
    pub max_search_radius: f64,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        dotenv::dotenv().ok();

        let jwt_expiration = env::var("JWT_EXPIRATION")?
            .trim_end_matches('h')
            .parse::<u64>()
            .unwrap_or(24);
        let temp_token_expiration = env::var("TEMP_TOKEN_EXPIRATION")?
            .trim_end_matches('h')
            .parse::<u64>()
            .unwrap_or(1);
        Ok(Config {
            database_url: env::var("DATABASE_URL")?,
            redis_url: env::var("REDIS_URL")?,
            server_host: env::var("SERVER_HOST")?,
            server_port: env::var("SERVER_PORT")?.parse().unwrap_or(3000),
            jwt_secret: env::var("JWT_SECRET")?,
            jwt_expiration_secs: jwt_expiration * 3600,
            temp_token_expiration_secs: temp_token_expiration * 3600,
            rate_limit_window_secs: env::var("RATE_LIMIT_WINDOW")?.parse().unwrap_or(60),
            rate_limit_requests: env::var("RATE_LIMIT_REQUESTS")?.parse().unwrap_or(100),
            max_search_radius: env::var("MAX_SEARCH_RADIUS")?.parse().unwrap_or(5000.0),
        })
    }

    pub fn jwt_expiration(&self) -> Duration {
        Duration::from_secs(self.jwt_expiration_secs)
    }

    pub fn temp_token_expiration(&self) -> Duration {
        Duration::from_secs(self.temp_token_expiration_secs)
    }

    pub fn rate_limit_window(&self) -> Duration {
        Duration::from_secs(self.rate_limit_window_secs)
    }
}
