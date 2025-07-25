use chrono::{DateTime, TimeZone, Utc};
use prost_types::Timestamp;

/// Converts a `prost_types::Timestamp` to a `chrono::DateTime<Utc>`
pub fn timestamp_to_datetime(ts: &Timestamp) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    Utc.timestamp_opt(ts.seconds, ts.nanos as u32)
        .single()
        .ok_or("Failed to parse timestamp: Out of range".into())
}

/// Converts a `chrono::DateTime<Utc>` to a `prost_types::Timestamp`
pub fn datetime_to_timestamp(dt: &DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}
