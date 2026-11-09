mod auth;
mod bootstrap;
mod config;
mod error;
mod models;
mod routes;

use std::{sync::Arc, time::Duration};

use sqlx::mysql::MySqlPoolOptions;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::MySqlPool,
    pub config: Arc<AppConfig>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mori_shop=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env();
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    bootstrap::bootstrap(
        &pool,
        &config.admin_username,
        &config.admin_password,
        config.seed_demo_data,
    )
    .await?;

    let state = AppState {
        db: pool,
        config: Arc::new(config.clone()),
    };
    let app = routes::router(state);
    let listener = TcpListener::bind(&config.bind_address).await?;

    tracing::info!(
        address = %config.bind_address,
        "MORI 商城已启动，浏览器访问 http://localhost:5050"
    );
    axum::serve(listener, app).await?;
    Ok(())
}
