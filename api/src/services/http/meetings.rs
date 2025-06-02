use crate::{
    AppState,
    models::Session,
    services::query_preparer::{
        SqlType,
        select::{JoinRow, JoinType, RowType, SelectQuery},
    },
};
use actix_web::{
    HttpResponse, Responder, get,
    web::{self, Data},
};
use chrono::Datelike;
use metrics_one_proto::proto::{self};
use serde::Deserialize;
use sqlx::Execute;
use tracing::{debug, error, info, trace};

use crate::models::Meeting;

/* ///////////////////////// */
/* //// HTTP Parameters //// */
/* ///////////////////////// */

#[derive(Debug, Clone, Deserialize)]
struct MeetingsParams {
    pub year: Option<i32>,
    pub expand: Option<String>,
}

impl MeetingsParams {
    fn get_year(&self) -> i32 {
        match self.year {
            Some(res) => res,
            None => {
                let current_year = chrono::Utc::now().year();
                debug!("No year specified, defaulting to {}", current_year);
                current_year
            }
        }
    }

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

#[get("/{year}/meetings")]
async fn fetch_meetings(
    state: Data<AppState>,
    info: web::Query<MeetingsParams>,
    path: web::Path<i32>,
) -> impl Responder {
    let params = MeetingsParams {
        year: Some(path.into_inner()),
        ..info.into_inner()
    };
    let mut worker = state.worker.clone();

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

    // If meetings are found in the database,
    if meetings.len() > 1 {
        return HttpResponse::Ok().json(meetings);
    }

    // Prepare gRPC request
    let meetings_keys = meetings.iter().map(|m| m.key).collect();
    let req = proto::FetchMeetingsRequest {
        year: params.get_year(),
        keys: meetings_keys,
    };

    // Send fetch request to the worker
    match worker.fetch_meetings(req).await {
        Ok(_) => {
            // Respond with "Accepted" status to indicate the request is being process
            HttpResponse::Accepted().body("[]")
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

    // Add 'expands' to the query
    let expands = params.get_expands();
    for exp in expands {
        match exp {
            "sessions" => query_builder.add_join(
                JoinType::LeftJoin,
                JoinRow::new(
                    RowType::AggBy(Meeting::SQL_TABLE, "id"),
                    Session::SQL_TABLE,
                    Vec::from(Session::SQL_FIELDS),
                    "sessions",
                ),
                (Meeting::SQL_TABLE, "key"),
                (Session::SQL_TABLE, "meeting_key"),
            ),
            _ => (),
        }
    }

    // Add 'filters' to the query
    query_builder.add_filter(
        (Meeting::SQL_TABLE, "year"),
        SqlType::Int(params.get_year()),
    );

    query_builder
}
