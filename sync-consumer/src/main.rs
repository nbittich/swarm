/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use anyhow::{Context, anyhow};
use async_compression::tokio::bufread::GzipDecoder;
use async_zip::base::read::seek::ZipFileReader;
use chrono::Local;
use cron::Schedule;
use itertools::Itertools;
use reqwest::{
    Client,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use sparql_client::{SparqlClient, SparqlUpdateType, TARGET_GRAPH};
use std::{
    borrow::Cow,
    collections::HashMap,
    env::var,
    io::Cursor,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use swarm_common::{
    AsyncReadExt, IdGenerator,
    constant::{APPLICATION_NAME, CHUNK_SIZE, ROOT_OUTPUT_DIR, XSD},
    domain::{AuthBody, AuthPayload, Task, TaskResult},
    error, info, json, setup_tracing, warn,
};
use tokio::{io::BufReader, task::JoinSet};
use tortank::turtle::turtle_doc::{Node, RdfJsonTriple, Statement, TurtleDoc};
use xxhash_rust::xxh3::xxh3_64;

const ENABLE_INITIAL_SYNC: &str = "ENABLE_INITIAL_SYNC";
const CRON_EXPRESSION: &str = "CRON_EXPRESSION";
const SWARM_BASE_URL: &str = "SWARM_BASE_URL";
const SWARM_USERNAME: &str = "SWARM_USERNAME";
const SWARM_PASSWORD: &str = "SWARM_PASSWORD";
const SWARM_GRAPH: &str = "SWARM_GRAPH";
const DELTA_ENDPOINT: &str = "DELTA_ENDPOINT";
const ENABLE_DELTA_PUSH: &str = "ENABLE_DELTA_PUSH";
const DELETE_FILES: &str = "DELETE_FILES";
const HEAP_SIZE_MB: &str = "HEAP_SIZE_MB";
const DELTA_BUFFER_SLOT_CAP: &str = "DELTA_BUFFER_SLOT_CAP";
const DELTA_SLEEP_MS: &str = "DELTA_SLEEP_MS";

#[derive(Debug, Clone)]
struct Config {
    enable_initial_sync: bool,
    schedule: Schedule,
    sparql_client: SparqlClient,
    swarm_base_url: Arc<String>,
    chunk_size: usize,
    delta_sleep_ms: u64,
    swarm_client: Client,
    swarm_graph: Arc<String>,
    delta_endpoint: Arc<String>,
    target_graph: Arc<String>,
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
    let delta_sleep_ms = var(DELTA_SLEEP_MS)
        .unwrap_or_else(|_| "100".into())
        .parse::<u64>()?;
    let schedule = var(CRON_EXPRESSION)
        .map(|c| cron::Schedule::from_str(&c))
        .unwrap_or_else(|_| cron::Schedule::from_str("0 * * * * * *"))?;
    let target_graph = Arc::new(var(TARGET_GRAPH)?);

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

    let swarm_graph = Arc::new(
        var(SWARM_GRAPH)
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "http://bittich.be/graphs/swarm-consumer".into()),
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
        delta_sleep_ms,
        target_graph,
        swarm_graph,
        swarm_client,
        delta_endpoint: if enable_delta_push {
            if let Some(delta_endpoint) = delta_endpoint {
                Arc::new(delta_endpoint)
            } else {
                return Err(anyhow!("missing delta endpoint"));
            }
        } else {
            Arc::new("".into())
        },
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

    // allocate a large chunk of memory to reduce allocations
    // when reading ttl files
    let mut buffer = String::with_capacity(
        var(HEAP_SIZE_MB)
            .ok()
            .and_then(|h| h.parse::<usize>().ok())
            .unwrap_or(50)
            * 1024
            * 1024,
    ); // 500mb

    // allocate a large chunk of memory to reduce allocation
    // when accumulating delta
    let mut delta_buffer: HashMap<u64, RdfJsonTriple> = if config.enable_delta_push {
        HashMap::with_capacity(
            var(DELTA_BUFFER_SLOT_CAP)
                .ok()
                .and_then(|h| h.parse::<usize>().ok())
                .unwrap_or(32_768),
        )
    } else {
        HashMap::with_capacity(0)
    };

    info!("getting the current state...");
    let mut current_state = match get_state(&config).await {
        Ok(cs) => cs,
        Err(e) => {
            error!("Error while getting the current state: {e}");
            return Err(e);
        }
    };
    if config.enable_initial_sync && !current_state.initial_sync_ran {
        let consumer_root_dir = config.root_output_dir.join(IdGenerator.get());
        match consume(
            &consumer_root_dir,
            &config,
            &mut buffer,
            &mut delta_buffer,
            true,
            &[],
        )
        .await
        {
            Ok(mut consumed_tasks) => {
                info!("initial sync done. sleeping for a while before starting cron schedule.");
                update_initial_sync(&config, true).await?;
                for ct in consumed_tasks.iter() {
                    add_consumed_task(&config, ct).await?;
                }
                current_state.consumed_task_ids.append(&mut consumed_tasks);
                tokio::time::sleep(Duration::from_secs(60)).await; // FIXME make it configurable
            }
            Err(e) => {
                error!("could not run initial sync! {e}. shutdown...");
                return Err(e);
            }
        }
    }
    info!("starting cron schedule");

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

        let consumer_root_dir = config.root_output_dir.join(IdGenerator.get());
        match consume(
            &consumer_root_dir,
            &config,
            &mut buffer,
            &mut delta_buffer,
            false,
            &current_state.consumed_task_ids,
        )
        .await
        {
            Ok(mut consumed_tasks) => {
                for ct in consumed_tasks.iter() {
                    add_consumed_task(&config, ct).await?;
                }
                current_state.consumed_task_ids.append(&mut consumed_tasks);
            }
            Err(e) => {
                error!("could not run delta sync! {e}. will try again during the next run...");
                if consumer_root_dir.exists() {
                    tokio::fs::remove_dir_all(&consumer_root_dir).await?;
                }
            }
        }
    }

    info!("closing service...BYE");
    Ok(())
}

// optimization for mu-search
async fn flush_delta(
    config: &Config,
    operation: sparql_client::SparqlUpdateType,
    delta: &mut HashMap<u64, RdfJsonTriple>,
) -> anyhow::Result<()> {
    if delta.is_empty() {
        return Ok(());
    }
    let (_, delta): (Vec<u64>, Vec<RdfJsonTriple>) = delta.drain().unzip();

    info!(
        "sending delta message for operation {operation:?}. Len: {}",
        delta.len()
    );
    for chunk in delta.chunks(config.chunk_size) {
        let delta = chunk.to_vec();
        let payload = match operation {
            sparql_client::SparqlUpdateType::Insert => json! ([
                {"deletes": [], "inserts":delta}
            ]),
            sparql_client::SparqlUpdateType::Delete => json! ([
                {"deletes": delta, "inserts":[]}
            ]),
            sparql_client::SparqlUpdateType::NoOp => return Ok(()),
        };
        config
            .swarm_client
            .post(config.delta_endpoint.as_str())
            .json(&payload)
            .send()
            .await?;
        info!("delta push: sleep before sending next chunk");
        tokio::time::sleep(Duration::from_millis(config.delta_sleep_ms)).await;
    }

    Ok(())
}
async fn flush_triple_buffer(
    config: &Config,
    stmts: Vec<Statement<'_>>,
    operation: sparql_client::SparqlUpdateType,
    delta_buffer: &mut HashMap<u64, RdfJsonTriple>,
) -> anyhow::Result<()> {
    let stmts = stmts
        .into_iter()
        .map(|s| Statement {
            subject: s.subject,
            predicate: s.predicate,
            object: remove_datatype_xsd_string(s.object),
        })
        .collect::<Vec<_>>();
    config
        .sparql_client
        .bulk_update(
            &config.target_graph,
            &stmts
                .iter()
                .map(|stmt| stmt.to_string())
                .collect::<Vec<_>>(),
            operation,
        )
        .await?;

    if config.enable_delta_push {
        for stmt in stmts {
            let subject = stmt.subject.get_iri().map_err(|e| anyhow!("{e}"))?;

            let subject = xxh3_64(subject.as_bytes());
            delta_buffer
                .entry(subject)
                .or_insert_with(|| RdfJsonTriple::from(&stmt));
        }
    }
    Ok(())
}
async fn process_zip_file(
    config: &Config,
    buffer: &mut String,
    delta_buffer: &mut HashMap<u64, RdfJsonTriple>,
    dir: PathBuf,
    sut: SparqlUpdateType,
) -> anyhow::Result<()> {
    info!("processing {dir:?}");
    let mut read_dir = tokio::fs::read_dir(dir).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let zip_path = entry.path();
        info!("processing {sut:?} for {zip_path:?}");
        let mut zip_file = BufReader::new(tokio::fs::File::open(&zip_path).await?);
        let mut zip_reader = ZipFileReader::with_tokio(&mut zip_file).await?;
        for idx in 0..zip_reader.file().entries().len() {
            let mut entry_reader = zip_reader.reader_with_entry(idx).await?;
            info!(
                "processing ttl file {}",
                entry_reader.entry().filename().as_str()?
            );
            buffer.clear();
            if entry_reader.entry().uncompressed_size() == 0 {
                info!("entry seems to be empty, skipping,...");
                continue;
            }
            // it will always be gzipped, thus probably refactor this
            // get rid of the buffer that doesn't solve any problem anymore
            if entry_reader.entry().filename().as_str()?.ends_with(".gz") {
                use tokio::io::AsyncReadExt;
                let mut gz_buff =
                    Vec::with_capacity(entry_reader.entry().uncompressed_size() as usize);
                entry_reader.read_to_end(&mut gz_buff).await?;
                let cursor = Cursor::new(gz_buff);
                let mut reader = BufReader::new(cursor);
                let mut decoder = GzipDecoder::new(&mut reader);
                decoder.read_to_string(buffer).await?;
            } else {
                entry_reader.read_to_string(buffer).await?;
            }
            let doc =
                TurtleDoc::try_from((buffer.as_str(), None)).map_err(|e| anyhow::anyhow!("{e}"))?;
            for chunk in doc.into_iter().chunks(config.chunk_size).into_iter() {
                flush_triple_buffer(config, chunk.collect(), sut, delta_buffer).await?;
            }
        }
        flush_delta(config, sut, delta_buffer).await?;
    }
    Ok(())
}

async fn consume(
    consumer_root_dir: &Path,
    config: &Config,
    buffer: &mut String,
    delta_buffer: &mut HashMap<u64, RdfJsonTriple>,
    is_initial_sync: bool,
    consumed_tasks: &[String],
) -> anyhow::Result<Vec<String>> {
    let tasks: Vec<Task> = config
        .swarm_client
        .post(format!("{}/publications", config.swarm_base_url))
        .send()
        .await?
        .json::<Vec<Task>>()
        .await?
        .into_iter()
        .filter(|t| !consumed_tasks.contains(&t.id))
        .collect::<Vec<_>>();
    if tasks.is_empty() {
        info!("no new publication.");
        return Ok(vec![]);
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
                    ));
                }
            }
        }
    }

    // we start with the delete ones.
    if let Some(to_remove_dir) = to_remove_dir.take() {
        process_zip_file(
            config,
            buffer,
            delta_buffer,
            to_remove_dir,
            SparqlUpdateType::Delete,
        )
        .await?;
    }

    // we then process the new inserts
    process_zip_file(
        config,
        buffer,
        delta_buffer,
        new_inserts_dir,
        SparqlUpdateType::Insert,
    )
    .await?;

    // finally, the intersects if present
    // the type is insert, because if intersect needs to be consumed, we're probably in initial
    // sync
    if let Some(intersect_dir) = intersect_dir.take() {
        process_zip_file(
            config,
            buffer,
            delta_buffer,
            intersect_dir,
            SparqlUpdateType::Insert,
        )
        .await?;
    }
    // cleanup
    if config.delete_files {
        tokio::fs::remove_dir_all(consumer_root_dir).await?;
    }
    Ok(tasks.into_iter().map(|t| t.id).collect())
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

fn remove_datatype_xsd_string(mut term: Node<'_>) -> Node<'_> {
    match term {
        Node::Literal(tortank::turtle::turtle_doc::Literal::Quoted {
            ref mut datatype, ..
        }) => match datatype {
            Some(iri) => {
                if iri.as_ref() == &Node::Iri(Cow::Owned(XSD("string"))) {
                    *datatype = None;
                }
                term
            }
            _ => term,
        },
        Node::Ref(node) => {
            let node = &*node;
            remove_datatype_xsd_string(node.clone())
        }
        _ => term,
    }
}

#[derive(Debug)]
struct SyncConsumerState {
    initial_sync_ran: bool,
    consumed_task_ids: Vec<String>,
}

async fn add_consumed_task(config: &Config, task_id: &str) -> anyhow::Result<()> {
    let graph = &config.swarm_graph;
    let q = format!(
        r#"
        PREFIX ex: <http://example.org/schema#>
        PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>   
        INSERT DATA {{
            GRAPH <{graph}> {{
              ex:SwarmState a ex:State.
              ex:SwarmState ex:consumedTasks "{task_id}".
            }}
            
        }}
    "#
    );

    config.sparql_client.update(&q).await
}
async fn update_initial_sync(config: &Config, initial_sync: bool) -> anyhow::Result<()> {
    let graph = &config.swarm_graph;
    let q = format!(
        r#"
        PREFIX ex: <http://example.org/schema#>
        DELETE WHERE{{
            GRAPH <{graph}> {{
              ex:SwarmState ?p ?o.
            }}
            
        }}
        ;
        INSERT DATA{{
            GRAPH <{graph}> {{
              ex:SwarmState a ex:State.
              ex:SwarmState ex:initialSync {initial_sync}.
            }}
            
        }}
    "#
    );

    config.sparql_client.update(&q).await
}
async fn get_state(config: &Config) -> anyhow::Result<SyncConsumerState> {
    let q = format!(
        r#"
        PREFIX ex: <http://example.org/schema#>
        SELECT distinct ?consumedTask WHERE {{
            GRAPH <{}> {{
                ?state a  ex:State;
                       ex:consumedTasks ?consumedTask.
                    
            }}
      }}
   "#,
        config.swarm_graph
    );
    let res = config.sparql_client.query(&q).await?;

    let mut consumed_task_ids = Vec::with_capacity(res.results.bindings.len());
    for mut binding in res.results.bindings {
        if let Some(consumer_task_id) = binding.remove("consumedTask").map(|b| b.value) {
            consumed_task_ids.push(consumer_task_id);
        }
    }
    let q = format!(
        r#"
        PREFIX ex: <http://example.org/schema#>
        SELECT distinct ?initialSync WHERE {{
            GRAPH <{}> {{
                ?state a  ex:State;
                       ex:initialSync ?initialSync.
                    
            }}
      }}
   "#,
        config.swarm_graph
    );
    let res = config.sparql_client.query(&q).await?;

    let initial_sync_ran = if res.results.bindings.is_empty() {
        false
    } else {
        res.results.bindings[0]["initialSync"]
            .value
            .parse::<bool>()
            .unwrap_or(false)
    };
    Ok(SyncConsumerState {
        initial_sync_ran,
        consumed_task_ids,
    })
}
