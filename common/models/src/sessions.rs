use chrono::{DateTime, Utc};
use metrics_one_macros::SqlNames;
use serde::{Deserialize, Serialize};
use sqlx::{self, FromRow};

#[derive(Serialize, Deserialize, FromRow, SqlNames)]
#[sql_names(table_name = "sessions")]
pub struct Session {
    key: i32,
    kind: String,
    name: String,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    path: String,
    meeting_key: i32,
}
