/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;
//
mod config;
use anyhow::anyhow;
use chrono::Local;
use config::{INDEX_ID_KEY, IndexConfiguration};
use itertools::Itertools;
use meilisearch_sdk::client::Client as MeiliSearchClient;
use serde_json::Value;
use sparql_client::{SparqlClient, SparqlUpdateType};
use std::{
    borrow::Cow, collections::HashMap, env::var, path::PathBuf, str::FromStr, sync::Arc,
    time::Duration,
};
use swarm_common::{
    StreamExt,
    constant::{
        APPLICATION_NAME, INDEX_CONFIG_PATH, INDEX_CONSUMER, MEILISEARCH_KEY, MEILISEARCH_URL,
        SUB_TASK_EVENT_STREAM, SUB_TASK_STATUS_CHANGE_SUBJECT, TASK_EVENT_STREAM,
        TASK_STATUS_CHANGE_EVENT, TASK_STATUS_CHANGE_SUBJECT, UUID_COMPLEMENT_PREDICATE, XSD,
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
                            payload: Some(TaskResult::Publish { .. }),
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
        payload:
            Some(TaskResult::Publish {
                diff_manifest_file_path,
                ..
            }),
        ..
    } = &task.payload
    {
        if task.output_dir.exists() {
            tokio::fs::remove_dir_all(&task.output_dir).await?;
        }
        tokio::fs::create_dir_all(&task.output_dir).await?;
        let mut manifest =
            tokio::io::BufReader::new(tokio::fs::File::open(diff_manifest_file_path).await?)
                .lines();
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
                    errors.push(format!("error during indexing!  error: {e:?}"));
                }
                _ => {}
            }
        }

        task.modified_date = Some(Local::now());

        task.result = None; // FIXME, just pure laziness, at least a manifest with the meilisearch task uid
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

    if let Some(to_remove) = payload.to_remove_path.as_ref() {
        let to_remove = to_remove.clone();
        let config = config.clone();
        update(config, to_remove, SparqlUpdateType::Delete).await?;
    }
    if let Some(to_insert) = payload.new_insert_path.as_ref() {
        let to_insert = to_insert.clone();
        let config = config.clone();
        update(config, to_insert, SparqlUpdateType::Insert).await?;
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
                    .flat_map(|u| {
                        doc.list_statements(
                            Some(&u.subject),
                            Some(&Node::Iri(Cow::Borrowed(&config.uuid_predicate))),
                            None,
                        )
                    })
                    .map(|e| e.object.clone())
                    .map(|o| remove_datatype_xsd_string(o))
                    .map(|o| o.to_string().replace('"', ""))
                    .dedup()
                    .collect_vec();
                debug!(
                    "deleting the following documents in index {}: {uuids:?}",
                    ic.name
                );
                let task = config
                    .search_client
                    .index(&ic.name)
                    .delete_documents(&uuids)
                    .await?;
                debug!("{task:?}"); // FIXME this is just being lazy. it might hurt in the future,
                // as we don't really now if the meilisearch task succeeded once the job is already
                // set to success
            }
            SparqlUpdateType::Insert => {
                // inserting is a bit more work
                // we need to query sparql because
                // the document might need extra information from the triplestore
                // e.g: the name of the municipality
                let mut documents = Vec::with_capacity(updates.len());
                'sub: for subject in updates
                    .drain(..)
                    .map(|s| s.subject.to_string())
                    .dedup()
                    .collect_vec()
                {
                    let subject = &subject[1..subject.len() - 1]; // remove first and last character <url>
                    let mut doc_data = HashMap::new();

                    let uuid = {
                        let uuid_stmt = doc.list_statements(
                            Some(&Node::Iri(Cow::Borrowed(subject))),
                            Some(&Node::Iri(Cow::Borrowed(&config.uuid_predicate))),
                            None,
                        );
                        if uuid_stmt.is_empty() {
                            debug!("no uuid found in model for {subject}, skipping...");
                            None
                        } else {
                            uuid_stmt
                                .first()
                                .map(|e| e.object.clone())
                                .map(|o| remove_datatype_xsd_string(o))
                                .map(|o| o.to_string().replace('"', ""))
                        }
                    };
                    let Some(uuid) = uuid else {
                        continue 'sub;
                    };
                    doc_data.insert(
                        INDEX_ID_KEY.to_string(),
                        Value::from_str(&uuid).unwrap_or_else(|_| Value::String(uuid)),
                    );
                    for prop in ic.properties.iter() {
                        prop.validate()?;
                        let where_clause = prop.to_query_op(subject);
                        let query = format!(
                            r#"
                            SELECT ?{} WHERE {{
                                # TODO do we want to limit to specific graphs?
                                {where_clause}
                            }}
                        "#,
                            prop.name
                        );
                        let res = config.sparql_client.query(&query).await?;
                        if res.results.bindings.is_empty() && !prop.optional {
                            debug!(
                                "{} is not optional in {}. skipping indexing document {subject}",
                                prop.name, ic.name
                            );
                            continue 'sub;
                        }
                        let mut res = res
                            .results
                            .bindings
                            .into_iter()
                            .flat_map(|b| b.into_values())
                            .map(|b| {
                                Value::from_str(&b.value).unwrap_or_else(|_| Value::String(b.value))
                            })
                            .dedup()
                            .collect_vec();
                        if res.is_empty() {
                            continue;
                        }
                        if res.len() == 1 {
                            doc_data.insert(prop.name.clone(), res.remove(0));
                        } else {
                            doc_data.insert(prop.name.clone(), Value::Array(res));
                        }
                    }
                    documents.push(doc_data);
                }
                let task = config
                    .search_client
                    .index(&ic.name)
                    .add_or_update(&documents, Some(INDEX_ID_KEY))
                    .await?;
                debug!("{task:?}"); // FIXME this is just being lazy. it might hurt in the future,
                // as we don't really now if the meilisearch task succeeded once the job is already
                // set to success
            }
            SparqlUpdateType::NoOp => info!("index update: no op"),
        }
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
