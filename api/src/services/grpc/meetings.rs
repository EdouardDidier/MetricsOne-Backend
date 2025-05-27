use metrics_one_proto::proto::{self, insert_meetings_request::Meeting};
use sqlx::Execute;
use tracing::{debug, error, info, instrument, trace};

use crate::{
    // models::Meeting,
    services::query_preparer::{SqlType, insert::InsertQuery},
};

use super::InsertServiceHandler;

#[instrument(name = "[gRPC Handler] Insert Meetings", skip_all)]
pub async fn insert(
    handler: &InsertServiceHandler,
    request: tonic::Request<proto::InsertMeetingsRequest>,
) -> Result<tonic::Response<proto::InsertMeetingsResponse>, tonic::Status> {
    let meetings = &request.get_ref().meetings;

    debug!("Request received with {} insertions", meetings.len());
    let time = std::time::Instant::now();

    let response = proto::InsertMeetingsResponse {};

    // If no meetings, we do nothing and return an 'ok' response
    if meetings.is_empty() {
        return Ok(tonic::Response::new(response));
    }

    // Prepare the query
    let mut query_builder = InsertQuery::new(Meeting::SQL_TABLE, Vec::from(Meeting::SQL_FIELDS));

    for m in meetings {
        let mut values = Vec::new();

        values.push(SqlType::Int(m.key));
        values.push(SqlType::Int(m.number));
        values.push(SqlType::Text(m.location.clone()));
        values.push(SqlType::Text(m.official_name.clone()));
        values.push(SqlType::Text(m.name.clone()));
        values.push(SqlType::Int(m.year));

        if let Err(err) = query_builder.add_values(values) {
            let message = "Failed to preapre the query";
            error!(error = ?err, message);
            return Err(tonic::Status::internal(message));
        }
    }

    let query = query_builder.build();
    trace!("Query prepared in {:?}", time.elapsed());

    debug!("SQL query - {}", query.sql());

    if let Err(err) = query.execute(handler.db.as_ref()).await {
        let message = "Failed to process the SQL request";
        error!(error = ?err, message);
        return Err(tonic::Status::internal(message));
    }

    info!(
        "Inserted {} entries successfully in {:?}",
        meetings.len(),
        time.elapsed()
    );

    Ok(tonic::Response::new(response))
}
