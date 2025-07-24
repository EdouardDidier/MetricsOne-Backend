use tracing::{debug, error, info, instrument};

pub mod models;

// TODO: Refactor into a class and split into different functions
#[instrument(name = "RabbitMQ connection", skip_all)]
pub async fn get_rabbitmq_channel(
    addr: &str,
    user: &str,
    password: &str,
    queue: &str,
) -> Result<lapin::Channel, lapin::Error> {
    debug!("Connection to RabbitMQ on amqp://{} initiated", addr);

    let connection = lapin::Connection::connect(
        &format!("amqp://{}:{}@{}/%2f", user, password, addr),
        lapin::ConnectionProperties::default(),
    )
    .await
    .inspect_err(|err| {
        error!(error = ?err, "Failed to connect to RabbitMQ");
    })?;

    info!("Connection to RabbitMQ establised");

    let channel = connection.create_channel().await.inspect_err(|err| {
        error!(error = ?err, "Failed to create RabbitMQ channel");
    })?;

    channel
        .queue_declare(
            queue,
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

    info!(queue = ?queue, "Queue declared");

    Ok(channel)
}
