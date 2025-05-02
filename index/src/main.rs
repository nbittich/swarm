mod config;
use anyhow::anyhow;
/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;
use chrono::Local;
use config::IndexConfiguration;
use itertools::Itertools;
use meilisearch_sdk::client::Client as MeiliSearchClient;
use sparql_client::{SparqlClient, SparqlUpdateType};
use std::{borrow::Cow, env::var, path::PathBuf, sync::Arc, time::Duration};
use swarm_common::{
    StreamExt,
    constant::{
        APPLICATION_NAME, INDEX_CONFIG_PATH, INDEX_CONSUMER, MEILISEARCH_KEY, MEILISEARCH_URL,
        SUB_TASK_EVENT_STREAM, SUB_TASK_STATUS_CHANGE_SUBJECT, TASK_EVENT_STREAM,
        TASK_STATUS_CHANGE_EVENT, TASK_STATUS_CHANGE_SUBJECT, UUID_COMPLEMENT_PREDICATE,
    },
    debug,
    domain::{DiffResult, JsonMapper, Payload, Status, Task, TaskResult},
    error, info,
    nats_client::{self, NatsClient},
    setup_tracing,
};
use tokio::{io::AsyncBufReadExt, task::JoinSet};
use tortank::turtle::turtle_doc::{Node, Statement, TurtleDoc};

pub const NS_TYPE: Node = Node::Iri(Cow::Borrowed(
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
));

#[derive(Clone)]
struct Config {
    nc: NatsClient,
    sparql_client: SparqlClient,
    search_client: MeiliSearchClient,
    uuid_predicate: String,
    index_config: Arc<Vec<IndexConfiguration>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "index".into());
    let uuid_predicate = var(UUID_COMPLEMENT_PREDICATE)?;
    let meilisearch_url = var(MEILISEARCH_URL)?;
    let meilisearch_key = var(MEILISEARCH_KEY)?;
    let index_config_path = var(INDEX_CONFIG_PATH)?;

    let index_config = {
        info!("reading index config file {index_config_path}...");
        let config_str = tokio::fs::read_to_string(&index_config_path).await?;
        let ic: Vec<IndexConfiguration> = serde_json::from_str(&config_str)?;
        Arc::new(ic)
    };

    let nc = nats_client::connect().await?;
    let search_client = MeiliSearchClient::new(meilisearch_url, Some(meilisearch_key))?;

    while search_client.health().await.is_err() {
        error!("Meilisearch is not available yet. Sleeping for a sec before retrying");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

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
        .create_durable_consumer(INDEX_CONSUMER, &task_event_stream)
        .await?;

    let config = Config {
        nc,
        uuid_predicate,
        index_config,
        search_client,
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
        let mut errors = vec![];
        let mut tasks = JoinSet::new();
        while let Ok(Some(line)) = manifest.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            debug!("handling line {line}");

            let config = config.clone();
            tasks.spawn(async move { index(&line, &config).await });
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

        task.result = Some(TaskResult::Index);
        task.status = if errors.is_empty() {
            Status::Success
        } else {
            Status::Failed(errors)
        };
        return Ok(Some(()));
    }
    Ok(None)
}

async fn index(line: &str, config: &Config) -> anyhow::Result<()> {
    let payload = DiffResult::deserialize(line)?;

    let mut tasks = JoinSet::new();
    if let Some(to_remove) = payload.to_remove_path.as_ref() {
        let to_remove = to_remove.clone();
        let config = config.clone();
        tasks.spawn(async move { update(config, to_remove, SparqlUpdateType::Delete).await });
    }
    if let Some(to_insert) = payload.new_insert_path.as_ref() {
        let to_insert = to_insert.clone();
        let config = config.clone();
        tasks.spawn(async move { update(config, to_insert, SparqlUpdateType::Insert).await });
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
    update_type: SparqlUpdateType,
) -> anyhow::Result<()> {
    debug!("index {triples_path:?} with operation type {update_type:?}");

    let turtle_str = tokio::fs::read_to_string(&triples_path).await?;
    let doc = TurtleDoc::try_from((turtle_str.as_str(), None)).map_err(|e| anyhow!("{e}"))?;

    for ic in config.index_config.iter() {
        let mut updates: Vec<&Statement> = Vec::with_capacity(doc.len());
        // we first filter the subjects based on the rdf type
        for t in ic.rdf_type.iter() {
            debug!("handling {t} with op {update_type:?}");
            updates.extend(
                doc.list_statements(None, Some(&NS_TYPE), Some(&Node::Iri(Cow::Borrowed(t))))
                    .iter(),
            );
        }
        match update_type {
            SparqlUpdateType::Delete => {
                // we only need the uuid to delete
                let uuids = updates
                    .into_iter()
                    .map(|u| {
                        doc.list_statements(
                            Some(&u.subject),
                            Some(&Node::Iri(Cow::Borrowed(&config.uuid_predicate))),
                            None,
                        )
                    })
                    .flatten()
                    .map(|uuid| uuid.object.to_string().replace('"', ""))
                    .collect_vec();
                debug!(
                    "deleting the following documents in index {}: {uuids:?}",
                    ic.name
                );
                config
                    .search_client
                    .index(&ic.name)
                    .delete_documents(&uuids)
                    .await?;
            }
            SparqlUpdateType::Insert => {}
            SparqlUpdateType::NoOp => info!("index update: no op"),
        }
    }

    Ok(())
}
