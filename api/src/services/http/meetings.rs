use crate::{
    AppState,
    models::Session,
    services::query_preparer::{
        SqlOperator, SqlType,
        select::{JoinRow, JoinType, RowType, SelectQuery},
    },
};
use actix_web::{
    HttpResponse, Responder, get,
    web::{self, Data},
};
use chrono::Datelike;
use lapin::{
    BasicProperties,
    types::{AMQPValue, FieldTable},
};
use opentelemetry::{global, propagation::Injector};
use serde::Deserialize;
use sqlx::Execute;
use tracing::{Span, debug, error, info, trace};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::models::Meeting;

/* ///////////////////////// */
/* //// HTTP Parameters //// */
/* ///////////////////////// */

#[derive(Debug, Clone, Deserialize)]
struct MeetingsParams {
    pub key: Option<i32>,
    pub location: Option<String>,
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

// TODO: Move to the common crate
struct AmqpHeaderInjector<'a> {
    headers: &'a mut FieldTable,
}

impl<'a> Injector for AmqpHeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.headers
            .insert(key.into(), AMQPValue::LongString(value.into()));
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
    let rabbitmq = state.rabbitmq.clone();

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

    // If meetings are found in the database, send fetched data
    // But if it's current year, new data might be availaible
    // So proceed to send a request to fetch new data
    if meetings.len() > 0 && params.get_year() != chrono::Utc::now().year() {
        return HttpResponse::Ok().json(meetings);
    }

    // If there are filters parameters, it might just be a bad filter
    // So, it doesn't trigger a fetch job even if there are no meetings
    if params.key.is_some() || params.location.is_some() {
        return HttpResponse::Ok().json(meetings);
    }

    // Prepare RabbitMQ payload
    let meetings_keys = meetings.iter().map(|m| m.key).collect();

    let rabbitmq_payload = metrics_one_queue::models::Meetings {
        year: params.get_year(),
        keys: meetings_keys,
    };

    // Encode payload into JSON
    let rabbitmq_body = match serde_json::to_vec(&rabbitmq_payload) {
        Ok(body) => {
            trace!("Serialized queue payload in {:?}", time.elapsed());
            body
        }
        Err(err) => {
            error!(error = ?err, "Failed to serialize queue payload");
            return HttpResponse::Ok().json(serde_json::json!([]));
        }
    };

    let mut rabbitmq_headers = FieldTable::default();

    // Inject trace context into RabbitMQ headers
    let context = Span::current().context();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(
            &context,
            &mut AmqpHeaderInjector {
                headers: &mut rabbitmq_headers,
            },
        )
    });

    let rabbitmq_request_properties = BasicProperties::default()
        .with_headers(rabbitmq_headers)
        .with_content_type("application/json".into());

    // Prepare RabbitMQ request
    let rabbitmq_request = rabbitmq.basic_publish(
        "",
        "fetch.meetings",
        lapin::options::BasicPublishOptions::default(),
        &rabbitmq_body,
        rabbitmq_request_properties,
    );

    // Send fetch request to the queue
    let rabbitmq_publish = match rabbitmq_request.await {
        Ok(publish) => {
            trace!(
                "Published meetings fetch request to the queue in {:?}",
                time.elapsed()
            );
            publish
        }
        Err(err) => {
            error!(error = ?err, "Failed to publish meetings fetch request to the queue");
            return HttpResponse::Ok().json(serde_json::json!([]));
        }
    };

    // Check if acknowledgement received
    // TODO: Check if producer acknowledgement is necessary ?
    match rabbitmq_publish.await {
        Ok(_) => {
            trace!("Acknowledgement received in {:?}", time.elapsed());
            // If we fetched meetings earlier, send data as a response
            if meetings.len() > 0 {
                return HttpResponse::Ok().json(meetings);
            }

            // Respond with "Accepted" status to indicate the request is being process
            HttpResponse::Accepted().json(serde_json::json!([]))
        }
        Err(err) => {
            error!(error = ?err, "Failed to receive queue confirmation");
            HttpResponse::Ok().json(meetings)
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
        SqlOperator::Eq,
        SqlType::Int(params.get_year()),
    );

    if let Some(key) = params.key {
        query_builder.add_filter(
            (Meeting::SQL_TABLE, "key"),
            SqlOperator::Eq,
            SqlType::Int(key),
        );
    }

    if let Some(location) = &params.location {
        query_builder.add_filter(
            (Meeting::SQL_TABLE, "location"),
            SqlOperator::ILike,
            SqlType::Text(location.clone()),
        );
    }

    query_builder
}
