pub mod timestamp_format;
pub mod utils;

pub mod proto {
    tonic::include_proto!("fetch");
    tonic::include_proto!("insert");
}
