use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::str::FromStr;
use async_trait::async_trait;
use log::debug;
use sqlx::mysql::MySqlConnectOptions;
use sqlx::{MySqlPool, Row};
use sqlx::types::chrono::{Local, Utc};
use tokio::fs;
use tokio::process::Command;
use uuid::{NoContext, Timestamp, Uuid};
use which::which;
use crate::DB_POOL;
use crate::service::mysql::config::MySQLDumpConfig;
use crate::service::mysql::mysql_defaults::MySqlDefaultsReader;
use crate::service::mysql::mysql_service::MySQLService;
use crate::utils::get_size;

pub fn create_command(defaults_path: &Path, file_path: PathBuf) -> Result<Command, Box<dyn std::error::Error>> {
    let command_path = which("mysqldump")?;
    let mut cmd = Command::new(command_path);
    cmd.arg(format!("--defaults-file={}", defaults_path.to_str().unwrap()));
    cmd.arg("--quick");
    cmd.arg("--single-transaction");
    cmd.arg(format!("--result-file={}", file_path.to_str().unwrap()));
    Ok(cmd)
}

#[async_trait]
pub trait MySqlDumpRunner {
    async fn do_mysqldump(&self, mysql_config: &MySQLDumpConfig) -> Result<(), Box<dyn std::error::Error>>;

    async fn save_backup(&self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
impl MySqlDumpRunner for MySQLService {
    async fn do_mysqldump(&self, mysql_config: &MySQLDumpConfig) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config) = &self.config.backup {
            let defaults = self.get_defaults_file().await?;
            let defaults_path = defaults.path();

            // Create new pool.
            let connection_config = MySqlConnectOptions::from_defaults_file(defaults_path)?;
            let pool = MySqlPool::connect_lazy_with(connection_config);
            let current_date = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();

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
                    let temp_dir = PathBuf::from_str(&self.backup_config.basedir)?.join(current_date.clone()).join(database);
                    fs::create_dir_all(temp_dir.clone()).await?;

                    for table in tables {
                        let table_name: String = table.get(0);
                        debug!("Dumping table: {}.{}", database, table_name);

                        // Create a result path, where the SQL will be dumped off to.
                        let result_path = temp_dir.clone().join(format!("{}.{}.sql", database, table_name));

                        // Create the command to dump the data.
                        let mut cmd = create_command(defaults_path, result_path.clone())?;
                        cmd.arg(database);
                        cmd.arg(table_name);

                        // Run the command and expect output.
                        let status = cmd.stdout(Stdio::null()).status().await?;
                        if status.success() {
                            debug!("-> Dumped!");
                            // Save it to database.
                            self.save_backup(result_path.clone()).await?;

                        } else {
                            debug!("-> Failed to dump!");
                        }
                    }
                } else {
                    debug!("Dumping database: {}", database);

                    // Create a result path, where the SQL will be dumped off to.
                    fs::create_dir_all(self.backup_config.basedir.clone()).await?;
                    let result_path = PathBuf::from_str(&self.backup_config.basedir)?.join(format!("{}-{}.sql", current_date, database));

                    // Create the command to dump the data.
                    let mut cmd = create_command(defaults_path, result_path.clone())?;
                    cmd.arg(database);

                    // Run the command and expect output.
                    let status = cmd.stdout(Stdio::null()).status().await?;
                    if status.success() {
                        debug!("-> Dumped!");

                        // Save it to database.
                        self.save_backup(result_path.clone()).await?;
                    } else {
                        debug!("-> Failed to dump!");
                    }
                }
            }
        }
        Ok(())
    }

    async fn save_backup(&self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let uuid = Uuid::new_v7(Timestamp::now(NoContext));
        let path_str = path.to_str().unwrap().to_string();
        let size = get_size(path).unwrap() as i64;
        let created_at = Utc::now().naive_utc();

        sqlx::query("INSERT INTO backups (uuid, type, path, size, created_at) VALUES ($1, 0, $2, $3, $4)")
            .bind(uuid)
            .bind(path_str)
            .bind(size)
            .bind(created_at)
            .execute(DB_POOL.get().unwrap())
            .await?;

        Ok(())
    }
}
