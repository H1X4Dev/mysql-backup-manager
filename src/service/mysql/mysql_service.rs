use std::str::FromStr;
use std::sync::{Arc, Mutex};
use log::{error, info};
use tokio_cron_scheduler::{Job, JobScheduler};
use crate::service::mysql::config::MySQLConnectionConfig;
use crate::service::service::Service;
use cron::Schedule;

pub struct MySQLService {
    pub config: MySQLConnectionConfig
}

impl MySQLService {
    pub fn new(config: MySQLConnectionConfig) -> MySQLService {
        return MySQLService {
            config
        }
    }
}

impl Service for MySQLService {
    async fn update(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Updating!");
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