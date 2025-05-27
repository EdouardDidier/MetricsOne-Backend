mod meetings;

use std::sync::Arc;

use metrics_one_proto::proto::{
    self, fetch_service_server::FetchService, insert_service_client::InsertServiceClient,
};
use tonic::transport::Channel;

#[derive(Debug)]
pub struct FetchServiceHandler {
    pub api: Arc<InsertServiceClient<Channel>>,
}

#[tonic::async_trait]
impl FetchService for FetchServiceHandler {
    async fn fetch_meetings(
        &self,
        request: tonic::Request<proto::FetchMeetingsRequest>,
    ) -> Result<tonic::Response<proto::FetchMeetingsResponse>, tonic::Status> {
        meetings::fetch(&self, request).await
    }
}
