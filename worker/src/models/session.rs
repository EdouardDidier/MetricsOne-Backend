use metrics_one_grpc::proto;
use prost_types::Timestamp;
use serde::{Deserialize, Serialize};

use metrics_one_grpc::serde::{gmt_offset, timestamp};

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

impl From<Session> for proto::insert_meetings_request::meeting::Session {
    fn from(s: Session) -> Self {
        proto::insert_meetings_request::meeting::Session {
            key: s.key,
            kind: s.kind,
            name: s.name,
            start_date: s.start_date.map(|mut t| {
                t.seconds -= s.gmt_offset;
                t
            }),
            end_date: s.end_date.map(|mut t| {
                t.seconds -= s.gmt_offset;
                t
            }),
            path: s.path,
        }
    }
}
