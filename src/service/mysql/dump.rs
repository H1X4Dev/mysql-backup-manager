use async_trait::async_trait;

#[async_trait]
pub trait MySqlBackup {
    async fn dump();
}