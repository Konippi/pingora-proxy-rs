use tracing_subscriber::{EnvFilter, FmtSubscriber};

/// Register a subscriber for logging
pub fn register_subscriber() {
    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .expect("Setting default subscriber failed");
}
