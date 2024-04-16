use std::any::Any;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc};
use std::time::Duration;
use async_trait::async_trait;
use log::{error, info, warn};
use tokio_cron_scheduler::{Job, JobScheduler};
use crate::service::mysql::config::{MySQLBackupType, MySQLConnectionConfig};
use crate::service::service::{ServiceScheduler, Service};
use cron::Schedule;
use tempfile::NamedTempFile;
use ini::Ini;
use sqlx::types::chrono::Utc;
use tokio::fs;
use tokio::sync::Mutex;
use crate::config::BackupConfig;
use crate::DB_POOL;
use crate::service::mysql::database::MysqlBackupRow;
use crate::service::mysql::mysqldump::MySqlDumpRunner;
use crate::service::mysql::xtrabackup::XtraBackupRunner;

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

    pub async fn get_defaults_file(&self) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
        let file = NamedTempFile::new()?;
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
}

#[async_trait]
impl Service for MySQLService {
    async fn update(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(backup_config) = &self.config.backup {
            // If keep last was specified, then we have to clean up old backups from the base directory.
            if let Some(keep_last) = self.backup_config.keep_last {
                let pool = DB_POOL.get().unwrap();
                let interval = Utc::now() - Duration::from_secs(keep_last * 24 * 60 * 60);
                let older_than_interval: Vec<MysqlBackupRow> = sqlx::query_as("SELECT * FROM backups WHERE created_at < $1")
                    .bind(interval)
                    .fetch_all(pool)
                    .await?;

                // Now iterate everything and nuke.
                for backup in older_than_interval {
                    let str_path = backup.path.clone();
                    let path = PathBuf::from_str(&str_path).unwrap();
                    if path.is_file() {
                        fs::remove_file(path).await?;
                    } else {
                        fs::remove_dir_all(path).await?;
                    }
                }
            }

            // Otherwise we simply do the task.
            match &backup_config.backup_type {
                MySQLBackupType::XtraBackup(config) => self.do_xtrabackup(config).await?,
                MySQLBackupType::MySqlDump(config) => self.do_mysqldump(config).await?
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ServiceScheduler for MySQLService {
    async fn schedule<T: Service + Any>(service: Arc<T>, sched: &mut JobScheduler, service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let service_clone = service.clone();
        match Arc::downcast::<MySQLService>(service_clone) {
            Ok(mysql_service) => {
                if let Some(backup_config) = &mysql_service.config.backup {
                    let service_name = service_name.to_string();
                    let backup_timer = backup_config.interval.clone();

                    let job = Job::new_async(Schedule::from_str(&backup_timer)?, move |uuid, _| {
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