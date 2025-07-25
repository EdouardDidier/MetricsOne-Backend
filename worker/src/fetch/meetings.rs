use metrics_one_grpc::proto::{InsertMeetingsRequest, insert_service_client::InsertServiceClient};
use tonic::{service::interceptor::InterceptedService, transport::Channel};
use tracing::{debug, info, instrument, trace};

use crate::{models::Meetings, settings::ENV};

#[instrument(name = "[Job] Fetch Meetings", skip_all, err)]
pub async fn fetch_job<F>(
    mut api_client: InsertServiceClient<InterceptedService<Channel, F>>,
    params: metrics_one_queue::models::Meetings,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: tonic::service::Interceptor,
{
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
    let meetings: Meetings = serde_json::from_str(text.trim_start_matches('\u{feff}').trim())?;

    // Prepare meetings to be sent to API service for insertion
    let mut response: InsertMeetingsRequest = meetings.into();
    trace!("Data parsed in {:?}", time.elapsed());

    response.meetings.retain(|m| !params.keys.contains(&m.key));
    trace!("Data processed in {:?}", time.elapsed());

    let nb_new_entry = response.meetings.len();
    if nb_new_entry == 0 {
        info!("No new entry found");
        return Ok(());
    }

    //Send request for processing to API
    trace!("Send {} new entries to API for insertion", nb_new_entry);
    api_client.insert_meetings(response).await?; // TODO: Handle error

    info!(
        "{} new entries fetched and processed by API service sucessfully in {:?}",
        nb_new_entry,
        time.elapsed(),
    );

    Ok(())
}
