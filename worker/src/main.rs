mod fetch;
mod models;
mod settings;

use std::{collections::HashMap, sync::Arc, time::Duration};

use lapin::types::{AMQPValue, FieldTable};
use metrics_one_grpc::proto::insert_service_client::InsertServiceClient;
use metrics_one_utils::{grpc::try_get_grpc_client, utils};
use settings::ENV;
use tokio_stream::StreamExt;
use tracing::{Span, debug, error, info, info_span, trace};

use opentelemetry::{KeyValue, global, propagation::Extractor};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::fetch::meetings::fetch_job;

// TODO: Move to common crate and move to traditional struct
struct AmqpHeaderExtractor {
    headers: HashMap<String, String>,
}

impl AmqpHeaderExtractor {
    fn from_field_table(field_table: &FieldTable) -> Self {
        let headers = field_table
            .inner()
            .iter()
            .filter_map(|(k, v)| match v {
                AMQPValue::LongString(s) => Some((k.to_string(), s.to_string())),
                _ => None,
            })
            .collect();

        Self { headers }
    }
}

impl Extractor for AmqpHeaderExtractor {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|v| v.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.headers.keys().map(|k| k.as_str()).collect()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let _otel_guard =
        metrics_one_utils::otel::init_tracing_subscriber("metrics-one-worker", &ENV.rust_log);

    let meter = global::meter("metrics-one-worker");
    let counter = meter.u64_counter("m1.messages.count").build();

    // TODO: Move above code to its own crate

    // Setup of gRPC - TODO: Move to its own class
    let api_client = {
        let _span = info_span!("gRPC setup").entered();

        let addr = format!("http://{}:{}", ENV.api.host, ENV.api.port);
        debug!("Connection to API service on {} initiated", addr);

        // Connection to API with gRPC
        match try_get_grpc_client(InsertServiceClient::connect, &addr, Duration::from_secs(1)).await
        {
            Some(res) => {
                info!("Connection to API service established");
                res
            }
            None => {
                info!("No connection to API service, aborting server startup...");
                return Ok(());
            }
        }
    };

    // Setup of RabbitMQ - TODO: Move to its own class
    let mut rabbitmq_consumer = {
        let _span = info_span!("RabbitMQ setup").entered();

        // Connection to RabbitMQ
        let addr = format!("{}:{}", ENV.rabbitmq.host, ENV.rabbitmq.port);
        let channel = Arc::new(
            metrics_one_queue::get_rabbitmq_channel(
                &addr,
                &ENV.rabbitmq.user,
                &ENV.rabbitmq.password,
                &ENV.rabbitmq.queue, // TODO: Move as constant to the common crate
            )
            .await
            .inspect_err(|err| {
                error!(error = ?err, "Failed to connect to RabbitMQ");
            })?,
        );

        // Initializing RabbitMQ listenser
        // TODO: Create a class to handle multiple queues
        let consumer = channel
            .basic_consume(
                &ENV.rabbitmq.queue, // TODO: Move as constant to the common crate
                "worker",            // TODO: Move to a constant
                lapin::options::BasicConsumeOptions::default(),
                lapin::types::FieldTable::default(),
            )
            .await
            .inspect_err(|err| {
                error!(error = ?err, "Consumer failed");
            })?;

        info!(
            "RabbitMQ consumer setup completed and listening to queue '{}'",
            ENV.rabbitmq.queue
        );

        consumer
    };

    // Start listening on RabbitMQ
    tokio::select! {
        _ = utils::get_shutdown_signals() => {
            info!("Shutdown signal received, shutting down the server...");
        }
        _ = async {
            // TODO: Create a class to handle multiple queues
            while let Some(delivery) = rabbitmq_consumer.next().await {

                let delivery = match delivery {
                    Ok(delivery) => delivery,
                    Err(err) => {
                        error!(error = ?err, "Error in RabbitMQ consumer, stopping consumption");
                        counter.add(1, &[KeyValue::new("message.status", "failed")]);

                        break;
                    }
                };

                // Get Trace context from request metadata
                let parent_cx = if let Some(headers) = delivery.properties.headers() {
                    global::get_text_map_propagator(|propagator| {
                        propagator.extract(&AmqpHeaderExtractor::from_field_table(headers))
                    })
                } else {
                    Span::current().context()
                };

                let span = info_span!("Message consumer");
                span.set_parent(parent_cx);
                let _enter = span.enter();

                // Deserialize the message
                let payload: metrics_one_queue::models::Meetings = match serde_json::from_slice(&delivery.data) {
                    Ok(payload) => payload,
                    Err(err) => {
                        error!(error = ?err, "Failed to deserialize message payload, discarding message");
                        counter.add(1, &[KeyValue::new("message.status", "failed")]);

                        if let Err(err) = delivery.nack(lapin::options::BasicNackOptions::default()).await {
                            error!(error = ?err, "Failed to nack message");
                        }
                        continue;
                    }
                };

                // The message is now correctly deserialized, we can process it
                match fetch_job(api_client.clone(), payload).await {
                    Ok(_) => {
                        trace!("Successfully processed message");
                        counter.add(1, &[KeyValue::new("message.status", "success")]);

                        if let Err(err) = delivery.ack(lapin::options::BasicAckOptions::default()).await {
                            error!(error = ?err, "Failed to ack message");
                        }
                    }
                    Err(err) => {
                        error!(error = ?err, "Failed to process message");
                        counter.add(1, &[KeyValue::new("message.status", "failed")]);

                        if let Err(err) = delivery.nack(lapin::options::BasicNackOptions::default()).await {
                            error!(error = ?err, "Failed to nack message");
                        }
                    }
                }
            }
        } => {}
    }

    Ok(())
}
