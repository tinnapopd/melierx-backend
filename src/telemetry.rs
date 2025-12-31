use actix_web::rt::task::{JoinHandle, spawn_blocking};
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

/// Compose a tracing subscriber
/// # Arguments
/// * `name` - The name of the application
/// * `env_filter` - The environment filter string
/// * `sink` - The sink to write logs to
/// # Returns
/// A tracing subscriber instance
pub fn get_subscriber(
    name: String,
    env_filter: String,
    sink: impl for<'a> MakeWriter<'a> + Send + Sync + 'static,
) -> impl tracing::Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(
        name, // Output the formatted spans to stdout
        sink,
    );
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Initialize the tracing subscriber as global default
/// # Arguments
/// * `subscriber` - The tracing subscriber to set as global default
/// Returns nothing
pub fn init_subscriber(subscriber: impl tracing::Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger.");
    set_global_default(subscriber).expect("Failed to set subscriber.");
}

/// Spawns a blocking task with the current tracing span
/// # Arguments
/// * `f` - The blocking function to execute
/// # Returns
/// A JoinHandle to the spawned task
pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    spawn_blocking(move || current_span.in_scope(f))
}
