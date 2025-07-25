pub mod interceptor;
pub mod serde;
pub mod utils;

pub mod proto {
    tonic::include_proto!("fetch");
    tonic::include_proto!("insert");
}
