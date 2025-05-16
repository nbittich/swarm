use std::sync::LazyLock;

use chrono::Local;
use tracing::Level;
use tracing_subscriber::{EnvFilter, FmtSubscriber, fmt::time::FormatTime};

struct LocalTime;

pub use swarm_retryable_fut::retryable_fut;
pub use tracing::{debug, error, info, trace, warn};
pub mod constant;
pub mod domain;
pub mod nats_client;
pub use futures::*;
// pub mod alloc;
pub mod compress;
pub mod mongo;
pub mod retry_fs;
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

pub fn chunk_drain<T>(arr: &mut Vec<T>, chunk_size: usize) -> Vec<Vec<T>> {
    let mut chunks = Vec::new();

    while !arr.is_empty() {
        let chunk = arr.drain(..chunk_size.min(arr.len())).collect::<Vec<_>>();
        chunks.push(chunk);
    }
    chunks
}

pub static REGEX_CLEAN_JSESSIONID: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(";jsessionid=[a-zA-Z;0-9]*").expect("could not compile regex")
});
pub static REGEX_CLEAN_S_UUID: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"/\(S\([^)]+\)\)").expect("could not compile regex"));
