pub mod insert;
pub mod select;

use chrono::{DateTime, Utc};

#[derive(Clone)]
pub enum SqlType {
    Int(i32),
    Text(String),
    Timestamp(DateTime<Utc>),
}
