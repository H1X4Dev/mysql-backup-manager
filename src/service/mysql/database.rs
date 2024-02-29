use sqlx::types::chrono::NaiveDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct XtraBackupSaveRow {
    uuid: Uuid,
    base_uuid: Option<Uuid>,
    path: String,
    size: u64,
    day: u8,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime
}

#[derive(Debug, FromRow)]
pub struct MysqlDumpBackupSaveRow {
    uuid: Uuid,
    path: String,
    size: u64,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime
}