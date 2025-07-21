use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_sdk::{
    Resource,
    metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider},
    propagation::TraceContextPropagator,
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::{SCHEMA_URL, attribute::SERVICE_VERSION};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

fn resource(service_name: &'static str) -> Resource {
    Resource::builder()
        .with_service_name(service_name)
        .with_schema_url(
            [KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"))],
            SCHEMA_URL,
        )
        .build()
}

pub fn init_tracing_subscriber(
    service_name: &'static str,
    filter_level: &str,
) -> (SdkTracerProvider, SdkMeterProvider) {
    // Setup of OpenTelemetry - Traces
    let tracer_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to initialize OTLP exporter");

    let tracer_provider = SdkTracerProvider::builder()
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            1.0,
        ))))
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource(service_name))
        .with_batch_exporter(tracer_exporter)
        .build();

    // TODO: Check if the name is correct
    let tracer = tracer_provider.tracer("tracing-otel-subscriber");

    global::set_text_map_propagator(TraceContextPropagator::new());

    // Setup of OpenTelemetry - Metrics
    let meter_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::default())
        .build()
        .expect("Failed to initialize OTLP exporter");

    let reader = PeriodicReader::builder(meter_exporter)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource(service_name))
        .with_reader(reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    // Register of Traces, Metrics and Logs subscribers
    let fmt_layer = tracing_subscriber::fmt::layer().compact();
    let filter = EnvFilter::new(filter_level);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(OpenTelemetryLayer::new(tracer))
        .with(MetricsLayer::new(meter_provider.clone()))
        .init();

    (tracer_provider, meter_provider)
}
