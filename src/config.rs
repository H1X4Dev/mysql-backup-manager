use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::fs;

pub struct test {
    pub interval: String,
    pub keep_last: u16
}

#[derive(Serialize, Deserialize, Debug)]
pub struct XtrabackupEncryptConfig {
    pub enabled: bool,
    pub key_file: String,
    pub threads: Option<u8>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MySQLBackupConfig {
    pub encrypt: Option<XtrabackupEncryptConfig>,
    pub databases: Option<Vec<String>>,
    pub databases_exclude: Option<Vec<String>>,
    pub incremental: Option<bool>,
    pub incremental_basedir: Option<String>,
    pub parallel_threads: Option<u8>,
    pub rsync: Option<bool>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MySQLConnectionConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub defaults_file: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ServiceConfigEnum {
    MySQL(MySQLConnectionConfig)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum BackupConfigEnum {
    MySQL(MySQLBackupConfig)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackupConfig {
    pub basedir: String,
    pub services: HashMap<String, BackupConfigEnum>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub services: HashMap<String, ServiceConfigEnum>,
    pub backup: BackupConfig,
}

impl Config {
    pub async fn new(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let config_str = fs::read_to_string(path).await?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }
}