use std::{fmt::Debug, sync::LazyLock, time::Duration};

use crate::{
    constant::{
        NATS_ACK_WAIT, NATS_CONNECTION_URL, NATS_MAX_RECONNECT, NATS_PASSWORD, NATS_USERNAME,
    },
    domain::JsonMapper,
};
pub use async_nats::jetstream::Message;
pub use async_nats::jetstream::consumer::PullConsumer;
pub use async_nats::jetstream::stream::Stream;
use async_nats::{
    Client,
    jetstream::{
        self, Context,
        consumer::{AckPolicy, pull::Config as ConsumerConfig},
        stream::{Config, DiscardPolicy},
    },
};
use tracing::instrument;

pub static NATS_WAIT_BEFORE_REDELIVERY: LazyLock<Duration> = LazyLock::new(|| {
    std::env::var(NATS_ACK_WAIT)
        .unwrap_or_else(|_| "600".to_string())
        .parse::<u64>()
        .map(Duration::from_secs)
        .unwrap_or_else(|_| panic!("missing {NATS_ACK_WAIT}"))
});

#[derive(Clone, Debug)]
pub struct NatsClient {
    pub client: Client,
    pub jetstream: Context,
}

pub async fn connect() -> anyhow::Result<NatsClient> {
    let url = std::env::var(NATS_CONNECTION_URL).unwrap_or_else(|_| "nats://localhost:4222".into());
    let mut options = async_nats::ConnectOptions::new()
        .max_reconnects(
            std::env::var(NATS_MAX_RECONNECT)
                .unwrap_or_else(|_| "5".into())
                .parse::<usize>()
                .ok(),
        )
        .retry_on_initial_connect();
    if let (Some(username), Some(password)) = (
        std::env::var(NATS_USERNAME).ok(),
        std::env::var(NATS_PASSWORD).ok(),
    ) {
        options = options.user_and_password(username, password);
    }
    let client = async_nats::connect_with_options(url, options).await?;
    let js = jetstream::new(client.clone());
    Ok(NatsClient {
        client,
        jetstream: js,
    })
}

impl NatsClient {
    pub async fn add_stream(&self, stream: &str, subjects: Vec<String>) -> anyhow::Result<Stream> {
        let stream = self
            .jetstream
            .get_or_create_stream(Config {
                name: stream.to_string(),
                subjects,
                max_messages: 100_000,
                discard: DiscardPolicy::Old,
                ..Default::default()
            })
            .await?;
        Ok(stream)
    }

    /* usage
     * https://github.com/nats-io/nats.rs/blob/main/async-nats/examples/jetstream_pull.rs
     */
    pub async fn create_durable_consumer(
        &self,
        consumer_name: &str,
        stream: &Stream,
    ) -> anyhow::Result<PullConsumer> {
        let consumer = stream
            .get_or_create_consumer(
                consumer_name,
                ConsumerConfig {
                    ack_wait: *NATS_WAIT_BEFORE_REDELIVERY,
                    durable_name: Some(consumer_name.to_string()),
                    name: Some(consumer_name.to_string()),
                    ack_policy: AckPolicy::Explicit,
                    ..Default::default()
                },
            )
            .await?;

        Ok(consumer)
    }
    #[instrument(level = "debug")]
    pub async fn publish(
        &self,
        subject: String,
        payload: &(impl JsonMapper + Debug),
    ) -> anyhow::Result<()> {
        let bytes = payload.serialize_bytes()?;
        self.jetstream.publish(subject, bytes.into()).await?; // do not wait for ack?
        Ok(())
    }
}
