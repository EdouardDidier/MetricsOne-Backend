use metrics_one_macros::SqlNames;
use serde::{Deserialize, Serialize};
use sqlx::{self, FromRow, prelude::Type, types::Json};

use super::{Driver, TeamsImages};

#[derive(Serialize, Deserialize, FromRow, SqlNames, Type)]
#[sql_names(table_name = "teams")]
pub struct Team {
    name: String,
    url: String,
    colour: String,
    year: i32,

    #[sql_names(skip)]
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    drivers: Option<Json<Vec<Driver>>>,

    #[sql_names(skip)]
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Json<TeamsImages>>,
}
