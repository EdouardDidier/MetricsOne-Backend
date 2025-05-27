use crate::{
    AppState,
    services::query_preparer::{SqlType, select::SelectQuery},
};
use actix_web::{
    HttpResponse, Responder, get,
    web::{self, Data},
};
use chrono::Datelike;
use metrics_one_proto::proto::{self, insert_meetings_request::Meeting};
use metrics_one_utils::utils;
use serde::Deserialize;
use sqlx::Execute;
use tracing::instrument;
use tracing::{debug, error, info, trace};

/* ///////////////////////// */
/* //// HTTP Parameters //// */
/* ///////////////////////// */

#[derive(Debug, Clone, Deserialize)]
struct MeetingsParams {
    pub year: Option<i32>,
}

/* /////////////////////// */
/* //// HTTP Handlers //// */
/* /////////////////////// */

#[instrument(name = "[HTTP Handler] GET /{year}/drivers", skip_all)]
#[get("/{year}/meetings")]
async fn fetch_meetings(
    state: Data<AppState>,
    _info: web::Query<MeetingsParams>,
    path: web::Path<i32>,
) -> impl Responder {
    let mut worker_client = state.worker.clone();

    let params = MeetingsParams {
        year: Some(path.into_inner()),
    };

    debug!(parameters = ?params, "Request received with");
    let time = std::time::Instant::now();

    // Prepare the query
    let mut query_builder = prepare_query(&params);
    let query = query_builder.build();
    trace!("Query prepared in {:?}", time.elapsed());

    debug!("SQL query - {}", query.sql());

    // Execute the query
    let meetings = match query.fetch_all(state.db.as_ref()).await {
        Ok(meetings) => {
            info!(
                "Fetched {} meetings successfully in {:?}",
                meetings.len(),
                time.elapsed()
            );
            meetings
        }
        Err(err) => {
            error!(error = ?err, "Failed to execute SQL request");
            return HttpResponse::Ok().json(serde_json::json!([]));
        }
    };

    let year = utils::get_year(params.year);
    let current_year = chrono::Utc::now().year();

    // If meetings are found in the database and the request is from a previous year
    // Reply with fetched data
    // Otherwise proceed to prepare gRPC request to fetch additional meetings
    if meetings.len() > 1 && year != current_year {
        return HttpResponse::Ok().json(meetings);
    }

    // Prepare gRPC request
    let meetings_keys = meetings.iter().map(|m| m.key).collect();
    let req = proto::FetchMeetingsRequest {
        year,
        keys: meetings_keys,
    };

    // Send fetch request to the worker
    match worker_client.fetch_meetings(req).await {
        Ok(_) => {
            // Respond with "Accepted" status to indicate the request is being process
            HttpResponse::Accepted().json(meetings)
        }
        Err(err) => {
            error!(error = ?err, "Failed to execute gRPC request");
            HttpResponse::InternalServerError().json(serde_json::json!([]))
        }
    }
}

/* ///////////////// */
/* //// Helpers //// */
/* ///////////////// */

fn prepare_query(params: &MeetingsParams) -> SelectQuery<Meeting> {
    // Start to prepare the query
    let mut query_builder =
        SelectQuery::<Meeting>::new(Meeting::SQL_TABLE, Vec::from(Meeting::SQL_FIELDS));

    // Add 'filters' to the query
    query_builder.add_filter(
        (Meeting::SQL_TABLE, "year"),
        SqlType::Int(utils::get_year(params.year)),
    );

    query_builder
}
