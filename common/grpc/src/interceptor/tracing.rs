use std::str::FromStr;

use opentelemetry::{
    global,
    propagation::{Extractor, Injector},
};
use tonic::{Request, Status};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

// TODO: Consider to move into a common and use Tower OtelCrate with metrics tracker

/* /////////////////////// */
/* //// gRPC Injector //// */
/* /////////////////////// */

struct MetadataMapInjector<'a>(&'a mut tonic::metadata::MetadataMap);

impl Injector for MetadataMapInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = tonic::metadata::MetadataKey::from_str(key) {
            if let Ok(val) = tonic::metadata::MetadataValue::try_from(&value) {
                self.0.insert(key, val);
            }
        }
    }
}

pub fn tracing_injector(mut request: Request<()>) -> Result<Request<()>, Status> {
    let context = tracing::Span::current().context();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut MetadataMapInjector(request.metadata_mut()))
    });

    Ok(request)
}

/* //////////////////////// */
/* //// gRPC Extractor //// */
/* //////////////////////// */

pub struct MetadataMapExtractor<'a>(pub &'a tonic::metadata::MetadataMap);

impl Extractor for MetadataMapExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0
            .keys()
            .map(|key| match key {
                tonic::metadata::KeyRef::Ascii(ascii_key) => ascii_key.as_str(),
                tonic::metadata::KeyRef::Binary(binary_key) => binary_key.as_str(),
            })
            .collect()
    }
}

// TODO: Move to tower, this implementation can't work with an interceptor
pub fn tracing_extractor(request: Request<()>) -> Result<Request<()>, Status> {
    let parent_cx = global::get_text_map_propagator(|propagator| {
        propagator.extract(&MetadataMapExtractor(request.metadata()))
    });

    Span::current().set_parent(parent_cx);

    Ok(request)
}
