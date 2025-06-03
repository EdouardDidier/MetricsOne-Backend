use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S>(seconds: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let sign = if *seconds < 0 { "-" } else { "" };

    let abs = (*seconds).abs();
    let hours = abs / 3600;
    let minutes = (abs % 3600) / 60;
    let secs = abs % 60;

    let formatted = format!("{sign}{:02}:{:02}:{:02}", hours, minutes, secs);

    serializer.serialize_str(&formatted)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    let sign = if s.starts_with('-') { -1 } else { 1 };

    let split: Vec<&str> = s.trim_start_matches('-').split(':').collect();

    if split.len() != 3 {
        return Err(serde::de::Error::custom(
            "Invalid time format, expected -hh:mm:ss",
        ));
    }

    let hours: i64 = split[0]
        .parse()
        .map_err(|_| serde::de::Error::custom("Invalid hours"))?;
    let minutes: i64 = split[1]
        .parse()
        .map_err(|_| serde::de::Error::custom("Invalid minutes"))?;
    let seconds: i64 = split[2]
        .parse()
        .map_err(|_| serde::de::Error::custom("Invalid seconds"))?;

    Ok(sign * (hours * 3600 + minutes * 60 + seconds))
}
