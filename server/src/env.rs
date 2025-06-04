use dotenvy::dotenv;
use serde::Deserialize;
use std::env;
use once_cell::sync::Lazy;

#[derive(Deserialize)]
pub struct AppConfig {
    pub app_host: String,
    pub app_port: u16,
    pub worker_count: usize,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenv().ok();

        AppConfig {
            app_host: env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            app_port: env::var("APP_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("Invalid APP_PORT"),
            worker_count: env::var("WORKER_COUNT")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .expect("Invalid WORKER_COUNT"),
        }
    }
}

pub static APP_URL: Lazy<String> = Lazy::new(|| {
    env::var("APP_URL").expect("APP_URL must be set")
});

pub static MONGODB_URI: Lazy<String> = Lazy::new(|| {
    env::var("MONGODB_URI").expect("MONGODB_URI must be set")
});

pub static REDIS_URI: Lazy<String> = Lazy::new(|| {
    env::var("REDIS_URI").unwrap_or("redis://localhost:6379".to_string())
});

pub static JWT_SECRET: Lazy<String> = Lazy::new(|| {
    env::var("JWT_SECRET").expect("JWT_SECRET must be set")
});

pub static GOOGLE_CLIENT_ID: Lazy<String> = Lazy::new(|| {
    env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID must be set")
});

pub static GOOGLE_CLIENT_SECRET: Lazy<String> = Lazy::new(|| {
    env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET must be set")
});

// key: TELEGRAM_ONLY
// pub static BOT_TOKEN: Lazy<String> = Lazy::new(|| {
//     env::var("BOT_TOKEN").expect("BOT_TOKEN must be set")
// });