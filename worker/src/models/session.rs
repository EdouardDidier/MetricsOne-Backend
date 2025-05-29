use prost_types::Timestamp;
use serde::{Deserialize, Serialize};

use metrics_one_proto::timestamp_format;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Session {
    pub key: i32,

    #[serde(rename(deserialize = "Type"))]
    pub kind: String,
    pub name: String,

    #[serde(with = "timestamp_format")]
    pub start_date: Option<Timestamp>,

    #[serde(with = "timestamp_format")]
    pub end_date: Option<Timestamp>,
    pub path: String,
}
