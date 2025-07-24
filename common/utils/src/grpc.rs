use std::time::Duration;
use tracing::{Level, event, instrument};

// TODO: Change return type to Result
#[instrument(name = "gRPC connection", skip_all)]
pub async fn try_get_grpc_client<C, CFut, CFn>(
    connect_fn: CFn,
    addr: &str,
    interval: Duration,
) -> Option<C>
where
    CFn: Fn(String) -> CFut,
    CFut: Future<Output = Result<C, tonic::transport::Error>>,
{
    event!(
        Level::DEBUG,
        "Connection process to gRPC service on {} initiated",
        addr
    );

    let connection_job = async {
        let mut count = 1;
        loop {
            event!(Level::TRACE, "Connection attempt {}...", count,);

            match connect_fn(addr.to_string()).await {
                Ok(client) => {
                    event!(Level::DEBUG, "Connection to gRPC service established");
                    return client;
                }
                Err(_) => {
                    count += 1;
                    event!(
                        Level::TRACE,
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
            event!(Level::DEBUG, "Shutdown signal received, aborting connection to gRPC service...");
            None
        },
        res = connection_job => {Some(res)},
    }
}
