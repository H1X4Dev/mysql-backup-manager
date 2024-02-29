use std::env;
use std::path::Path;
use log::{error, info};
use sqlx::Sqlite;
use sqlx::sqlite::SqlitePoolOptions;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use crate::config::*;

mod config;
mod service;

const DB_URL: &str = "sqlite://sqlite.db?mode=rwc";

#[tokio::main]
async fn main() -> Result<(), i32> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    // Fetch the current path
    let current_path = if env::var("RUST_ENV") == Ok("production".to_string()) {
        match env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                error!("Failed to fetch path. Error: {}", error);
                return Err(-1)
            }
        }
    } else {
        let create_dir = match env::var("CARGO_MANIFEST_DIR") {
            Ok(path) => path,
            Err(error) => {
                error!("Failed ot fetch manifest dir. Error: {}", error);
                return Err(-1)
            }
        };
        Path::new(&create_dir).to_path_buf()
    };

    // Read the configuration
    let config_path = current_path.clone().join("config.toml");
    let config = match Config::new(config_path.to_str().unwrap()).await {
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
    let migrations_path = current_path.clone().join("migrations");
    let result = match sqlx::migrate::Migrator::new(migrations_path).await {
        Ok(m) => m.run(&pool).await,
        Err(error) => {
            error!("An error occurred while initializing migrations: {}", error);
            return Err(-1)
        }
    };

    // See if migration was successful.
    match result {
        Ok(_) => (),
        Err(error) => {
            error!("An error occurred while migrating the database: {}", error);
            return Err(-1)
        }
    };

    info!("config: {:?}", config);
    info!("pool: {:?}", pool);

    // Now we will need a way to iterate all used services and schedule their job.

    Ok(())
}
