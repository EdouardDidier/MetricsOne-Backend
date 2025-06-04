use metrics_one_proto::{proto, utils::timestamp_to_datetime};
use sqlx::Execute;
use tracing::{debug, error, info, instrument, trace};

use crate::models::{Meeting, Session};
use crate::services::query_preparer::{SqlType, insert::InsertQuery};

use super::InsertServiceHandler;

#[instrument(name = "[gRPC Handler] Insert Meetings", skip_all)]
pub async fn insert(
    handler: &InsertServiceHandler,
    request: tonic::Request<proto::InsertMeetingsRequest>,
) -> Result<tonic::Response<proto::InsertMeetingsResponse>, tonic::Status> {
    let year = request.get_ref().year;
    let meetings = request.into_inner().meetings;

    let nb_meetings = meetings.len();

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

    // TODO: use move statement and into_iter() to avoid data cloning
    for m in meetings.into_iter() {
        let mut meetings_values = Vec::new();

        // TODO: Order matter here, change so it doesn't depend on order
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

        for s in m.sessions.into_iter() {
            let mut sessions_values = Vec::new();

            // TODO: Handle errors properly
            let start_date = timestamp_to_datetime(&s.start_date.unwrap()).unwrap();
            let end_date = timestamp_to_datetime(&s.end_date.unwrap()).unwrap();

            // TODO: Order matter here, change so it doesn't depend on order
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

    //TODO: Add info on the number of sessions added
    info!(
        "Inserted {} 'meetings' successfully in {:?}",
        nb_meetings,
        time.elapsed()
    );

    Ok(tonic::Response::new(response))
}
