use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::fs;


#[derive(Serialize, Deserialize, Debug)]
pub struct TimerConfig {
    pub interval: String,
    pub keep_last: u16
}

#[derive(Serialize, Deserialize, Debug)]
pub struct XtrabackupEncryptConfig {
    pub key_file: String,
    pub threads: Option<u8>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct XtraBackupIncrementalConfig {
    pub enabled: bool,
    pub basedir: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct XtraBackupConfig {
    pub encrypt: Option<XtrabackupEncryptConfig>,
    pub incremental: Option<XtraBackupIncrementalConfig>,
    pub parallel_threads: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MySQLDump {
    //
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum MySQLBackupType {
    xtrabackup(XtraBackupConfig),
    mysqldump(MySQLDump)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MySQLBackupConfig {
    #[serde(flatten)]
    pub backup_type: MySQLBackupType,
    pub databases: Option<Vec<String>>,
    pub databases_exclude: Option<Vec<String>>,
    #[serde(flatten)]
    pub timer: TimerConfig
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MySQLConnectionConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub defaults_file: Option<String>,
    pub backup: Option<MySQLBackupConfig>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ServiceConfigEnum {
    MySQL(MySQLConnectionConfig)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub backup_basedir: String,
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
            if let ServiceConfigEnum::MySQL(mysql_config) = service {
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
                if let Some(MySQLBackupConfig { backup_type: MySQLBackupType::xtrabackup(_), .. }) = &mysql_config.backup {
                    if cfg!(target_os = "windows") {
                        return Err("xtrabackup is not supported on Windows platforms.".into());
                    }
                }
            }
        }

        Ok(())
    }
}