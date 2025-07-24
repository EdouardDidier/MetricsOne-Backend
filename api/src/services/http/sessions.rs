use crate::{
    AppState,
    services::query_preparer::{SqlOperator, SqlType, select::SelectQuery},
};
use actix_web::{
    HttpResponse, Responder, get,
    web::{self, Data},
};
use serde::Deserialize;
use sqlx::Execute;
use tracing::{debug, error, info, trace};

use crate::models::Session;

/* ///////////////////////// */
/* //// HTTP Parameters //// */
/* ///////////////////////// */

#[derive(Debug, Clone, Deserialize)]
struct SessionsParams {
    pub key: Option<i32>,
    pub meeting: Option<i32>,
}

/* /////////////////////// */
/* //// HTTP Handlers //// */
/* /////////////////////// */

#[get("/sessions")]
async fn fetch_sessions(state: Data<AppState>, info: web::Query<SessionsParams>) -> impl Responder {
    let params = info.into_inner();

    debug!(parameters = ?params, "Request received with");
    let time = std::time::Instant::now();

    // Prepare the query
    let mut query_builder = prepare_query(&params);
    let query = query_builder.build();
    trace!("Query prepared in {:?}", time.elapsed());

    debug!("SQL query - {}", query.sql());

    // Execute the query
    match query.fetch_all(state.db.as_ref()).await {
        Ok(sessions) => {
            info!(
                "Fetched {} sessions successfully in {:?}",
                sessions.len(),
                time.elapsed()
            );
            HttpResponse::Ok().json(sessions)
        }
        Err(err) => {
            error!(error = ?err, "Failed to execute SQL request");
            HttpResponse::Ok().json(serde_json::json!([]))
        }
    }
}

/* ///////////////// */
/* //// Helpers //// */
/* ///////////////// */

fn prepare_query(params: &SessionsParams) -> SelectQuery<Session> {
    // Start to prepare the query
    let mut query_builder =
        SelectQuery::<Session>::new(Session::SQL_TABLE, Vec::from(Session::SQL_FIELDS));

    // Add 'filters' to the query
    if let Some(key) = params.key {
        query_builder.add_filter(
            (Session::SQL_TABLE, "key"),
            SqlOperator::Eq,
            SqlType::Int(key),
        );
    }

    if let Some(meeting_key) = params.meeting {
        query_builder.add_filter(
            (Session::SQL_TABLE, "meeting_key"),
            SqlOperator::Eq,
            SqlType::Int(meeting_key),
        );
    }

    query_builder
}
