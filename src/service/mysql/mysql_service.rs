use std::env::temp_dir;
use std::fmt::format;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::str::FromStr;
use std::sync::{Arc};
use log::{error, info};
use tokio_cron_scheduler::{Job, JobScheduler};
use crate::service::mysql::config::{MySQLBackupType, MySQLConnectionConfig, MySQLDumpConfig, XtraBackupConfig};
use crate::service::service::Service;
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

pub struct MySQLService {
    pub backup_config: BackupConfig,
    pub config: MySQLConnectionConfig,
    pub running: Arc<Mutex<bool>>
}

impl MySQLService {
    pub fn new(config: MySQLConnectionConfig, backup_config: BackupConfig) -> MySQLService {
        return MySQLService {
            backup_config,
            config,
            running: Arc::new(Mutex::new(false))
        };
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

    fn read_defaults_file(&self, defaults_file: &NamedTempFile) -> Result<MySqlConnectOptions, Box<dyn std::error::Error>> {
        let mut conf = Ini::load_from_file(defaults_file.path())?;
        let mut options = MySqlConnectOptions::new();
        let section = conf.section(Some("client")).unwrap();

        if let Some(host) = section.get("host") {
            options = options.host(host);
        }
        if let Some(port) = section.get("port") {
            let port: u16 = port.parse()?;
            options = options.port(port);
        }
        if let Some(user) = section.get("user") {
            options = options.username(user);
        }
        if let Some(password) = section.get("password") {
            options = options.password(password);
        }
        if let Some(socket) = section.get("socket") {
            options = options.socket(socket);
        }

        Ok(options)
    }

    pub async fn do_mysqldump(&self, mysql_config: &MySQLDumpConfig) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config) = &self.config.backup {
            let mut defaults = self.get_defaults_file().await?;
            let defaults_path = defaults.path();

            // Create new pool.
            let connection_config = self.read_defaults_file(&defaults)?;
            let pool = MySqlPool::connect_with(connection_config).await?;

            // Load the databases...
            let databases = if let Some(databases) = &config.databases {
                databases.clone()
            } else {
                // If databases are not provided, fetch all databases except the excluded ones
                let excluded_databases = config.databases_exclude.clone().unwrap_or_default();
                let databases = sqlx::query("SHOW DATABASES")
                    .fetch_all(&pool)
                    .await?
                    .into_iter()
                    .map(|row| row.get(0))
                    .filter(|db| !excluded_databases.contains(db))
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

    /*
    Current issues:
    1. Overlapping jobs
    2. MySQLService is being copied each time.
    */

    async fn schedule(&self, sched: &mut JobScheduler, service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(backup_config) = &self.config.backup {
            let service_name = service_name.to_string();
            let backup_timer = backup_config.timer.interval.clone();
            let config = self.config.clone();
            let backup_config = self.backup_config.clone();
            let running = self.running.clone();

            let job = Job::new_async(Schedule::from_str(&backup_timer)?, move |uuid, mut l| {
                let service_name = service_name.clone();
                let config = config.clone();
                let backup_config = backup_config.clone();
                let running = running.clone();

                Box::pin(async move {
                    {
                        let mut m = running.lock().await;
                        if *m {
                            info!("Skipping backup for MySQL service: {}", service_name);
                            return;
                        }
                        *m = true;
                    }
                    info!("Running backup for MySQL service: {}, UUID: {}", service_name, uuid);

                    let mysql_service = MySQLService::new(config, backup_config);
                    match mysql_service.update().await {
                        Ok(_) => {
                            info!("Backup completed!");
                        },
                        Err(error) => {
                            error!("Failed to run backup for MySQL service: {}, error: {}", service_name, error);
                        }
                    };
                    {
                        let mut m = running.lock().await;
                        *m = false;
                    }
                })
            })?;

            sched.add(job).await?;
        }

        Ok(())
    }
}