use chrono::{DateTime, Utc};
use metrics_one_proto::{proto, utils::timestamp_to_datetime};
use prost_types::Timestamp;
use sqlx::Execute;
use tracing::{debug, error, info, instrument, trace};

use crate::models::{Meeting, Session};
use crate::services::query_preparer::{SqlType, insert::InsertQuery};

use super::InsertServiceHandler;

/* ///////////////////// */
/* //// gRPC Helper //// */
/* ///////////////////// */

fn process_date(s: &Option<Timestamp>) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    let ts = &s.ok_or("Missing timestamp")?;
    timestamp_to_datetime(ts)
}

/* /////////////////////// */
/* //// gRPC Handlers //// */
/* /////////////////////// */

#[instrument(name = "[gRPC Handler] Insert Meetings", skip_all)]
pub async fn insert(
    handler: &InsertServiceHandler,
    request: tonic::Request<proto::InsertMeetingsRequest>,
) -> Result<tonic::Response<proto::InsertMeetingsResponse>, tonic::Status> {
    let year = request.get_ref().year;
    let meetings = request.into_inner().meetings;

    let nb_meetings = meetings.len();
    let mut nb_sessions = 0;

    debug!("Request received with {} insertions", meetings.len());
    let time = std::time::Instant::now();

    let response = proto::InsertMeetingsResponse {};

    // If no meetings, we do nothing and return an 'ok' response
    if meetings.is_empty() {
        return Ok(tonic::Response::new(response));
    }

    // Prepare queries
    let mut meetings_query = InsertQuery::new(Meeting::SQL_TABLE, Vec::from(Meeting::SQL_FIELDS));
    let mut sessions_query = InsertQuery::new(Session::SQL_TABLE, Vec::from(Session::SQL_FIELDS));

    for m in meetings.into_iter() {
        let mut meetings_values = Vec::new();

        // Order should be the same as 'SQL_FIELDS'
        meetings_values.push(SqlType::Int(m.key));
        meetings_values.push(SqlType::Int(m.number));
        meetings_values.push(SqlType::Text(m.location));
        meetings_values.push(SqlType::Text(m.official_name));
        meetings_values.push(SqlType::Text(m.name));
        meetings_values.push(SqlType::Int(year));

        if let Err(err) = meetings_query.add_values(meetings_values) {
            let message = "Failed to prepare 'meetings' query";
            error!(error = ?err, message);
            return Err(tonic::Status::internal(message));
        }

        nb_sessions += m.sessions.len();

        for s in m.sessions.into_iter() {
            let mut sessions_values = Vec::new();

            let start_date = match process_date(&s.start_date) {
                Ok(res) => res,
                Err(err) => {
                    let message = "Failed to parse timestamp";
                    error!(error = ?err, message);
                    return Err(tonic::Status::internal(message));
                }
            };

            let end_date = match process_date(&s.end_date) {
                Ok(res) => res,
                Err(err) => {
                    let message = "Failed to parse timestamp";
                    error!(error = ?err, message);
                    return Err(tonic::Status::internal(message));
                }
            };

            // Order should be the same as 'SQL_FIELDS'
            sessions_values.push(SqlType::Int(s.key));
            sessions_values.push(SqlType::Text(s.kind));
            sessions_values.push(SqlType::Text(s.name));
            sessions_values.push(SqlType::Timestamp(start_date));
            sessions_values.push(SqlType::Timestamp(end_date));
            sessions_values.push(SqlType::Text(s.path));
            sessions_values.push(SqlType::Int(m.key));

            if let Err(err) = sessions_query.add_values(sessions_values) {
                let message = "Failed to prepare 'sessions' query";
                error!(error = ?err, message);
                return Err(tonic::Status::internal(message));
            }
        }
    }

    let meetings_query = meetings_query.build();
    let sessions_query = sessions_query.build();
    trace!("Queries prepared in {:?}", time.elapsed());

    debug!("SQL query - {}", meetings_query.sql());

    if let Err(err) = meetings_query.execute(handler.db.as_ref()).await {
        let message = "Failed to process the SQL request";
        error!(error = ?err, message);
        return Err(tonic::Status::internal(message));
    }

    debug!("SQL query - {}", sessions_query.sql());

    if let Err(err) = sessions_query.execute(handler.db.as_ref()).await {
        let message = "Failed to process the SQL request";
        error!(error = ?err, message);
        return Err(tonic::Status::internal(message));
    }

    info!(
        "Inserted {} meetings and {} sessions successfully in {:?}",
        nb_meetings,
        nb_sessions,
        time.elapsed()
    );

    Ok(tonic::Response::new(response))
}
