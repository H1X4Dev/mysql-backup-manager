use serde::{Deserialize, Serialize};
use crate::config::TimerConfig;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XtrabackupEncryptConfig {
    pub key_file: String,
    pub threads: Option<u8>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XtraBackupIncrementalConfig {
    pub enabled: bool,
    pub basedir: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XtraBackupConfig {
    pub encrypt: Option<XtrabackupEncryptConfig>,
    pub incremental: Option<XtraBackupIncrementalConfig>,
    pub parallel_threads: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MySQLDump {
    //
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MySQLBackupType {
    xtrabackup(XtraBackupConfig),
    mysqldump(MySQLDump)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MySQLBackupConfig {
    #[serde(flatten)]
    pub backup_type: MySQLBackupType,
    pub databases: Option<Vec<String>>,
    pub databases_exclude: Option<Vec<String>>,
    #[serde(flatten)]
    pub timer: TimerConfig
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MySQLConnectionConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub defaults_file: Option<String>,
    pub backup: Option<MySQLBackupConfig>
}