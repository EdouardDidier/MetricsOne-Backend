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
use tracing::{debug, error, info, info_span};
use tracing_actix_web::TracingLogger;

use metrics_one_grpc::proto::insert_service_server::InsertServiceServer;
use metrics_one_utils::utils;

pub struct AppState {
    db: Arc<Pool<Postgres>>,
    rabbitmq: Arc<lapin::Channel>,
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let _otel_guard =
        metrics_one_utils::otel::init_tracing_subscriber("metrics-one-api", &ENV.rust_log);

    // Connection to PostgreSQL database
    let db_pool = {
        let _span = info_span!("Database connection").entered();

        let addr = format!("{}:{}", ENV.db.host, ENV.db.port);
        debug!("Connection to database on postgresql://{} initiated", addr);

        let db_pool = Arc::new(
            PgPoolOptions::new()
                .max_connections(ENV.db.pool_max_size)
                .connect(&format!(
                    "postgresql://{}:{}@{}/{}",
                    ENV.db.user, ENV.db.password, addr, ENV.db.dbname
                ))
                .await
                .inspect_err(|err| {
                    error!(error = ?err, "Failed to build database connection pool");
                })?,
        );

        info!("Connection to database establised");

        db_pool
    };

    // Connection to RabbitMQ
    let rabbitmq_channel = {
        let _span = info_span!("RabbitMQ setup").entered();

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

        info!("RabbitMQ consumer setup completed",);

        channel
    };

    // Get shutdown signals for gRPC and HTTP servers graceful shutdown
    let shutdown_signal = utils::get_shutdown_signals();

    // Initializing gRPC service
    let grpc_server_handle = {
        let _span = info_span!("gRPC server setup").entered();

        let server_grpc_url = format!("{}:{}", ENV.server.grpc.host, ENV.server.grpc.port);
        let addr = server_grpc_url.parse().inspect_err(|_| {
            error!(address = ?server_grpc_url, "Failed to parse server address");
        })?;

        let insert_service = InsertServiceHandler {
            db: db_pool.clone(),
        };

        // Starting gRPC service
        info!("gRPC service listening on http://{}", addr);

        tokio::spawn(
            Server::builder()
                .add_service(InsertServiceServer::new(insert_service))
                .serve_with_shutdown(addr, shutdown_signal),
        )
    };

    // Starting HTTP service
    let http_server_handle = {
        let _span = info_span!("HTTP server setup").entered();

        let addr = format!("{}:{}", ENV.server.http.host, ENV.server.http.port);
        info!("HTTP service listening on http://{}", addr);

        tokio::spawn(
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
                    .wrap(TracingLogger::default())
                    .service(services::http::fetch_drivers)
                    .service(services::http::fetch_driver_by_name)
                    .service(services::http::fetch_teams)
                    .service(services::http::fetch_team_by_name)
                    .service(services::http::fetch_meetings)
                    .service(services::http::fetch_sessions)
            })
            .bind(addr.clone())
            .inspect_err(|err| {
                error!(error = ?err, "Error while binding server to address {}", addr);
            })?
            .run(),
        )
    };

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
