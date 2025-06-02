use metrics_one_macros::SqlNames;
use serde::{Deserialize, Serialize};
use sqlx::{self, FromRow, types::Json};

use super::Session;

#[derive(Serialize, Deserialize, FromRow, SqlNames)]
#[sql_names(table_name = "meetings")]
pub struct Meeting {
    pub key: i32,
    pub number: i32,
    pub location: String,
    pub official_name: String,
    pub name: String,
    pub year: i32,

    #[sql_names(skip)]
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Json<Vec<Session>>>,
}
