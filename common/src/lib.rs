use std::sync::LazyLock;

use chrono::Local;
use tracing::Level;
use tracing_subscriber::{fmt::time::FormatTime, EnvFilter, FmtSubscriber};

struct LocalTime;

pub use tracing::{debug, error, info, trace, warn};
pub mod constant;
pub mod domain;
pub mod nats_client;
pub use futures::*;
// pub mod alloc;
pub mod mongo;

pub use serde_json::json;
impl FormatTime for LocalTime {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("%Y-%m-%d %H:%M:%S"))
    }
}
pub fn setup_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_timer(LocalTime)
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    info!("Application started");
}
pub struct IdGenerator;

impl IdGenerator {
    pub fn get(&self) -> String {
        uuid::Uuid::now_v7().to_string().replace("-", "")
    }
}
pub static REGEX_CLEAN_URL: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(";jsessionid=[a-zA-Z;0-9]*").expect("could not compile regex")
});
