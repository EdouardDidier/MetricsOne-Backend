mod services;
mod settings;

use std::{sync::Arc, time::Duration};

use actix_web::{App, HttpServer, web::Data};
use services::grpc::InsertServiceHandler;
use settings::ENV;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use tonic::transport::{Channel, Server};
use tracing::{debug, error, info};

use metrics_one_proto::proto::{
    fetch_service_client::FetchServiceClient, insert_service_server::InsertServiceServer,
};
use metrics_one_utils::{grpc::try_get_grpc_client, utils};

pub struct AppState {
    db: Arc<Pool<Postgres>>,
    worker: FetchServiceClient<Channel>,
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up logger
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(tracing_subscriber::EnvFilter::new(ENV.rust_log.clone()))
        .init();

    // Connection to PostgreSQL database
    let database_addr = format!("{}:{}", ENV.db.host, ENV.db.port);
    debug!(
        "Connection to database on postgresql://{} initiated",
        database_addr
    );

    let db_pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(ENV.db.pool_max_size)
            .connect(&format!(
                "postgresql://{}:{}@{}/{}",
                ENV.db.user, ENV.db.password, database_addr, ENV.db.dbname
            ))
            .await
            .inspect_err(|err| {
                error!(error = ?err, "Error occured while building database connection pool");
            })?,
    );

    info!("Connection to database establised");

    // Initializing gRPC service
    let shutdown_signal = utils::get_shutdown_signals();

    let server_grpc_url = format!("{}:{}", ENV.server.grpc.host, ENV.server.grpc.port);
    let server_grpc_addr = server_grpc_url.parse().inspect_err(|_| {
        error!(address = ?server_grpc_url, "Failed to parse server address");
    })?;

    let insert_service = InsertServiceHandler {
        db: db_pool.clone(),
    };

    // Starting gRPC service
    info!("gRPC service listening on http://{}", server_grpc_addr);

    let grpc_server_handle = tokio::spawn(async move {
        Server::builder()
            .add_service(InsertServiceServer::new(insert_service))
            .serve_with_shutdown(server_grpc_addr, shutdown_signal)
            .await
    });

    // Connection to worker with gRPC
    let worker_addr = format!("http://{}:{}", ENV.worker.host, ENV.worker.port);
    let worker_client = match try_get_grpc_client(
        FetchServiceClient::connect,
        &worker_addr,
        Duration::from_secs(1),
    )
    .await
    {
        Some(res) => {
            info!(
                "Connection to Worker service established on {}",
                worker_addr
            );
            res
        }
        None => {
            info!("No connection to API service, aborting server startup...");
            return Ok(());
        }
    };

    // Starting http service
    let server_http_addr = format!("{}:{}", ENV.server.http.host, ENV.server.http.port);

    info!("HTTP service listening on http://{}", server_http_addr);

    let http_server_handle = tokio::spawn(
        HttpServer::new(move || {
            App::new()
                .app_data(Data::new(AppState {
                    db: db_pool.clone(),
                    worker: worker_client.clone(),
                }))
                .service(services::http::fetch_drivers)
                .service(services::http::fetch_driver_by_name)
                .service(services::http::fetch_teams)
                .service(services::http::fetch_team_by_name)
                .service(services::http::fetch_meetings)
                .service(services::http::fetch_sessions)
        })
        .bind(server_http_addr.clone())
        .inspect_err(|err| {
            error!(error = ?err, "Error while binding server to address {}", server_http_addr);
        })?
        .run(),
    );

    // Waiting for both services to end
    let (http_result, grpc_result) = tokio::join!(http_server_handle, grpc_server_handle);

    info!("Shutdown signal received, shutting down the server...");

    // Catching error from either server
    http_result
        .inspect_err(|err| {
            error!(error = ?err, "Error from HTTP server");
        })?
        .inspect_err(|err| {
            error!(error = ?err, "Error from HTTP server");
        })?;

    grpc_result
        .inspect_err(|err| {
            error!(error = ?err, "Error from gRPC server");
        })?
        .inspect_err(|err| {
            error!(error = ?err, "Error from gRPC server");
        })?;

    Ok(())
}
