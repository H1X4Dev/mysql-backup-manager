use std::any::Any;
use std::env::temp_dir;
use std::fmt::format;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::str::FromStr;
use std::sync::{Arc};
use std::time::Duration;
use async_trait::async_trait;
use log::{error, info, warn};
use tokio_cron_scheduler::{Job, JobScheduler};
use crate::service::mysql::config::{MySQLBackupType, MySQLConnectionConfig, MySQLDumpConfig, XtraBackupConfig};
use crate::service::service::{ServiceScheduler, Service};
use cron::Schedule;
use tempfile::{NamedTempFile, tempfile};
use tokio::fs;
use filepath::FilePath;
use ini::Ini;
use sqlx::{MySqlPool, Row};
use sqlx::mysql::MySqlConnectOptions;
use tokio::process::Command;
use tokio::sync::Mutex;
use which::which;
use crate::config::BackupConfig;
use crate::service::mysql::mysql_defaults::MySqlDefaultsReader;

pub struct MySQLService {
    pub backup_config: BackupConfig,
    pub config: MySQLConnectionConfig,
    pub running: Arc<Mutex<bool>>,
}

impl MySQLService {
    pub fn new(config: MySQLConnectionConfig, backup_config: BackupConfig) -> MySQLService {
        return MySQLService {
            backup_config,
            config,
            running: Arc::new(Mutex::new(false)),
        };
    }

    pub async fn try_set_running(&self) -> bool {
        let mut running = self.running.lock().await;
        if *running {
            false
        } else {
            *running = true;
            true
        }
    }

    pub async fn set_running(&self, value: bool) {
        let mut running = self.running.lock().await;
        *running = value;
    }

    pub fn create_command(&self, defaults_path: &Path, file_path: PathBuf) -> Result<Command, Box<dyn std::error::Error>> {
        let command_path = which("mysqldump")?;
        let mut cmd = Command::new(command_path);
        cmd.arg(format!("--defaults-file={}", defaults_path.clone().to_str().unwrap()));
        cmd.arg("--quick");
        cmd.arg("--single-transaction");
        cmd.arg(format!("--result-file={}", file_path.to_str().unwrap()));
        Ok(cmd)
    }

    async fn get_defaults_file(&self) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
        let mut file = NamedTempFile::new()?;
        // If defaults file already exists, we will create a copy of it and return.
        if let Some(defaults_file) = &self.config.defaults_file {
            let data = Ini::load_from_file(defaults_file)?;
            data.write_to_file(file.path())?;
        } else {
            // Otherwise, we will simply create a new defaults file to use for ourselves.
            let mut conf = Ini::new();
            conf.with_section(Some("client"))
                .set("host", self.config.host.clone().unwrap_or("localhost".to_string()));
            if let Some(port) = &self.config.port {
                conf.with_section(Some("client"))
                    .set("port", format!("{}", port));
            }
            conf.with_section(Some("client"))
                .set("user", self.config.username.clone().unwrap_or("root".to_string()));
            conf.with_section(Some("client"))
                .set("password", self.config.password.clone().unwrap_or("".to_string()));
            if let Some(socket) = &self.config.socket {
                conf.with_section(Some("client"))
                    .set("socket", socket);
            }
            conf.write_to_file(file.path())?;
        }
        Ok(file)
    }

    pub async fn do_mysqldump(&self, mysql_config: &MySQLDumpConfig) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config) = &self.config.backup {
            let mut defaults = self.get_defaults_file().await?;
            let defaults_path = defaults.path();

            // Create new pool.
            let connection_config = MySqlConnectOptions::from_defaults_file(defaults_path)?;
            let pool = MySqlPool::connect_lazy_with(connection_config);

            // Fetch the list of databases.
            let databases = if let Some(databases) = &config.databases {
                databases.clone()
            } else {
                // If databases are not provided, fetch all databases except the excluded ones
                let excluded_databases = config.databases_exclude.clone().unwrap_or_default();
                let excluded_default_databases = vec!["information_schema".to_string(), "mysql".to_string(), "performance_schema".to_string(), "sys".to_string()];
                let databases = sqlx::query("SHOW DATABASES")
                    .fetch_all(&pool)
                    .await?
                    .into_iter()
                    .map(|row| row.get(0))
                    .filter(|db| !excluded_databases.contains(db) && !excluded_default_databases.contains(db))
                    .collect::<Vec<String>>();
                databases
            };

            // Iterate each database and dump it individually.
            for database in &databases {
                if mysql_config.separate_tables.is_some() && mysql_config.separate_tables.unwrap() {
                    // Fetch the table names for the database
                    sqlx::query(&format!("USE {}", database)).execute(&pool).await?;
                    let tables = sqlx::query("SHOW TABLES").fetch_all(&pool).await?;
                    let temp_dir = PathBuf::from_str(&self.backup_config.basedir)?.join(database);

                    for table in tables {
                        let table_name: String = table.get(0);
                        info!("Dumping table: {}.{}", database, table_name);

                        // Create a result path, where the SQL will be dumped off to.
                        let result_path = temp_dir.clone().join(format!("{}.{}.sql", database, table_name));

                        // Create the command to dump the data.
                        let mut cmd = self.create_command(defaults_path, result_path)?;
                        cmd.arg(database);
                        cmd.arg(table_name);

                        // Run the command and expect output.
                        let status = cmd.stdout(Stdio::null()).status().await?;
                        if status.success() {
                            info!("-> Dumped!");
                        } else {
                            info!("-> Failed to dump!");
                        }
                    }
                } else {
                    info!("Dumping database: {}", database);

                    // Create a result path, where the SQL will be dumped off to.
                    let result_path = PathBuf::from_str(&self.backup_config.basedir)?.join(format!("{}.sql", database));

                    // Create the command to dump the data.
                    let mut cmd = self.create_command(defaults_path, result_path)?;
                    cmd.arg(database);

                    // Run the command and expect output.
                    let status = cmd.stdout(Stdio::null()).status().await?;
                    if status.success() {
                        info!("-> Dumped!");
                    } else {
                        info!("-> Failed to dump!");
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn do_xtrabackup(&self, config: &XtraBackupConfig) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[async_trait]
impl Service for MySQLService {
    async fn update(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(backup_config) = &self.config.backup {
            match &backup_config.backup_type {
                MySQLBackupType::xtrabackup(config) => self.do_xtrabackup(config).await?,
                MySQLBackupType::mysqldump(config) => self.do_mysqldump(config).await?
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ServiceScheduler for MySQLService {

    /*
    Current issues:
    1. Overlapping jobs
    2. MySQLService is being copied each time.
    */

    async fn schedule<T: Service + Any>(service: Arc<T>, sched: &mut JobScheduler, service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let service_clone = service.clone();
        match Arc::downcast::<MySQLService>(service_clone) {
            Ok(mysql_service) => {
                if let Some(backup_config) = &mysql_service.config.backup {
                    let service_name = service_name.to_string();
                    let backup_timer = backup_config.timer.interval.clone();

                    let job = Job::new_async(Schedule::from_str(&backup_timer)?, move |uuid, mut l| {
                        let service_name = service_name.clone();
                        let self_clone = mysql_service.clone();

                        Box::pin(async move {
                            if !self_clone.try_set_running().await {
                                warn!("MySQL backup already running.");
                                return;
                            }

                            info!("Running backup for MySQL service: {}, UUID: {}", service_name, uuid);

                            match self_clone.update().await {
                                Ok(_) => {
                                    info!("Backup completed!");
                                }
                                Err(error) => {
                                    error!("Failed to run backup for MySQL service: {}, error: {}", service_name, error);
                                }
                            };

                            self_clone.set_running(false).await;
                        })
                    })?;

                    sched.add(job).await?;
                }
            }
            Err(_) => {}
        }
        Ok(())
    }
}