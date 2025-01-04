/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use chrono::{DateTime, Local, Utc};
use cron::Schedule;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client,
};
use serde::{Deserialize, Serialize};
use sparql_client::{SparqlClient, TARGET_GRAPH};
use std::{env::var, str::FromStr};
use swarm_common::{
    constant::APPLICATION_NAME,
    domain::{AuthBody, AuthPayload},
    info, setup_tracing,
};

const INITIAL_SYNC: &str = "INITIAL_SYNC";
const CRON_EXPRESSION: &str = "CRON_EXPRESSION";
const SWARM_BASE_URL: &str = "SWARM_BASE_URL";
const SWARM_USERNAME: &str = "SWARM_USERNAME";
const SWARM_PASSWORD: &str = "SWARM_PASSWORD";
const START_FROM_DELTA_TIMESTAMP: &str = "START_FROM_DELTA_TIMESTAMP";
const DELTA_ENDPOINT: &str = "DELTA_ENDPOINT";
const ENABLE_DELTA_PUSH: &str = "ENABLE_DELTA_PUSH";
const CONSUMER_GRAPH: &str = "CONSUMER_GRAPH";

#[derive(Debug, Clone)]
struct Config {
    initial_sync: bool,
    schedule: Schedule,
    sparql_client: SparqlClient,
    swarm_base_url: String,
    swarm_client: Client,
    start_from_delta_timestamp: DateTime<Utc>,
    delta_endpoint: String,
    consumer_graph: String,
    target_graph: String,
    enable_delta_push: bool,
}

async fn get_config() -> anyhow::Result<Config> {
    let initial_sync = var(INITIAL_SYNC)
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    let schedule = var(CRON_EXPRESSION)
        .map(|c| cron::Schedule::from_str(&c))
        .unwrap_or_else(|_| cron::Schedule::from_str("0 * * * * * * *"))?;
    let consumer_graph = var(CONSUMER_GRAPH)?;
    let target_graph = var(TARGET_GRAPH)?;
    let start_from_delta_timestamp: DateTime<Utc> = var(START_FROM_DELTA_TIMESTAMP)
        .map(|d| {
            let d: DateTime<Local> = DateTime::from_str(&d).unwrap();
            d.to_utc()
        })
        .unwrap_or(Local::now().to_utc());
    let delta_endpoint = var(DELTA_ENDPOINT).unwrap_or("https://swarm.bittich.be".into());
    let enable_delta_push = var(ENABLE_DELTA_PUSH)
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    let swarm_base_url = var(SWARM_BASE_URL)?;
    let swarm_username = var(SWARM_USERNAME)?;
    let swarm_password = var(SWARM_PASSWORD)?;

    let client = Client::builder().build()?;
    let response = client
        .post(format!("{swarm_base_url}/authorize"))
        .json(&AuthPayload {
            username: swarm_username,
            password: swarm_password,
        })
        .send()
        .await?;
    let at: AuthBody = response.json().await?;
    let swarm_client = Client::builder()
        .default_headers(HeaderMap::from_iter(
            [(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", at.access_token)).unwrap(),
            )]
            .into_iter(),
        ))
        .build()?;
    let sparql_client = SparqlClient::new()?;

    Ok(Config {
        initial_sync,
        sparql_client,
        schedule,
        swarm_base_url,
        consumer_graph,
        target_graph,
        swarm_client,
        start_from_delta_timestamp,
        delta_endpoint,
        enable_delta_push,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "sync-consumer".into());

    let config = get_config().await?;

    info!("config:\n{config:?}");

    info!("app {app_name} started and ready.");

    for next_schedule in config.schedule.upcoming(chrono::Local) {
        info!("running consumer sync at {next_schedule}");
    }

    info!("closing service...BYE");
    Ok(())
}
