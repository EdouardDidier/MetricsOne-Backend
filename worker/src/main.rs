mod grpc;
mod models;
mod settings;

use std::{sync::Arc, time::Duration};

use grpc::FetchServiceHandler;
use metrics_one_proto::proto::{
    fetch_service_server::FetchServiceServer, insert_service_client::InsertServiceClient,
};
use metrics_one_utils::{grpc::try_get_grpc_client, utils};
use settings::ENV;
use tonic::transport::Server;
use tracing::{error, info};

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

    // Initializing gRPC service
    let server_url = format!("{}:{}", ENV.server.host, ENV.server.port);
    let server_addr = server_url.parse().inspect_err(|_| {
        error!(address = ?server_url, "Failed to parse server address");
    })?;

    let fetch_service = FetchServiceHandler {
        api: Arc::new(api_client),
    };

    // Starting gRPC server
    info!("gRPC service listening on http://{}", server_addr);

    Server::builder()
        .add_service(FetchServiceServer::new(fetch_service))
        .serve_with_shutdown(server_addr, utils::get_shutdown_signals())
        .await
        .inspect_err(|err| {
            error!(error = ?err, "Server error");
        })?;

    info!("Shutdown signal received, shutting down the server...");

    Ok(())
}
