use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Meetings {
    pub keys: Vec<i32>,
    pub year: i32,
}
