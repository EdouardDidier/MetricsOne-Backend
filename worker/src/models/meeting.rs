use metrics_one_proto::proto::{self, InsertMeetingsRequest};
use serde::{Deserialize, Serialize};

use crate::models::Session;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Meetings {
    pub year: i32,
    pub meetings: Vec<Meeting>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Meeting {
    pub key: i32,
    pub number: i32,
    pub location: String,
    pub official_name: String,
    pub name: String,
    pub sessions: Vec<Session>,
}

impl From<Meetings> for InsertMeetingsRequest {
    fn from(meetings: Meetings) -> Self {
        InsertMeetingsRequest {
            year: meetings.year,
            meetings: meetings
                .meetings
                .into_iter()
                .map(move |m| {
                    let sessions = m
                        .sessions
                        .into_iter()
                        .map(move |s| proto::insert_meetings_request::meeting::Session {
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
                        })
                        .collect();

                    proto::insert_meetings_request::Meeting {
                        key: m.key,
                        number: m.number,
                        location: m.location,
                        official_name: m.official_name,
                        name: m.name,
                        sessions,
                    }
                })
                .collect(),
        }
    }
}
