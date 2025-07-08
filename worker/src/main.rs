mod fetch;
mod models;
mod settings;

use std::{sync::Arc, time::Duration};

use metrics_one_models::queue;
use metrics_one_proto::proto::insert_service_client::InsertServiceClient;
use metrics_one_utils::{grpc::try_get_grpc_client, utils};
use settings::ENV;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, trace};

use crate::fetch::meetings::fetch_job;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up logger
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(tracing_subscriber::EnvFilter::new(ENV.rust_log.clone()))
        .init();

    // Connection to API with gRPC
    let api_addr = format!("http://{}:{}", ENV.api.host, ENV.api.port);
    let api_client = match try_get_grpc_client(
        InsertServiceClient::connect,
        &api_addr,
        Duration::from_secs(1),
    )
    .await
    {
        Some(res) => {
            info!("Connection to API service established on {}", api_addr);
            res
        }
        None => {
            info!("No connection to API service, aborting server startup...");
            return Ok(());
        }
    };

    // Connection to RabbitMQ
    // TODO: Move to its own crate
    let rabbitmq_addr = format!("{}:{}", ENV.rabbitmq.host, ENV.rabbitmq.port);
    debug!(
        "Connection to RabbitMQ on amqp://{} initiated",
        rabbitmq_addr
    );

    let rabbitmq_connection = lapin::Connection::connect(
        &format!(
            "amqp://{}:{}@{}/%2f",
            ENV.rabbitmq.user, ENV.rabbitmq.password, rabbitmq_addr
        ),
        lapin::ConnectionProperties::default(),
    )
    .await
    .inspect_err(|err| {
        error!(error = ?err, "Failed to connect to RabbitMQ");
    })?;

    info!("Connection to RabbitMQ established");

    // Set up of RabbitMQ channels
    // TODO: Move to its own crate
    debug!("Set up of RabbitMQ initiated");

    let rabbitmq_channel = Arc::new(rabbitmq_connection.create_channel().await.inspect_err(
        |err| {
            error!(error = ?err, "Failed to create RabbitMQ channel");
        },
    )?);

    rabbitmq_channel
        .queue_declare(
            "fetch.meetings",
            lapin::options::QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            lapin::types::FieldTable::default(),
        )
        .await
        .inspect_err(|err| {
            error!(error = ?err, "Failed to declare RabbitMQ queue");
        })?;

    info!("Set up of RabbitMQ completed");

    // Initializing RabbitMQ listenser
    let mut rabbitmq_consumer = rabbitmq_channel
        .basic_consume(
            &ENV.rabbitmq.queue,
            "worker",
            lapin::options::BasicConsumeOptions::default(),
            lapin::types::FieldTable::default(),
        )
        .await
        .inspect_err(|err| {
            error!(error = ?err, "Consumer failed");
        })?;

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
                        break;
                    }
                };

                // Deserialize the message
                let payload: queue::Meetings = match serde_json::from_slice(&delivery.data) {
                    Ok(payload) => payload,
                    Err(err) => {
                        error!(error = ?err, "Failed to deserialize message payload, discarding message");
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
                        if let Err(err) = delivery.ack(lapin::options::BasicAckOptions::default()).await {
                            error!(error = ?err, "Failed to ack message");
                        }
                    }
                    Err(err) => {
                        error!(error = ?err, "Failed to process message");
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
