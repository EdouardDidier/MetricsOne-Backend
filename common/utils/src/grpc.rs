use std::time::Duration;
use tracing::{debug, instrument, trace};

#[derive(Debug, Clone)]
pub struct ShutdownSignalError;

impl std::fmt::Display for ShutdownSignalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Shutdown signal received, aborting connection to gRPC service..."
        )
    }
}

impl std::error::Error for ShutdownSignalError {}

#[instrument(name = "gRPC connection", skip_all)]
pub async fn try_get_grpc_channel(
    addr: impl AsRef<str>,
    interval: Duration,
) -> Result<tonic::transport::Channel, Box<dyn std::error::Error>> {
    debug!(
        "Connection process to gRPC service on {} initiated",
        addr.as_ref().to_string()
    );

    let connection_job =
        async || -> Result<tonic::transport::Channel, Box<dyn std::error::Error>> {
            let endpoint = tonic::transport::Endpoint::from_shared(addr.as_ref().to_string())?;

            let mut count = 1;
            loop {
                trace!("Connection attempt {}...", count,);

                match endpoint.connect().await {
                    Ok(channel) => {
                        debug!("Connection to gRPC service established");
                        return Ok(channel);
                    }
                    Err(_) => {
                        count += 1;
                        trace!(
                            "Connection to gRPC service failed, retrying in {}s",
                            interval.as_secs()
                        );
                        tokio::time::sleep(interval).await;
                    }
                }
            }
        };

    tokio::select! {
        _ = crate::utils::get_shutdown_signals() => {
            Err(Box::new(ShutdownSignalError))
        },
        res = connection_job() => {res},
    }
}
