use std::path::Path;
use log::{error, info};
use sqlx::Sqlite;
use sqlx::sqlite::SqlitePoolOptions;
use crate::config::*;

mod config;
mod database;
mod service;

const DB_URL: &str = "sqlite://sqlite.db?mode=rwc";

#[tokio::main]
async fn main() -> Result<(), i32> {
    env_logger::init();

    // Read the configuration
    let config = match Config::new("config.toml").await {
        Ok(config) => config,
        Err(error) => {
            error!("An error occurred while parsing config: {}", error);
            return Err(-1)
        }
    };

    // Create a new pool
    let pool = match SqlitePoolOptions::new().max_connections(15).connect(DB_URL).await {
        Ok(pool) => pool,
        Err(error) => {
            error!("An error occurred while connecting to the database: {}", error);
            return Err(-1)
        }
    };

    // Migrate the database
    let result = match sqlx::migrate::Migrator::new(Path::new("./migrations")).await {
        Ok(m) => m.run(&pool).await,
        Err(error) => {
            error!("An error occurred while initializing migrations: {}", error);
            return Err(-1)
        }
    };

    match result {
        Ok(_) => (),
        Err(error) => {
            error!("An error occurred while migrating the database: {}", error);
            return Err(-1)
        }
    };

    info!("config: {:?}", config);
    info!("pool: {:?}", pool);

    Ok(())
}
