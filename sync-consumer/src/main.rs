/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use anyhow::anyhow;
use chrono::{DateTime, Local, Utc};
use cron::Schedule;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client,
};
use sparql_client::{SparqlClient, TARGET_GRAPH};
use std::{
    env::var,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};
use swarm_common::{
    constant::{APPLICATION_NAME, ROOT_OUTPUT_DIR},
    domain::{AuthBody, AuthPayload, GetPublicationsPayload, Task, TaskResult},
    error, info, setup_tracing, warn, IdGenerator,
};
use tokio::task::JoinSet;

const ENABLE_INITIAL_SYNC: &str = "ENABLE_INITIAL_SYNC";
const CRON_EXPRESSION: &str = "CRON_EXPRESSION";
const SWARM_BASE_URL: &str = "SWARM_BASE_URL";
const SWARM_USERNAME: &str = "SWARM_USERNAME";
const SWARM_PASSWORD: &str = "SWARM_PASSWORD";
const START_FROM_DELTA_TIMESTAMP: &str = "START_FROM_DELTA_TIMESTAMP";
const DELTA_ENDPOINT: &str = "DELTA_ENDPOINT";
const ENABLE_DELTA_PUSH: &str = "ENABLE_DELTA_PUSH";
const DELETE_FILES: &str = "DELETE_FILES";

#[derive(Debug, Clone)]
struct Config {
    enable_initial_sync: bool,
    schedule: Schedule,
    sparql_client: SparqlClient,
    swarm_base_url: Arc<String>,
    swarm_client: Client,
    start_from_delta_timestamp: DateTime<Utc>,
    delta_endpoint: String,
    target_graph: String,
    enable_delta_push: bool,
    root_output_dir: PathBuf,
    delete_files: bool,
}

async fn get_config() -> anyhow::Result<Config> {
    let enable_initial_sync = var(ENABLE_INITIAL_SYNC)
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    let delete_files = var(DELETE_FILES)
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(true);
    let schedule = var(CRON_EXPRESSION)
        .map(|c| cron::Schedule::from_str(&c))
        .unwrap_or_else(|_| cron::Schedule::from_str("0 * * * * * * *"))?;
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
    let swarm_base_url = Arc::new(var(SWARM_BASE_URL)?);
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
    let root_output_dir = std::env::var(ROOT_OUTPUT_DIR)
        .map(PathBuf::from)
        .unwrap_or_else(|_| "/share".into());
    if !root_output_dir.exists() {
        tokio::fs::create_dir_all(&root_output_dir).await?;
    }

    Ok(Config {
        enable_initial_sync,
        sparql_client,
        schedule,
        swarm_base_url,
        delete_files,
        root_output_dir,
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

    let mut config = get_config().await?;

    info!("config:\n{config:?}");

    info!("app {app_name} started and ready.");

    if config.enable_initial_sync {
        let consumer_root_dir = config.root_output_dir.join(IdGenerator.get());
        tokio::fs::create_dir(&consumer_root_dir).await?;
        match consume(&consumer_root_dir, &config, None, true).await {
            Ok(_) => info!("initial sync done."),
            Err(e) => {
                error!("could not run initial sync! {e}. cleanup then shutdown...");
                tokio::fs::remove_dir_all(&consumer_root_dir).await?;
                std::process::exit(1);
            }
        }
    }
    for next_schedule in config.schedule.upcoming(chrono::Local) {
        info!("running consumer sync at {next_schedule}");
    }

    info!("closing service...BYE");
    Ok(())
}

async fn consume(
    consumer_root_dir: &Path,
    config: &Config,
    since: Option<DateTime<Utc>>,
    is_initial_sync: bool,
) -> anyhow::Result<()> {
    let tasks: Vec<Task> = config
        .swarm_client
        .post(&format!("{}/publications", config.swarm_base_url))
        .json(&GetPublicationsPayload { since })
        .send()
        .await?
        .json()
        .await?;

    if tasks.is_empty() {
        info!("no new publications.");
        return Ok(());
    }

    // now the interesting bits. we can download the files in parallel
    // but we will insert triples one by one
    let new_inserts_dir = consumer_root_dir.join("new-inserts");
    tokio::fs::create_dir(&new_inserts_dir).await?;

    let to_remove_dir = if is_initial_sync {
        None
    } else {
        let trd = consumer_root_dir.join("new-inserts");
        tokio::fs::create_dir(&trd).await?;
        Some(trd)
    };
    let intersect_dir = if !is_initial_sync {
        None
    } else {
        let trd = consumer_root_dir.join("intersects");
        tokio::fs::create_dir(&trd).await?;
        Some(trd)
    };

    // each tasks has a maximum of 2 download.
    // To avoid spamming the download service,  we chunk the tasks by 5

    let mut download_tasks = JoinSet::new();
    for chunk in tasks.chunks(5).map(|c| c.to_vec()) {
        for task in chunk {
            let (base_url, swarm_client, new_inserts_dir, to_remove_dir, intersect_dir) = (
                config.swarm_base_url.clone(),
                config.swarm_client.clone(),
                new_inserts_dir.to_path_buf(),
                to_remove_dir.clone(),
                intersect_dir.clone(),
            );
            download_tasks.spawn(async move {
                match download_task(
                    &task,
                    &base_url,
                    &swarm_client,
                    &new_inserts_dir,
                    &to_remove_dir,
                    &intersect_dir,
                )
                .await
                {
                    Ok(_) => Ok(task),
                    Err(e) => Err((task, e)),
                }
            });
        }
        while let Some(handle) = download_tasks.join_next().await {
            match handle? {
                Ok(task) => info!("{} downloaded successfully.", task.id),
                Err((task, e)) => {
                    return Err(anyhow!(
                        "could not download task files. Error: {e}, task:\n {task:?}"
                    ))
                }
            }
        }
    }

    Ok(())
}

async fn download_task(
    task: &Task,
    base_url: &str,
    swarm_client: &Client,
    new_inserts_dir: &Path,
    to_remove_dir: &Option<PathBuf>,
    intersect_dir: &Option<PathBuf>,
) -> anyhow::Result<()> {
    if let Some(TaskResult::Publish {
        removed_triple_file_path,
        intersect_triple_file_path,
        inserted_triple_file_path,
        ..
    }) = &task.result
    {
        let url = format!("{base_url}/jobs/{}/download", &task.job_id);
        download(
            &swarm_client,
            &url,
            &inserted_triple_file_path,
            &new_inserts_dir.join(&inserted_triple_file_path),
        )
        .await?;

        if let Some(to_remove_dir) = to_remove_dir {
            download(
                &swarm_client,
                &url,
                &removed_triple_file_path,
                &to_remove_dir.join(&removed_triple_file_path),
            )
            .await?;
        }
        if let Some(intersect_dir) = intersect_dir {
            download(
                &swarm_client,
                &url,
                &intersect_triple_file_path,
                &intersect_dir.join(&intersect_triple_file_path),
            )
            .await?;
        }
    } else {
        warn!("task is not a publish task! {task:?}");
    }
    Ok(())
}

async fn download(
    swarm_client: &Client,
    url: &str,
    download_path: &Path,
    local_path: &Path,
) -> anyhow::Result<()> {
    let mut resp = swarm_client
        .get(url)
        .query(&[("path", download_path)])
        .send()
        .await?;
    let mut f = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(local_path)
        .await?;
    while let Some(chunk) = resp.chunk().await? {
        tokio::io::copy(&mut chunk.as_ref(), &mut f).await?;
    }
    Ok(())
}
