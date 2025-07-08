mod models;
mod services;
mod settings;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{App, HttpServer, web::Data};
use services::grpc::InsertServiceHandler;
use settings::ENV;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use tonic::transport::Server;
use tracing::{debug, error, info};

use metrics_one_proto::proto::insert_service_server::InsertServiceServer;
use metrics_one_utils::utils;

pub struct AppState {
    db: Arc<Pool<Postgres>>,
    rabbitmq: Arc<lapin::Channel>,
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
                error!(error = ?err, "Failed to build database connection pool");
            })?,
    );

    info!("Connection to database establised");

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

    info!("Connection to RabbitMQ establised");

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
            &ENV.rabbitmq.queue,
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

    // Starting http service
    let server_http_addr = format!("{}:{}", ENV.server.http.host, ENV.server.http.port);

    info!("HTTP service listening on http://{}", server_http_addr);

    let http_server_handle = tokio::spawn(
        HttpServer::new(move || {
            let cors = Cors::default()
                .allowed_origin("http://localhost:3000")
                .allowed_methods(vec!["GET", "POST", "OPTIONS"])
                .allowed_headers(vec!["Content-Type"])
                .supports_credentials();

            App::new()
                .app_data(Data::new(AppState {
                    db: db_pool.clone(),
                    rabbitmq: rabbitmq_channel.clone(),
                }))
                .wrap(cors)
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
