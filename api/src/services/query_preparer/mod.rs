pub mod insert;
pub mod select;

#[derive(Clone)]
pub enum SqlType {
    Int(i32),
    Text(String),
}
