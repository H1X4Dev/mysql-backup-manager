use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::fs;
use crate::service::mysql::config::{MySQLBackupConfig, MySQLBackupType, MySQLConnectionConfig};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ServiceConfigEnum {
    MySQL(MySQLConnectionConfig)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupConfig {
    pub basedir: String,
    pub keep_last: Option<u64>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub backup: BackupConfig,
    #[serde(flatten)]
    pub services: HashMap<String, ServiceConfigEnum>,
}

impl Config {
    pub async fn new(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let config_str = fs::read_to_string(path).await?;
        let config: Config = toml::from_str(&config_str)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        for service in self.services.values() {
            match service {
                ServiceConfigEnum::MySQL(mysql_config) => {
                    // Check 1: If defaults_file is specified, other connection options should not be.
                    if mysql_config.defaults_file.is_some() {
                        let other_options = [ &mysql_config.username, &mysql_config.password, &mysql_config.host ];
                        if other_options.iter().any(|option| option.is_some()) {
                            return Err("If defaults_file is specified, username, password, host, and port must not be set.".into());
                        }

                        // Check the port option separately
                        if mysql_config.port.is_some() {
                            return Err("If defaults_file is specified, port must not be set.".into());
                        }
                    }

                    // Check 2: If xtrabackup is selected, ensure it's not on Windows.
                    if let Some(MySQLBackupConfig { backup_type: MySQLBackupType::XtraBackup(_), .. }) = &mysql_config.backup {
                        if cfg!(target_os = "windows") {
                            return Err("xtrabackup is not supported on Windows platforms.".into());
                        }
                    }
                }
            }
        }

        Ok(())
    }
    /*
    pub async fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let result = toml::to_string(self)?;
        fs::write(path, result).await?;
        return Ok(())
    }*/
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;
    use tempfile::tempdir;
    use crate::service::mysql::config::{XtraBackupConfig, XtraBackupIncrementalConfig};

    #[tokio::test]
    async fn test_serialization() {
        let config = create_sample_config();
        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("[backup_basedir]"));
    }

    #[tokio::test]
    async fn test_deserialization() {
        let toml_str = r#"
[backup]
basedir = "/srv"
keep_last = 7

[mysql-r1]
type = "MySQL"
host = "127.0.0.1"
port = 3306
username = "root"
password = "123456"

[mysql-r1.backup]
type = "xtrabackup"
parallel_threads = 16
databases = ["auth", "wordpress"]
interval = "* * * * *"

[mysql-r1.backup.incremental]
enabled = true
basedir = "/home"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.backup.basedir, "/srv");
        assert_eq!(config.services.len(), 1);
    }

    #[tokio::test]
    async fn test_file_io() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("output.toml");
        let config = create_sample_config();

        let serialized = toml::to_string(&config).unwrap();
        fs::write(&file_path, serialized).unwrap();

        let mut file = fs::File::open(file_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert!(contents.contains("[backup]"));
    }

    fn create_sample_config() -> Config {
        Config {
            backup: BackupConfig {
                basedir:  "".to_string()
            },
            services: HashMap::from([
                ("mysql-r1".to_string(), ServiceConfigEnum::MySQL(MySQLConnectionConfig {
                    host: Some("127.0.0.1".to_string()),
                    port: Some(3306),
                    username: Some("root".to_string()),
                    password: Some("123456".to_string()),
                    socket: None,
                    defaults_file: None,
                    backup: Some(MySQLBackupConfig {
                        backup_type: MySQLBackupType::XtraBackup(XtraBackupConfig {
                            incremental: Some(true),
                            parallel_threads: Some(16),
                            use_memory: None,
                        }),
                        databases: Some(vec!["auth".to_string(), "wordpress".to_string()]),
                        databases_exclude: None,
                        interval: "* * * * *".to_string()
                    }),
                }))
            ])
        }
    }
}
