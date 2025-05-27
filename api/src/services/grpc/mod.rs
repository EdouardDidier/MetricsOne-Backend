mod meetings;

use std::sync::Arc;

use metrics_one_proto::proto::{self, insert_service_server::InsertService};
use sqlx::{Pool, Postgres};

//TODO: Move to dedicated file
#[derive(Debug)]
pub struct InsertServiceHandler {
    pub db: Arc<Pool<Postgres>>,
}

#[tonic::async_trait()]
impl InsertService for InsertServiceHandler {
    async fn insert_meetings(
        &self,
        request: tonic::Request<proto::InsertMeetingsRequest>,
    ) -> Result<tonic::Response<proto::InsertMeetingsResponse>, tonic::Status> {
        meetings::insert(&self, request).await
    }
}
