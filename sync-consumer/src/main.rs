/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use anyhow::{anyhow, Context};
use async_compression::tokio::bufread::GzipDecoder;
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
    time::Duration,
};
use swarm_common::{
    constant::{APPLICATION_NAME, CHUNK_SIZE, ROOT_OUTPUT_DIR},
    domain::{AuthBody, AuthPayload, GetPublicationsPayload, Task, TaskResult},
    error, info, json, setup_tracing, warn, IdGenerator,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{io::BufReader, task::JoinSet};
use tortank::turtle::turtle_doc::{RdfJsonTriple, Statement, TurtleDoc};

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
    chunk_size: usize,
    swarm_client: Client,
    start_from_delta_timestamp: Option<DateTime<Utc>>,
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
    let chunk_size = var(CHUNK_SIZE)
        .unwrap_or_else(|_| "1024".into())
        .parse::<usize>()?;
    let schedule = var(CRON_EXPRESSION)
        .map(|c| cron::Schedule::from_str(&c))
        .unwrap_or_else(|_| cron::Schedule::from_str("0 * * * * * *"))?;
    let target_graph = var(TARGET_GRAPH)?;
    let start_from_delta_timestamp = var(START_FROM_DELTA_TIMESTAMP)
        .map(|d| {
            let d: DateTime<Local> = DateTime::from_str(&d).unwrap();
            d.to_utc()
        })
        .ok();
    let delta_endpoint = var(DELTA_ENDPOINT)
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty());
    let enable_delta_push = var(ENABLE_DELTA_PUSH)
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    let swarm_base_url = Arc::new(
        var(SWARM_BASE_URL)
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| !s.is_empty())
            .context("swarm base url empty or not present")?,
    );
    let swarm_username = var(SWARM_USERNAME)?;
    let swarm_password = var(SWARM_PASSWORD)?;

    let client = Client::builder().build()?;
    let response = client
        .post(format!("{swarm_base_url}/login"))
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
        chunk_size,
        target_graph,
        swarm_client,
        start_from_delta_timestamp,
        delta_endpoint: if enable_delta_push {
            if let Some(delta_endpoint) = delta_endpoint {
                delta_endpoint
            } else {
                return Err(anyhow!("missing delta endpoint"));
            }
        } else {
            "".into()
        },
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

    // allocate a large chunk of memory to reduce allocations
    // when reading ttl files
    let mut buffer = String::with_capacity(500 * 1024 * 1024); // 500mb

    if config.enable_initial_sync {
        config.start_from_delta_timestamp.take();

        let consumer_root_dir = config.root_output_dir.join(IdGenerator.get());
        match consume(&consumer_root_dir, &config, &mut buffer, true).await {
            Ok(_) => {
                info!("initial sync done. sleeping for a while before starting cron schedule.");
                config.start_from_delta_timestamp = Some(Local::now().to_utc());
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
            Err(e) => {
                error!("could not run initial sync! {e}. shutdown...");
                return Err(e);
            }
        }
    }
    info!("starting cron schedule");
    config.start_from_delta_timestamp = if config.start_from_delta_timestamp.is_none() {
        Some(Local::now().to_utc())
    } else {
        config.start_from_delta_timestamp.take()
    };
    for next_schedule in config.schedule.upcoming(chrono::Local) {
        let now = Local::now();
        if now < next_schedule {
            let duration = next_schedule - now;
            info!(
                "sleeping {} hour(s) {} minute(s) {} second(s) before next run...",
                duration.num_hours(),
                duration.num_minutes() % 60,
                duration.num_seconds() % 3600 % 60
            );
            tokio::time::sleep(Duration::from_millis(duration.num_milliseconds() as u64)).await;
        }
        info!("running consumer sync at {next_schedule}");

        let consumer_root_dir = config.root_output_dir.join(IdGenerator.get());
        match consume(&consumer_root_dir, &config, &mut buffer, false).await {
            Ok(_) => {
                config.start_from_delta_timestamp = Some(Local::now().to_utc());
            }
            Err(e) => {
                error!("could not run delta sync! {e}. will try again during the next run...");
                // if consumer_root_dir.exists() {
                //     tokio::fs::remove_dir_all(&consumer_root_dir).await?;
                // }
            }
        }
    }

    info!("closing service...BYE");
    Ok(())
}

async fn flush_triple_buffer(
    config: &Config,
    is_initial_sync: bool,
    stmts: Vec<Statement<'_>>,
) -> anyhow::Result<()> {
    config
        .sparql_client
        .bulk_update(
            &config.target_graph,
            &stmts
                .iter()
                .map(|stmt| stmt.to_string())
                .collect::<Vec<_>>(),
            sparql_client::SparqlUpdateType::Delete,
        )
        .await?;

    if !is_initial_sync && config.enable_delta_push {
        let delta = stmts
            .iter()
            .map(Into::<RdfJsonTriple>::into)
            .collect::<Vec<_>>();
        config
            .swarm_client
            .post(&config.delta_endpoint)
            .json(&json! ([
                {"deletes": delta, "inserts":[]}
            ]))
            .send()
            .await?;
    }
    Ok(())
}
async fn consume(
    consumer_root_dir: &Path,
    config: &Config,
    buffer: &mut String,
    is_initial_sync: bool,
) -> anyhow::Result<()> {
    let tasks: Vec<Task> = config
        .swarm_client
        .post(format!("{}/publications", config.swarm_base_url))
        .json(&GetPublicationsPayload {
            since: config.start_from_delta_timestamp,
        })
        .send()
        .await?
        .json()
        .await?;

    if tasks.is_empty() {
        info!("no new publication.");
        return Ok(());
    }
    tokio::fs::create_dir(&consumer_root_dir).await?;

    // now the interesting bits. we can download the files in parallel
    // but we will insert/remove triple files one by one
    let new_inserts_dir = consumer_root_dir.join("new-inserts");
    tokio::fs::create_dir(&new_inserts_dir).await?;

    let mut to_remove_dir = if is_initial_sync {
        None
    } else {
        let trd = consumer_root_dir.join("to-remove");
        tokio::fs::create_dir(&trd).await?;
        Some(trd)
    };
    let mut intersect_dir = if !is_initial_sync {
        None
    } else {
        let trd = consumer_root_dir.join("intersects");
        tokio::fs::create_dir(&trd).await?;
        Some(trd)
    };

    // each tasks has a maximum of 2 downloads.
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

    // we start with the delete ones.
    if let Some(to_remove_dir) = to_remove_dir.take() {
        let mut read_dir = tokio::fs::read_dir(to_remove_dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            info!("processing deletes for {path:?}");
            read_ttl_file(&path, buffer).await?;

            let mut stmts = Vec::with_capacity(config.chunk_size);
            let mut rest = buffer.as_str();
            while !rest.trim().is_empty() {
                if let Some((remaining, stmt)) =
                    TurtleDoc::parse_ntriples_statement(rest).map_err(|e| anyhow!("{e}"))?
                {
                    stmts.push(stmt);
                    rest = remaining;
                    if stmts.len() == config.chunk_size {
                        flush_triple_buffer(config, is_initial_sync, stmts.drain(..).collect())
                            .await?;
                    }
                }
            }
            if !stmts.is_empty() {
                flush_triple_buffer(config, is_initial_sync, stmts).await?;
            }
        }
    }

    // we then process the new inserts
    let mut read_dir = tokio::fs::read_dir(new_inserts_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        info!("processing inserts for {path:?}");
        read_ttl_file(&path, buffer).await?;
        let mut stmts = Vec::with_capacity(config.chunk_size);
        let mut rest = buffer.as_str();
        while !rest.trim().is_empty() {
            if let Some((remaining, stmt)) =
                TurtleDoc::parse_ntriples_statement(rest).map_err(|e| anyhow!("{e}"))?
            {
                stmts.push(stmt);
                rest = remaining;
                if stmts.len() == config.chunk_size {
                    flush_triple_buffer(config, is_initial_sync, stmts.drain(..).collect()).await?;
                }
            }
        }
        if !stmts.is_empty() {
            flush_triple_buffer(config, is_initial_sync, stmts).await?;
        }
    }

    // finally, the intersects if present
    if let Some(intersect_dir) = intersect_dir.take() {
        let mut read_dir = tokio::fs::read_dir(intersect_dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            info!("processing intersects for {path:?}");
            read_ttl_file(&path, buffer).await?;
            let mut stmts = Vec::with_capacity(config.chunk_size);
            let mut rest = buffer.as_str();
            while !rest.trim().is_empty() {
                if let Some((remaining, stmt)) =
                    TurtleDoc::parse_ntriples_statement(rest).map_err(|e| anyhow!("{e}"))?
                {
                    stmts.push(stmt);
                    rest = remaining;
                    if stmts.len() == config.chunk_size {
                        flush_triple_buffer(config, is_initial_sync, stmts.drain(..).collect())
                            .await?;
                    }
                }
            }
            if !stmts.is_empty() {
                flush_triple_buffer(config, is_initial_sync, stmts).await?;
            }
        }
    }
    // cleanup
    if config.delete_files {
        tokio::fs::remove_dir_all(consumer_root_dir).await?;
    }
    Ok(())
}

async fn read_ttl_file(path: &Path, buffer: &mut String) -> anyhow::Result<()> {
    buffer.clear();
    let f = tokio::fs::File::open(&path).await?;
    let mut reader = BufReader::new(f);

    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| *ext == "gz")
        .is_some()
    {
        let mut decoder = GzipDecoder::new(reader);
        decoder.read_to_string(buffer).await?;
        decoder.shutdown().await?;
    } else {
        reader.read_to_string(buffer).await?;
        reader.shutdown().await?;
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
            swarm_client,
            &url,
            inserted_triple_file_path,
            &new_inserts_dir.join(
                inserted_triple_file_path
                    .file_name()
                    .context("no filename!")?,
            ),
        )
        .await?;

        if let Some(to_remove_dir) = to_remove_dir {
            download(
                swarm_client,
                &url,
                removed_triple_file_path,
                &to_remove_dir.join(
                    removed_triple_file_path
                        .file_name()
                        .context("no filename!")?,
                ),
            )
            .await?;
        }
        if let Some(intersect_dir) = intersect_dir {
            download(
                swarm_client,
                &url,
                intersect_triple_file_path,
                &intersect_dir.join(
                    intersect_triple_file_path
                        .file_name()
                        .context("no filename!")?,
                ),
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
    info!("download {download_path:?} in {local_path:?}");
    let mut resp = swarm_client
        .get(url)
        .query(&[("path", download_path)])
        .send()
        .await?;
    let mut f = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(local_path)
        .await?;
    while let Some(chunk) = resp.chunk().await? {
        tokio::io::copy(&mut chunk.as_ref(), &mut f).await?;
    }
    Ok(())
}
