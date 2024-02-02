use serde::Serialize;
use sqlx::types::chrono::{NaiveDateTime, Utc};
use sqlx::types::Uuid;
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
pub struct Backup {
    uuid: Uuid,
    #[sqlx(rename = "type")]
    backup_type: String,
    path: Option<String>,
    size: Option<String>,
    started_at: NaiveDateTime,
    ended_at: NaiveDateTime,


    incrbase: NaiveDateTime,
    weekno: u8,
    baseid: i32,
}