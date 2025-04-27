use anyhow::{Context, anyhow};
use async_zip::{Compression, ZipEntryBuilder, base::write::ZipFileWriter};
/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;
use chrono::Local;
use sparql_client::{SparqlClient, SparqlUpdateType, TARGET_GRAPH};
use std::{
    env::var,
    path::{Path, PathBuf},
    time::Duration,
};
use swarm_common::{
    IdGenerator, StreamExt,
    constant::{
        APPLICATION_NAME, CHUNK_SIZE, PUBLISH_CONSUMER, SUB_TASK_EVENT_STREAM,
        SUB_TASK_STATUS_CHANGE_SUBJECT, TASK_EVENT_STREAM, TASK_STATUS_CHANGE_EVENT,
        TASK_STATUS_CHANGE_SUBJECT,
    },
    debug,
    domain::{DiffResult, JsonMapper, Payload, Status, Task, TaskResult},
    error, info,
    nats_client::{self, NatsClient},
    setup_tracing,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    task::JoinSet,
};
use tortank::turtle::turtle_doc::TurtleDoc;

#[derive(Clone)]
struct Config {
    nc: NatsClient,
    sparql_client: SparqlClient,
    chunk_size: usize,
    target_graph: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "publish".into());
    let target_graph = var(TARGET_GRAPH)?;
    let chunk_size = var(CHUNK_SIZE)
        .unwrap_or_else(|_| "1024".into())
        .parse::<usize>()?;
    let nc = nats_client::connect().await?;

    let task_event_stream = nc
        .add_stream(
            TASK_EVENT_STREAM,
            vec![TASK_STATUS_CHANGE_SUBJECT.to_string()],
        )
        .await?;
    let _sub_task_event_stream = nc
        .add_stream(
            SUB_TASK_EVENT_STREAM,
            vec![SUB_TASK_STATUS_CHANGE_SUBJECT.to_string()],
        )
        .await?;
    let task_event_consumer = nc
        .create_durable_consumer(PUBLISH_CONSUMER, &task_event_stream)
        .await?;

    let config = Config {
        nc,
        chunk_size,
        target_graph,
        sparql_client: SparqlClient::new()?,
    };
    let mut messages = task_event_consumer.messages().await?;

    info!("app {app_name} started and ready to consume messages.");
    while let Some(message) = messages.next().await {
        match message {
            Ok(message) => match Task::deserialize_bytes(&message.payload) {
                Ok(mut task)
                    if matches!(
                        &task.payload,
                        Payload::FromPreviousStep {
                            payload: Some(TaskResult::Diff { .. }),
                            ..
                        }
                    ) && task.status == Status::Scheduled =>
                {
                    let config = config.clone();

                    tokio::spawn(async move {
                        if let Err(e) = message.ack().await {
                            error!("{e}");
                            return;
                        }
                        task.status = Status::Busy;
                        task.modified_date = Some(Local::now());
                        let _ = config
                            .nc
                            .publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task)
                            .await;
                        match handle_task(&config, &mut task).await {
                            Ok(Some(_)) => {
                                let _ = config
                                    .nc
                                    .publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task)
                                    .await;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                task.status =
                                    Status::Failed(vec![format!("unexpected error: {e}")]);
                                task.modified_date = Some(Local::now());
                                let _ = config
                                    .nc
                                    .publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task)
                                    .await;
                            }
                        }
                    });
                }
                Ok(task) => {
                    debug!("no op {task:?}");
                    message.ack().await.map_err(|e| anyhow::anyhow!("{e}"))?;
                }
                Err(e) => {
                    debug!("could not parse task! {e}");
                    message.ack().await.map_err(|e| anyhow::anyhow!("{e}"))?;
                }
            },
            Err(e) => error!("could not get message {e}"),
        }
    }
    info!("closing service...BYE");
    Ok(())
}

pub async fn append_to_file(path: &Path, s: &str) -> anyhow::Result<()> {
    let mut f = tokio::fs::File::options()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    f.write_all(s.as_bytes()).await?;
    Ok(())
}

pub async fn gzip_and_append_to_dir(dir: &Path, file: &Path) -> anyhow::Result<()> {
    let gzip_file = gzip(file).await?;
    let final_path = PathBuf::from(dir).join(gzip_file.file_name().context("no filename")?);
    tokio::fs::rename(gzip_file, final_path).await?;
    Ok(())
}
pub async fn gzip(path: &Path) -> anyhow::Result<PathBuf> {
    if !path.exists() {
        return Err(anyhow!("{path:?} doesn't exist. Cannot gzip it"));
    }
    use async_compression::tokio::write::GzipEncoder;
    let extension = path.extension().and_then(|ex| ex.to_str()).unwrap_or("");

    let gzip_path = path.with_extension(format!("{extension}.gz"));
    let input_file = tokio::fs::File::open(path).await?;
    let output_file = tokio::fs::File::create(&gzip_path).await?;
    let mut encoder = GzipEncoder::new(output_file);
    let mut buf = BufReader::new(input_file);
    tokio::io::copy_buf(&mut buf, &mut encoder).await?;

    encoder.shutdown().await?;
    Ok(gzip_path)
}
pub async fn zip(path: &Path) -> anyhow::Result<PathBuf> {
    let parent_dir = path.parent().context("zip: must have a parent dir")?;
    let zip_path = PathBuf::from(parent_dir).join(format!(
        "{}.zip",
        path.file_name().context("zip: filename")?.to_string_lossy()
    ));
    let mut zip = tokio::fs::File::create(&zip_path).await?;
    let mut writer = ZipFileWriter::with_tokio(&mut zip);
    let mut entries = tokio::fs::read_dir(&path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            return Err(anyhow!("zip: recursive not implemented."));
        } else {
            let mut file = tokio::fs::File::open(&path).await?;

            let builder = ZipEntryBuilder::new(
                path.file_name()
                    .context("zip entry: no filename {path:?}")?
                    .to_string_lossy()
                    .to_string()
                    .into(),
                Compression::Deflate,
            );
            let mut data = Vec::new();
            tokio::io::copy(&mut file, &mut tokio::io::BufWriter::new(&mut data)).await?;
            writer.write_entry_whole(builder, &data).await?;
        }
    }
    tokio::fs::remove_dir_all(&path).await?;
    Ok(zip_path)
}

async fn handle_task(config: &Config, task: &mut Task) -> anyhow::Result<Option<()>> {
    if let Payload::FromPreviousStep {
        payload: Some(TaskResult::Diff {
            manifest_file_path, ..
        }),
        ..
    } = &task.payload
    {
        if task.output_dir.exists() {
            tokio::fs::remove_dir_all(&task.output_dir).await?;
        }
        tokio::fs::create_dir_all(&task.output_dir).await?;
        let mut manifest =
            tokio::io::BufReader::new(tokio::fs::File::open(manifest_file_path).await?).lines();

        let removed_triple_file_path = task
            .output_dir
            .join(format!("removed-triples-{}", IdGenerator.get()));

        tokio::fs::create_dir_all(&removed_triple_file_path).await?;
        let inserted_triple_file_path = task
            .output_dir
            .join(format!("inserted-triples-{}", IdGenerator.get()));
        tokio::fs::create_dir_all(&inserted_triple_file_path).await?;
        // for e.g initial sync
        let intersect_triple_file_path = task
            .output_dir
            .join(format!("intersection-triples-{}", IdGenerator.get()));
        tokio::fs::create_dir_all(&intersect_triple_file_path).await?;
        // for debugging, maybe retrying

        let failed_query_file_path = task
            .output_dir
            .join(format!("failed-queries-{}.sparql", IdGenerator.get()));
        let mut errors = vec![];
        let mut tasks = JoinSet::new();
        while let Ok(Some(line)) = manifest.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            debug!("handling line {line}");

            let config = config.clone();
            let (
                removed_triple_file_path,
                inserted_triple_file_path,
                intersect_triple_file_path,
                failed_query_file_path,
            ) = (
                removed_triple_file_path.to_path_buf(),
                inserted_triple_file_path.to_path_buf(),
                intersect_triple_file_path.to_path_buf(),
                failed_query_file_path.to_path_buf(),
            );
            tasks.spawn(async move {
                publish(
                    &line,
                    &config,
                    &removed_triple_file_path,
                    &inserted_triple_file_path,
                    &intersect_triple_file_path,
                    &failed_query_file_path,
                )
                .await
            });
            // sleep for a while
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        while let Some(handle) = tasks.join_next().await {
            match handle.map_err(|e| anyhow!("{e}")) {
                Err(e) | Ok(Err(e)) => {
                    errors.push(format!("error during publishing!  error: {e:?}"));
                }
                _ => {}
            }
        }

        task.modified_date = Some(Local::now());

        task.result = Some(TaskResult::Publish {
            removed_triple_file_path: zip(&removed_triple_file_path).await?,
            intersect_triple_file_path: zip(&intersect_triple_file_path).await?,
            inserted_triple_file_path: zip(&inserted_triple_file_path).await?,
            failed_query_file_path: zip(&failed_query_file_path).await?,
        });
        task.status = if errors.is_empty() {
            Status::Success
        } else {
            Status::Failed(errors)
        };
        return Ok(Some(()));
    }
    Ok(None)
}

async fn publish(
    line: &str,
    config: &Config,
    removed_triple_file_path: &Path,
    inserted_triple_file_path: &Path,
    intersect_triple_file_path: &Path,
    failed_query_path: &Path,
) -> anyhow::Result<()> {
    let payload = DiffResult::deserialize(line)?;

    let mut tasks = JoinSet::new();
    if let Some(to_remove) = payload.to_remove_path.as_ref() {
        let to_remove = to_remove.clone();
        let (config, removed_triple_file_path, failed_query_path) = (
            config.clone(),
            removed_triple_file_path.to_path_buf(),
            failed_query_path.to_path_buf(),
        );
        tasks.spawn(async move {
            update(
                config,
                to_remove,
                &removed_triple_file_path,
                &failed_query_path,
                SparqlUpdateType::Delete,
            )
            .await
        });
    }
    if let Some(to_insert) = payload.new_insert_path.as_ref() {
        let to_insert = to_insert.clone();
        let (config, inserted_triple_file_path, failed_query_path) = (
            config.clone(),
            inserted_triple_file_path.to_path_buf(),
            failed_query_path.to_path_buf(),
        );
        tasks.spawn(async move {
            update(
                config,
                to_insert,
                &inserted_triple_file_path,
                &failed_query_path,
                SparqlUpdateType::Insert,
            )
            .await
        });
    }
    if let Some(intersect) = payload.intersect_path.as_ref() {
        let intersect = intersect.clone();
        let (config, intersect_triple_file_path, failed_query_path) = (
            config.clone(),
            intersect_triple_file_path.to_path_buf(),
            failed_query_path.to_path_buf(),
        );
        tasks.spawn(async move {
            update(
                config,
                intersect,
                &intersect_triple_file_path,
                &failed_query_path,
                SparqlUpdateType::NoOp,
            )
            .await
        });
    }
    debug!("running {} tasks", tasks.len());
    while let Some(handle) = tasks.join_next().await {
        handle??;
    }
    Ok(())
}

async fn update(
    config: Config,
    triples_path: PathBuf,
    append_to_file_path: &Path,
    query_error_path: &Path,
    update_type: SparqlUpdateType,
) -> anyhow::Result<()> {
    debug!("update {triples_path:?} with type {update_type:?} to {append_to_file_path:?}");

    let turtle_str = tokio::fs::read_to_string(&triples_path).await?;
    let doc = TurtleDoc::try_from((turtle_str.as_str(), None)).map_err(|e| anyhow!("{e}"))?;
    let mut chunk = Vec::with_capacity(config.chunk_size);
    let mut tasks = JoinSet::new();

    for stmt in doc {
        chunk.push(stmt.to_string());
        if chunk.len() == config.chunk_size {
            let chunk = std::mem::take(&mut chunk);
            let config = config.clone();
            tasks.spawn(async move {
                config
                    .sparql_client
                    .bulk_update(&config.target_graph, &chunk, update_type)
                    .await
            });
        }
    }
    if !chunk.is_empty() {
        let config = config.clone();
        tasks.spawn(async move {
            config
                .sparql_client
                .bulk_update(&config.target_graph, &chunk, update_type)
                .await
        });
    }
    while let Some(handle) = tasks.join_next().await {
        match handle? {
            Ok(_) => {}
            Err(failed_query) => {
                append_to_file(query_error_path, &format!("{failed_query}\n;\n")).await?
            }
        }
    }

    // copy file to future archive
    gzip_and_append_to_dir(append_to_file_path, &triples_path).await?;

    Ok(())
}
