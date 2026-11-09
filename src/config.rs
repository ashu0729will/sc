use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub bind_address: String,
    pub admin_username: String,
    pub admin_password: String,
    pub session_ttl_hours: i64,
    pub secure_cookie: bool,
    pub seed_demo_data: bool,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT").unwrap_or_else(|_| "5050".to_string());

        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "mysql://mori:mori_password@127.0.0.1:3306/mori_shop".to_string()
            }),
            bind_address: format!("{host}:{port}"),
            admin_username: env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string()),
            admin_password: env::var("ADMIN_PASSWORD")
                .unwrap_or_else(|_| "Admin@123456".to_string()),
            session_ttl_hours: env::var("SESSION_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(168),
            secure_cookie: env::var("SESSION_COOKIE_SECURE")
                .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
                .unwrap_or(false),
            seed_demo_data: env::var("SEED_DEMO_DATA")
                .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
                .unwrap_or(true),
        }
    }
}
