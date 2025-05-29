use metrics_one_proto::proto::{
    self, InsertMeetingsRequest, insert_service_client::InsertServiceClient,
};
use tonic::transport::Channel;
use tracing::{debug, info, instrument, trace};

use crate::{models::Meetings, settings::ENV};

use super::FetchServiceHandler;

#[instrument(name = "[gRPC Handler] Fetch Meetings", skip_all)]
pub async fn fetch(
    handler: &FetchServiceHandler,
    request: tonic::Request<proto::FetchMeetingsRequest>,
) -> Result<tonic::Response<proto::FetchMeetingsResponse>, tonic::Status> {
    let params = request.into_inner();

    debug!(parameter = ?params, "Request received with");

    let api_client = (*handler.api).clone();

    tokio::spawn(async move {
        let _ = fetch_job(api_client, params).await;
    });

    Ok(tonic::Response::new(proto::FetchMeetingsResponse {}))
}

#[instrument(name = "[Job] Fetch Meetings", skip_all, err)]
async fn fetch_job(
    mut api_client: InsertServiceClient<Channel>,
    params: proto::FetchMeetingsRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Fetch Meetings process initiated");
    let time = std::time::Instant::now();

    // Fetch data from Livetiming API
    let api_url = format!("{}/{}/{}", ENV.livetiming_url, params.year, "Index.json");

    debug!("Fetch data from {}", api_url);
    let res = reqwest::get(api_url).await?;

    // Check if API response is successful
    if !res.status().is_success() {
        return Err("Failed to fetch data from Livetiming API".into());
    }

    trace!("Data fetched in {:?}", time.elapsed());

    // Parse data from json
    let text = res.text().await?;
    let meetings_response: Meetings =
        serde_json::from_str(text.trim_start_matches('\u{feff}').trim())?;

    // TODO: convert start and end date witn offset in the 'into' implementation
    let mut meetings: InsertMeetingsRequest = meetings_response.into();

    trace!("Data parsed in {:?}", time.elapsed());

    // Prepare meetings to be sent to API service for insertion
    meetings.meetings.retain(|m| !params.keys.contains(&m.key));

    trace!("Data processed in {:?}", time.elapsed());

    let nb_new_entry = meetings.meetings.len();
    if nb_new_entry == 0 {
        info!("No new entry found");
        return Ok(());
    }

    //Send request for processing to API
    trace!("Send {} new entries to API for insertion", nb_new_entry);

    api_client.insert_meetings(meetings).await?;

    info!(
        "{} new entries fetched and processed by API service sucessfully in {:?}",
        nb_new_entry,
        time.elapsed(),
    );

    Ok(())
}
