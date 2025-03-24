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
    pub api_base_uri: String,
    pub max_search_radius: f64,
}

/// 解析带单位的时间字符串为秒数
/// 支持的单位: s(秒), m(分), h(小时), d(天)
/// 例如: "30s", "5m", "2h", "1d"
fn parse_time_to_seconds(time_str: &str) -> Result<u64, String> {
    // 匹配最后一个字符作为单位，其前面的都是数值
    let (value_str, unit) = match time_str.chars().last() {
        Some(c) => {
            if c.is_digit(10) {
                // 没有单位，直接作为秒处理
                (time_str, 's')
            } else {
                // 有单位，分割获取
                (&time_str[0..time_str.len() - 1], c)
            }
        }
        None => return Err("时间字符串为空".to_string()),
    };

    // 解析数值部分
    let value = value_str
        .parse::<u64>()
        .map_err(|e| format!("无法解析时间值: {}", e))?;

    // 根据单位转换为秒
    match unit {
        's' | 'S' => Ok(value),         // 秒
        'm' | 'M' => Ok(value * 60),    // 分钟
        'h' | 'H' => Ok(value * 3600),  // 小时
        'd' | 'D' => Ok(value * 86400), // 天
        _ => Err(format!("不支持的时间单位: {}", unit)),
    }
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        dotenv::dotenv().ok();

        // 解析JWT过期时间
        let jwt_expiration_secs = match env::var("JWT_EXPIRATION") {
            Ok(val) => parse_time_to_seconds(&val).unwrap_or(24 * 3600), // 默认24小时
            Err(_) => 24 * 3600,
        };

        // 解析临时令牌过期时间
        let temp_token_expiration_secs = match env::var("TEMP_TOKEN_EXPIRATION") {
            Ok(val) => parse_time_to_seconds(&val).unwrap_or(1 * 3600), // 默认1小时
            Err(_) => 1 * 3600,
        };

        // 解析速率限制窗口时间
        let rate_limit_window_secs = match env::var("RATE_LIMIT_WINDOW") {
            Ok(val) => parse_time_to_seconds(&val).unwrap_or(60), // 默认60秒
            Err(_) => 60,
        };

        Ok(Config {
            database_url: env::var("DATABASE_URL")?,
            redis_url: env::var("REDIS_URL")?,
            server_host: env::var("SERVER_HOST")?,
            server_port: env::var("SERVER_PORT")?.parse().unwrap_or(3000),
            api_base_uri: env::var("API_BASE_URI")?,
            jwt_secret: env::var("JWT_SECRET")?,
            jwt_expiration_secs,
            temp_token_expiration_secs,
            rate_limit_window_secs,
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
