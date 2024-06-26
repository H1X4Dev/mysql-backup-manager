use sqlx::types::chrono::NaiveDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct MysqlBackupRow {
    pub uuid: Uuid,
    pub base_uuid: Option<Uuid>, // used for xtrabackup
    #[sqlx(rename = "type")]
    pub backup_type: u8, // 0 = mysqldump, 1 = xtrabackup
    pub path: String,
    pub size: i64,
    pub created_at: NaiveDateTime
}