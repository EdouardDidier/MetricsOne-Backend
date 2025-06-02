use metrics_one_macros::SqlNames;
use serde::{Deserialize, Serialize};
use sqlx::{self, FromRow, types::Json};

use crate::{DriversImages, Team};

#[derive(Serialize, Deserialize, FromRow, SqlNames)]
#[sql_names(table_name = "drivers")]
pub struct Driver {
    first_name: String,
    last_name: String,
    url: String,
    number: i32,
    year: i32,

    #[sql_names(skip)]
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    team: Option<Json<Team>>,

    #[sql_names(skip)]
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Json<DriversImages>>,
}
