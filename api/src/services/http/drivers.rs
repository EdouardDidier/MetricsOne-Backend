use actix_web::{
    HttpResponse, Responder, get,
    web::{self, Data},
};
use metrics_one_utils::utils;
use serde::Deserialize;
use sqlx::Execute;
use tracing::{debug, error, info, instrument, trace};

use metrics_one_models::{Driver, DriversImages, Team};

use crate::{
    AppState,
    services::query_preparer::{
        SqlType,
        select::{JoinRow, JoinType, RowType, SelectQuery},
    },
};

/* ///////////////////////// */
/* //// HTTP Parameters //// */
/* ///////////////////////// */

#[derive(Debug, Clone, Deserialize)]
pub struct DriversParams {
    pub year: Option<i32>,
    pub name: Option<String>,
    pub expand: Option<String>,
}

impl DriversParams {
    pub fn get_expands<'q>(&'q self) -> Vec<&'q str> {
        if let Some(expands) = &self.expand {
            return expands.split(",").collect();
        }

        // Dafault to an empty vector
        Vec::new()
    }
}

/* /////////////////////// */
/* //// HTTP Handlers //// */
/* /////////////////////// */

#[instrument(name = "[HTTP Handler] GET /{year}/drivers", skip_all)]
#[get("/{year}/drivers")]
pub async fn fetch_drivers(
    state: web::Data<AppState>,
    info: web::Query<DriversParams>,
    path: web::Path<i32>,
) -> impl Responder {
    let params = DriversParams {
        year: Some(path.into_inner()),
        ..info.into_inner()
    };

    debug!(paramters = ?params, "Request received with");
    let time = std::time::Instant::now();

    // Prepare the query
    let mut query_builder = prepare_query(&params);
    let query = query_builder.build();
    trace!("Query prepared in {:?}", time.elapsed());

    debug!("SQL query - {}", query.sql());

    // Execute the query
    match query.fetch_all(state.db.as_ref()).await {
        Ok(drivers) => {
            info!(
                "Fetched {} drivers successfully in {:?}",
                drivers.len(),
                time.elapsed()
            );
            HttpResponse::Ok().json(drivers)
        }
        Err(err) => {
            error!(error = ?err, "Failed to execute SQL request");
            HttpResponse::Ok().json(serde_json::json!([]))
        }
    }
}

#[instrument(name = "[HTTP Handler] GET /{year}/drivers/{name}", skip_all)]
#[get("/{year}/drivers/{name}")]
pub async fn fetch_driver_by_name(
    state: Data<AppState>,
    info: web::Query<DriversParams>,
    path: web::Path<(i32, String)>,
) -> impl Responder {
    let path = path.into_inner();
    let params = DriversParams {
        name: Some(path.1),
        year: Some(path.0),
        ..info.into_inner()
    };

    debug!(paramters = ?params, "Request received with");
    let time = std::time::Instant::now();

    // Prepare the query
    let mut query_builder = prepare_query(&params);
    let query = query_builder.build();
    trace!("Query prepared in {:?}", time.elapsed());

    debug!("SQL query - {}", query.sql());

    // Execute the query
    match query.fetch_optional(state.db.as_ref()).await {
        Ok(Some(driver)) => {
            info!("Fetched 1 driver successfully in {:?}", time.elapsed());
            return HttpResponse::Ok().json(driver);
        }
        Ok(None) => {
            info!("Fetched 0 driver successfully in {:?}", time.elapsed());
        }
        Err(err) => {
            info!(error = ?err, "Failed to execute SQL request");
        }
    };

    HttpResponse::Ok().json(serde_json::json!({}))
}

/* ///////////////// */
/* //// Helpers //// */
/* ///////////////// */

fn prepare_query(params: &DriversParams) -> SelectQuery<Driver> {
    // Start to prepare the query
    let mut query_builder =
        SelectQuery::<Driver>::new(Driver::SQL_TABLE, Vec::from(Driver::SQL_FIELDS));

    // Add 'expands' to the query
    let expands = params.get_expands();
    for exp in expands {
        match exp {
            "team" => query_builder.add_join(
                JoinType::LeftJoin,
                JoinRow::new(
                    RowType::Single,
                    Team::SQL_TABLE,
                    Vec::from(Team::SQL_FIELDS),
                    "team",
                ),
                (Driver::SQL_TABLE, "team_id"),
                (Team::SQL_TABLE, "id"),
            ),
            "images" => query_builder.add_join(
                JoinType::LeftJoin,
                JoinRow::new(
                    RowType::Single,
                    DriversImages::SQL_TABLE,
                    Vec::from(DriversImages::SQL_FIELDS),
                    "images",
                ),
                (Driver::SQL_TABLE, "id"),
                (DriversImages::SQL_TABLE, "driver_id"),
            ),
            _ => (),
        }
    }

    // Add 'filters' to the query
    query_builder.add_filter(
        (Driver::SQL_TABLE, "year"),
        SqlType::Int(utils::get_year(params.year)),
    );

    if let Some(name) = &params.name {
        query_builder.add_filter((Driver::SQL_TABLE, "url"), SqlType::Text(name.clone()));
    }

    query_builder
}
