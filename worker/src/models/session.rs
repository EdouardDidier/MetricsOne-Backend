use prost_types::Timestamp;
use serde::{Deserialize, Serialize};

use metrics_one_proto::serde::{gmt_offset, timestamp};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Session {
    pub key: i32,

    #[serde(rename(deserialize = "Type"))]
    pub kind: String,

    pub name: String,

    #[serde(with = "timestamp")]
    pub start_date: Option<Timestamp>,

    #[serde(with = "timestamp")]
    pub end_date: Option<Timestamp>,

    #[serde(with = "gmt_offset")]
    pub gmt_offset: i64,

    pub path: String,
}
