use actix_web::{
    HttpResponse, Responder, get,
    web::{self, Data},
};
use metrics_one_utils::utils;
use serde::Deserialize;
use sqlx::Execute;
use tracing::{debug, error, info, instrument, trace};

use crate::{
    AppState,
    models::{Driver, Team, TeamsImages},
    services::query_preparer::{
        SqlType,
        select::{JoinRow, JoinType, RowType, SelectQuery},
    },
};

/* ///////////////////////// */
/* //// HTTP Parameters //// */
/* ///////////////////////// */

#[derive(Debug, Clone, Deserialize)]
pub struct TeamsParams {
    pub year: Option<i32>,
    pub name: Option<String>,
    pub expand: Option<String>,
}

impl TeamsParams {
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

#[instrument(name = "[HTTP Handler] GET /{year}/teams", skip_all)]
#[get("/{year}/teams")]
pub async fn fetch_teams(
    state: web::Data<AppState>,
    info: web::Query<TeamsParams>,
    path: web::Path<i32>,
) -> impl Responder {
    let params = TeamsParams {
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
        Ok(teams) => {
            info!(
                "Fetched {} teams successfully in {:?}",
                teams.len(),
                time.elapsed()
            );
            HttpResponse::Ok().json(teams)
        }
        Err(err) => {
            error!(error = ?err, "Failed to execute SQL request");
            HttpResponse::Ok().json(serde_json::json!([]))
        }
    }
}

#[instrument(name = "[HTTP Handler] GET /{year}/teams/{name}", skip_all)]
#[get("/{year}/teams/{name}")]
pub async fn fetch_team_by_name(
    state: Data<AppState>,
    info: web::Query<TeamsParams>,
    path: web::Path<(i32, String)>,
) -> impl Responder {
    let path = path.into_inner();
    let params = TeamsParams {
        year: Some(path.0),
        name: Some(path.1),
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
        Ok(Some(team)) => {
            info!("Fetched 1 team successfully in {:?}", time.elapsed());
            return HttpResponse::Ok().json(team);
        }
        Ok(None) => {
            info!("Fetched 0 team successfully in {:?}", time.elapsed());
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

fn prepare_query(params: &TeamsParams) -> SelectQuery<Team> {
    // Start to prepare the query
    let mut query_builder = SelectQuery::<Team>::new(Team::SQL_TABLE, Vec::from(Team::SQL_FIELDS));

    // Add 'expands' to the query
    let expands = params.get_expands();
    for exp in expands {
        match exp {
            "drivers" => query_builder.add_join(
                JoinType::LeftJoin,
                JoinRow::new(
                    RowType::AggBy(Team::SQL_TABLE, "id"),
                    Driver::SQL_TABLE,
                    Vec::from(Driver::SQL_FIELDS),
                    "drivers",
                ),
                (Team::SQL_TABLE, "id"),
                (Driver::SQL_TABLE, "team_id"),
            ),
            "images" => query_builder.add_join(
                JoinType::LeftJoin,
                JoinRow::new(
                    RowType::Single,
                    TeamsImages::SQL_TABLE,
                    Vec::from(TeamsImages::SQL_FIELDS),
                    "images",
                ),
                (Team::SQL_TABLE, "id"),
                (TeamsImages::SQL_TABLE, "team_id"),
            ),
            _ => (),
        }
    }

    // Add 'filters' to the query
    query_builder.add_filter(
        (Team::SQL_TABLE, "year"),
        SqlType::Int(utils::get_year(params.year)),
    );

    if let Some(name) = &params.name {
        query_builder.add_filter((Team::SQL_TABLE, "url"), SqlType::Text(name.clone()));
    }

    query_builder
}
