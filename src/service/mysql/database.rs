use sqlx::types::chrono::NaiveDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct MysqlBackupRow {
    uuid: Uuid,
    base_uuid: Option<Uuid>, // used for xtrabackup
    #[sqlx(rename = "type")]
    backup_type: u8, // 0 = mysqlbackup, 1 = xtrabackup
    storage_type: u8, // 0 = local, 1 = s3 bucket
    database_name: String,
    path: String,
    size: u64,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime
}