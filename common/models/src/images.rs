use metrics_one_macros::SqlNames;
use serde::{Deserialize, Serialize};
use sqlx::{self, FromRow};

#[derive(Serialize, Deserialize, FromRow, SqlNames)]
#[sql_names(table_name = "drivers_images")]
pub struct DriversImages {
    headshot_url: String,
    profile_url: String,
}

#[derive(Serialize, Deserialize, FromRow, SqlNames)]
#[sql_names(table_name = "teams_images")]
pub struct TeamsImages {
    car_url: String,
    logo_url: String,
}
