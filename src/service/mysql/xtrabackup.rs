use async_trait::async_trait;
use crate::service::mysql::config::XtraBackupConfig;
use crate::service::mysql::mysql_service::MySQLService;

#[async_trait]
pub trait XtraBackupRunner {
    async fn do_xtrabackup(&self, mysql_config: &XtraBackupConfig) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
impl XtraBackupRunner for MySQLService {
    async fn do_xtrabackup(&self, mysql_config: &XtraBackupConfig) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}