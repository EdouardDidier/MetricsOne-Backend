use std::str::FromStr;

use metrics_one_grpc::proto::{InsertMeetingsRequest, insert_service_client::InsertServiceClient};
use opentelemetry::global;
use tracing::{Span, debug, info, instrument, trace};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{models::Meetings, settings::ENV};

// TODO: Move to common crate and move to traditional struct
struct MetadataMapInjector<'a>(&'a mut tonic::metadata::MetadataMap);

impl opentelemetry::propagation::Injector for MetadataMapInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = tonic::metadata::MetadataKey::from_str(key) {
            if let Ok(val) = tonic::metadata::MetadataValue::try_from(&value) {
                self.0.insert(key, val);
            }
        }
    }
}

#[instrument(name = "[Job] Fetch Meetings", skip_all, err)]
pub async fn fetch_job(
    mut api_client: InsertServiceClient<tonic::transport::Channel>,
    params: metrics_one_queue::models::Meetings,
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

    let meetings: InsertMeetingsRequest = meetings_response.into();
    trace!("Data parsed in {:?}", time.elapsed());

    // Prepare meetings to be sent to API service for insertion
    let mut request = tonic::Request::new(meetings);

    // Attach Trace context to the request
    let context = Span::current().context();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut MetadataMapInjector(request.metadata_mut()))
    });

    request
        .get_mut()
        .meetings
        .retain(|m| !params.keys.contains(&m.key));
    trace!("Data processed in {:?}", time.elapsed());

    let nb_new_entry = request.get_ref().meetings.len();
    if nb_new_entry == 0 {
        info!("No new entry found");
        return Ok(());
    }

    //Send request for processing to API
    trace!("Send {} new entries to API for insertion", nb_new_entry);
    api_client.insert_meetings(request).await?; // TODO: Handle error

    info!(
        "{} new entries fetched and processed by API service sucessfully in {:?}",
        nb_new_entry,
        time.elapsed(),
    );

    Ok(())
}
