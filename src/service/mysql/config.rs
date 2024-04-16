use serde::{Deserialize, Serialize};
use crate::config::TimerConfig;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XtraBackupIncrementalConfig {
    pub enabled: bool,
    pub basedir: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XtraBackupConfig {
    pub incremental: Option<XtraBackupIncrementalConfig>,
    pub parallel_threads: Option<u8>,
    pub use_memory: Option<u32>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MySQLDumpConfig {
    pub separate_tables: Option<bool>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MySQLBackupType {
    #[serde(rename = "xtrabackup")]
    XtraBackup(XtraBackupConfig),
    #[serde(rename = "mysqldump")]
    MySqlDump(MySQLDumpConfig)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MySQLBackupConfig {
    #[serde(flatten)]
    pub backup_type: MySQLBackupType,
    pub databases: Option<Vec<String>>,
    pub databases_exclude: Option<Vec<String>>,
    pub interval: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MySQLConnectionConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub socket: Option<String>,
    pub defaults_file: Option<String>,
    pub backup: Option<MySQLBackupConfig>
}