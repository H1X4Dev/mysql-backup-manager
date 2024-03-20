use std::env;
use std::path::Path;
use std::sync::{Arc, Mutex};
use log::{error, info};
use sqlx::Sqlite;
use sqlx::sqlite::SqlitePoolOptions;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use crate::config::*;
use crate::service::mysql::config::MySQLConnectionConfig;
use crate::service::mysql::mysql_service::MySQLService;
use crate::service::service::{ServiceScheduler, Service};
use tokio::signal::ctrl_c;
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

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

    // Now we simply iterate all services and start handling them.
    let mut sched = match JobScheduler::new().await {
        Ok(scheduler) => scheduler,
        Err(error) => {
            error!("An error occurred while creating scheduler: {}", error);
            return Err(-1)
        }
    };
    sched.set_shutdown_handler(Box::new(|| {
        Box::pin(async move {
            info!("Shutting down scheduler.")
        })
    }));

    // Schedule the service.
    let mut services: Vec<Arc<dyn Service>> = vec![];
    for (service_name, service_config) in config.services {
        info!("Scheduling {}", service_name);

        match service_config {
            ServiceConfigEnum::MySQL(mysql_config) => {
                let mysql_service = Arc::new(MySQLService::new(mysql_config, config.backup.clone()));
                match MySQLService::schedule(mysql_service.clone(), &mut sched, &service_name).await {
                    Ok(_) => (),
                    Err(error) => {
                        error!("Failed to schedule MySQL task. Error: {}", error);
                        return Err(-1)
                    }
                };
                services.push(mysql_service);
            }
        }
    }

    // Start the scheduler.
    match sched.start().await {
        Ok(_) => (),
        Err(error) => {
            error!("Failed to start the scheduler due to error: {}", error);
            return Err(-1)
        }
    };

    // Create a future for handling the Ctrl+C signal (SIGINT)
    let ctrl_c_future = ctrl_c();

    #[cfg(unix)]
    {
        // Create a signal receiver for SIGTERM (Unix only)
        let sigterm_future = signal(SignalKind::terminate())?;

        // Wait for either Ctrl+C or SIGTERM signal
        tokio::select! {
            _ = ctrl_c_future => {
                info!("Received Ctrl+C signal, shutting down gracefully...");
            }
            _ = sigterm_future.recv() => {
                info!("Received SIGTERM signal, shutting down gracefully...");
            }
        }
    }

    #[cfg(not(unix))]
    {
        // Wait for the Ctrl+C signal (Windows and other platforms)
        ctrl_c_future.await.unwrap();
        info!("Received Ctrl+C signal, shutting down gracefully...");
    }

    // Shutdown the scheduler
    match sched.shutdown().await {
        Ok(_) => info!("Scheduler has been shutdown"),
        Err(error) => {
            error!("Failed to shutdown scheduler. Error: {}", error);
            return Err(-1)
        }
    }
    Ok(())
}