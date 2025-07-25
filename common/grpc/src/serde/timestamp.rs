use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S>(ts: &Option<Timestamp>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ts = ts.expect("Missing timestamp");

    let timestamp_millis = ts
        .seconds
        .checked_mul(1000)
        .and_then(|s| s.checked_add((ts.nanos / 1_000_000) as i64))
        .ok_or(serde::ser::Error::custom("Invalid timestamp components"))?;

    let datetime = DateTime::<Utc>::from_timestamp_millis(timestamp_millis)
        .ok_or(serde::ser::Error::custom("Invalid timestamp"))?;

    serializer.serialize_str(&datetime.to_rfc3339())
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Timestamp>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    // Format the string to the correct format
    let fixed = if !s.ends_with('Z') {
        format!("{}Z", s)
    } else {
        s
    };

    // Parse as chrono::DateTime<Utc>
    let dt = DateTime::parse_from_rfc3339(&fixed)
        .map_err(serde::de::Error::custom)?
        .with_timezone(&Utc);

    Ok(Some(Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }))
}
