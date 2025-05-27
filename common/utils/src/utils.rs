use chrono::{Datelike, FixedOffset};
use serde::{Deserialize, Deserializer};
use tokio::signal;

#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};
use tracing::debug;

//TODO: Log on signal set up
pub async fn get_shutdown_signals() {
    let ctrl_c = signal::ctrl_c();

    #[cfg(unix)]
    let sigterm = async {
        let mut sigterm_stream =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        sigterm_stream.recv().await;
    };

    #[cfg(unix)]
    tokio::select! {
        _ = ctrl_c => {},
        _ = sigterm => {},
    }

    #[cfg(not(unix))]
    ctrl_c
        .await
        .expect("Failed to install CTRL+C signal handler")
}

pub fn deserialize_gmt_offset<'de, D>(deserializer: D) -> Result<FixedOffset, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s: &str = Deserialize::deserialize(deserializer)?;

    // Extract the sign and removing it
    let sign = if s.starts_with('-') { -1 } else { 1 };
    s = s.trim_start_matches(['+', '-']);

    // Split and convert the str into hours, minutes and seconds
    let hms: Vec<i32> = s
        .split(':')
        .map(|s| s.parse::<i32>().map_err(serde::de::Error::custom))
        .collect::<Result<_, _>>()?;

    if hms.len() != 3 {
        return Err(serde::de::Error::custom("Invalid offset format"));
    }

    // Convert to seconds
    let total_seconds = sign * (hms[0] * 3600 + hms[1] * 60 + hms[2]);

    // Create and return Offset
    FixedOffset::east_opt(total_seconds).ok_or(serde::de::Error::custom("Offset out of range"))
}

pub fn get_year(year: Option<i32>) -> i32 {
    match year {
        Some(res) => res,
        None => {
            let current_year = chrono::Utc::now().year();
            debug!("No year specified, defaulting to {}", current_year);
            current_year
        }
    }
}
