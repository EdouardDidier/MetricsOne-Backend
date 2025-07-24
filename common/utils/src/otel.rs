use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_sdk::{
    Resource,
    metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider},
    propagation::TraceContextPropagator,
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::{SCHEMA_URL, attribute::SERVICE_VERSION};
use tracing::error;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

// Guard to handle OpenTelemetry termination
// This will ensure that all telemetry is exported before it is dropped
pub struct OtelGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(err) = self.tracer_provider.shutdown() {
            error!(error = ?err, "Failed to shutdown tracer provider");
        }
        if let Err(err) = self.meter_provider.shutdown() {
            error!(error = ?err, "Failed to shutdown meter provider");
        }
    }
}

// Define a comprehensive set of information about the service to embed with the telemetry
fn resource(service_name: &'static str) -> Resource {
    Resource::builder()
        .with_service_name(service_name)
        .with_schema_url(
            vec![KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"))],
            SCHEMA_URL,
        )
        .build()
}

// Construct the MeterProvider for MetricsLayer
fn init_meter_provider(service_name: &'static str) -> SdkMeterProvider {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::default())
        .build()
        .expect("Failed to initialize OTLP exporter");

    let reader = PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource(service_name))
        .with_reader(reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    meter_provider
}

// Construct the TracerProvider for OpenTelemetryLaye
fn init_tracer_provider(service_name: &'static str) -> SdkTracerProvider {
    // Setup of OpenTelemetry - Traces
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to initialize OTLP exporter");

    SdkTracerProvider::builder()
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            1.0,
        ))))
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource(service_name))
        .with_batch_exporter(exporter)
        .build()
}

// Initialize Tracing Subscriber and return OtelGuard to handle OpenTelemetry termination
pub fn init_tracing_subscriber(service_name: &'static str, filter_level: &str) -> OtelGuard {
    let tracer_provider = init_tracer_provider(service_name);
    let meter_provider = init_meter_provider(service_name);

    // Setup propagator to use W3C Trace Context
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Use the service name as the instrumentation library name for better traceability
    let tracer = tracer_provider.tracer(service_name);

    // Register of Traces, Metrics and Logs subscribers
    let fmt_layer = tracing_subscriber::fmt::layer().compact();
    let filter = EnvFilter::new(filter_level);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(MetricsLayer::new(meter_provider.clone()))
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    OtelGuard {
        tracer_provider,
        meter_provider,
    }
}
