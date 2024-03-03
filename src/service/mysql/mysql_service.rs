use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use log::{error, info};
use tokio_cron_scheduler::{Job, JobScheduler};
use crate::service::mysql::config::{MySQLBackupType, MySQLConnectionConfig};
use crate::service::service::Service;
use cron::Schedule;
use tempfile::tempfile;
use tokio::fs;

pub struct MySQLService {
    pub config: MySQLConnectionConfig
}

impl MySQLService {
    pub fn new(config: MySQLConnectionConfig) -> MySQLService {
        return MySQLService {
            config
        }
    }

    async fn get_defaults_file(&self) -> Result<File, Box<dyn std::error::Error>> {
        let mut file = tempfile()?;
        // If defaults file already exists, we will create a copy of it and return.
        if let Some(defaults_file) = &self.config.defaults_file {
            let data = fs::read_to_string(defaults_file).await?;
            write!(file, "{}", data)?;
        } else {
            // Otherwise, we will simply create a new defaults file to use for ourselves.
            writeln!(file, "[client]")?;
            writeln!(file, "host = {}", self.config.host.clone().unwrap_or("localhost".to_string()))?;
            if let Some(port) = &self.config.port {
                writeln!(file, "port = {}", port)?;
            }
            writeln!(file, "user = {}", self.config.username.clone().unwrap_or("root".to_string()))?;
            writeln!(file, "password = {}", self.config.password.clone().unwrap_or("".to_string()))?;
            if let Some(socket) = &self.config.socket {
                writeln!(file, "socket = {}", socket)?;
            }
        }
        Ok(file)
    }

    pub async fn do_mysqldump(&self) -> Result<(), Box<dyn std::error::Error>> {
        /**
         * mysqldump's are pretty simple, here we will just call mysqldump library
         * and then dump it how we need to do it, by the way, we also have to create
         * defaults file.
         */
        let mut defaults = self.get_defaults_file()?;

        // do shit

        Ok(())
    }

    pub async fn do_xtrabackup(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl Service for MySQLService {
    async fn update(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(backup_config) = &self.config.backup {
            match backup_config.backup_type {
                MySQLBackupType::xtrabackup(_) => self.do_xtrabackup().await?,
                MySQLBackupType::mysqldump(_) => self.do_mysqldump().await?
            }
        }
        Ok(())
    }

    async fn schedule(&self, sched: &mut JobScheduler, service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(backup_config) = &self.config.backup {
            let service_name = service_name.to_string();
            let backup_timer = backup_config.timer.interval.clone();
            let config = self.config.clone();

            let job = Job::new_async(Schedule::from_str(&backup_timer)?, move |uuid, mut l| {
                let service_name = service_name.clone();
                let config = config.clone();

                Box::pin(async move {
                    info!("Running backup for MySQL service: {}", service_name);
                    let mysql_service = MySQLService::new(config);
                    match mysql_service.update().await {
                        Ok(_) => (),
                        Err(error) => {
                            error!("Failed to run backup for MySQL service: {}, error: {}", service_name, error);
                        }
                    };
                })
            })?;

            sched.add(job).await?;
        }

        Ok(())
    }
}