use std::path::PathBuf;
use async_trait::async_trait;
use sqlx::types::chrono::{Local, Utc};
use tokio::process::Command;
use uuid::{NoContext, Timestamp, Uuid};
use which::which;
use crate::DB_POOL;
use crate::service::mysql::config::XtraBackupConfig;
use crate::service::mysql::mysql_service::MySQLService;

#[async_trait]
pub trait XtraBackupRunner {
    async fn do_xtrabackup(&self, mysql_config: &XtraBackupConfig) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
impl XtraBackupRunner for MySQLService {
    async fn do_xtrabackup(&self, mysql_config: &XtraBackupConfig) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config) = &self.config.backup {
            let defaults = self.get_defaults_file().await?;
            let defaults_path = defaults.path();

            // Create the current backup uuid.
            let backup_uuid = Uuid::new_v7(Timestamp::now(NoContext));
            let base_uuid: Option<Uuid> = None;
            let pool = DB_POOL.get().unwrap();

            // Figure out the target directory.
            let current_date = Local::now().format("%Y-%m-%d").to_string();
            let mut target_dir = PathBuf::from(&self.backup_config.basedir);
            target_dir.push(current_date);

            let command_path = which("xtrabackup")?;
            let mut cmd = Command::new(command_path);
            cmd.arg(format!("--defaults-file={}", defaults_path.to_str().unwrap()));
            cmd.arg("--backup");

            // Parallelize the backup process.
            if let Some(parallel_threads) = mysql_config.parallel_threads {
                cmd.arg(format!("--parallel={}", parallel_threads));
            }

            // Process the database exclusion.
            if let Some(databases_exclude) = &config.databases_exclude {
                cmd.arg(format!("--databases-exclude=\"{}\"", databases_exclude.join(" ")));
            }

            // Export only specific databases
            if let Some(databases) = &config.databases {
                cmd.arg(format!("--databases=\"{}\"", databases.join(" ")));
            }

            // Now if we are doing an incremental backup, we will want to handle it a little differently.
            if let Some(incremental_config) = &mysql_config.incremental {
                if incremental_config.enabled {
                    // Since we are creating an incremental backup, we just have to push the uuid into target directory.
                    target_dir.push(backup_uuid.to_string());

                    // We have to figure out the base directory.


                    //cmd.arg(format!("--incremental-basedir={}"));

                }
            }

            cmd.arg(format!("--target-dir={}", target_dir.to_str().unwrap()));

            // Store it in the database.
            {
                let path_str = target_dir.to_str().unwrap();
                let size = 0;
                let created_at = Utc::now().naive_utc();
                let result = sqlx::query("INSERT INTO backups (uuid, base_uuid, type, path, size, created_at) VALUES ($1, $2, 1, $4, $5, $6)")
                    .bind(backup_uuid)
                    .bind(base_uuid)
                    .bind(path_str)
                    .bind(size)
                    .bind(created_at)
                    .execute(pool).await?;

                println!("Test: {:?}", result);
            }
        }

        Ok(())
    }
}