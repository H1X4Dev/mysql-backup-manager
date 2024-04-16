use std::path::PathBuf;
use std::process::Stdio;
use async_trait::async_trait;
use log::debug;
use sqlx::types::chrono::{Local, Utc};
use tokio::process::Command;
use uuid::{NoContext, Timestamp, Uuid};
use which::which;
use crate::DB_POOL;
use crate::service::mysql::config::XtraBackupConfig;
use crate::service::mysql::database::MysqlBackupRow;
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
            debug!("Creating xtrabackup using {} defaults file.", defaults_path.to_str().unwrap());

            // Create the current backup uuid.
            let backup_uuid = Uuid::new_v7(Timestamp::now(NoContext));
            let mut base_uuid: Option<Uuid> = None;
            let pool = DB_POOL.get().unwrap();

            // Figure out the target directory.
            let current_date = Local::now().format("%Y-%m-%d").to_string();
            let mut target_dir = PathBuf::from(&self.backup_config.basedir);
            target_dir.push(current_date);
            debug!("Backup base directory: {}", target_dir.to_str().unwrap());

            let command_path = which("xtrabackup")?;
            let mut cmd = Command::new(command_path);
            cmd.arg(format!("--defaults-file={}", defaults_path.to_str().unwrap()));
            cmd.arg("--backup");

            // Parallelize the backup process.
            if let Some(parallel_threads) = mysql_config.parallel_threads {
                cmd.arg(format!("--parallel={}", parallel_threads));
                debug!("Will run the backup in parallel with {} threads.", parallel_threads);
            }

            // Process the database exclusion.
            if let Some(databases_exclude) = &config.databases_exclude {
                cmd.arg(format!("--databases-exclude=\"{}\"", databases_exclude.join(" ")));
                debug!("Excluding '{}' databases.", databases_exclude.join(" "));
            }

            // Export only specific databases
            if let Some(databases) = &config.databases {
                cmd.arg(format!("--databases=\"{}\"", databases.join(" ")));
                debug!("Only exporting '{}' databases.", databases.join(" "));
            }

            // Now if we are doing an incremental backup, we will want to handle it a little differently.
            if let Some(incremental_config) = &mysql_config.incremental {
                if incremental_config.enabled {
                    // Since we are creating an incremental backup, we just have to push the uuid into target directory.
                    target_dir.push(backup_uuid.to_string());
                    debug!("Target directory: {}", target_dir.to_str().unwrap());

                    // We have to figure out the base directory.
                    // how exactly do we figure out the base uuid???????????????????????????
                    // how about we just create that position index inside the directory and just track it there?
                    let previous_backup: Option<MysqlBackupRow> = sqlx::query_as("SELECT * FROM backups WHERE DATE(created_at) = $1 AND \"type\" = 1 ORDER BY uuid DESC")
                        .bind(Utc::now().date_naive())
                        .fetch_optional(pool)
                        .await?;

                    if let Some(backup_row) = previous_backup {
                        debug!("Previous backup found {} in {}.", backup_row.uuid, backup_row.path);
                        cmd.arg(format!("--incremental-basedir={}", backup_row.path));
                        base_uuid = Some(backup_row.uuid);
                    }
                }
            }

            // Now we export it to this directory.
            cmd.arg(format!("--target-dir={}", target_dir.to_str().unwrap()));

            // Run the command and expect output.
            let status = cmd.stdout(Stdio::null()).status().await?;
            if status.success() {
                debug!("-> Dumped!");

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
            } else {
                debug!("-> Failed to dump!");
            }
        }

        Ok(())
    }
}