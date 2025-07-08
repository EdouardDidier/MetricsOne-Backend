mod fetch;
mod models;
mod settings;

use std::{sync::Arc, time::Duration};

use metrics_one_proto::proto::insert_service_client::InsertServiceClient;
use metrics_one_utils::{grpc::try_get_grpc_client, utils};
use settings::ENV;
use tokio_stream::StreamExt;
use tracing::{error, info, trace};

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
    let rabbitmq_addr = format!("{}:{}", ENV.rabbitmq.host, ENV.rabbitmq.port);
    let rabbitmq_channel = Arc::new(
        match metrics_one_queue::get_rabbitmq_channel(
            &rabbitmq_addr,
            &ENV.rabbitmq.user,
            &ENV.rabbitmq.password,
            &ENV.rabbitmq.queue,
        )
        .await
        {
            Ok(res) => {
                info!("Connection to RabbitMQ established on {}", rabbitmq_addr);
                res
            }
            Err(err) => {
                error!(error = ?err, "Failed to connect to RabbitMQ");
                return Err(err.into());
            }
        },
    );

    // Initializing RabbitMQ listenser
    // TODO: Create a class to handle multiple queues
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
                let payload: metrics_one_queue::models::Meetings = match serde_json::from_slice(&delivery.data) {
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
