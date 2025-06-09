/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;
//
use anyhow::anyhow;
use chrono::{DateTime, Local, NaiveDateTime};
use itertools::Itertools;
use serde_json::{Number, Value};
use sparql_client::{SparqlClient, SparqlUpdateType};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::{borrow::Cow, env::var, path::PathBuf, str::FromStr, sync::Arc, time::Duration};
use swarm_common::compress::{gzip_str, ungzip};
use swarm_common::constant::{
    CHUNK_SIZE, INDEX_DELAY_BEFORE_NEXT_RETRY, INDEX_INTERVAL_WAIT_FOR_TASK, INDEX_MAX_RETRY,
    INDEX_MAX_TOTAL_HITS, INDEX_MAX_WAIT_FOR_TASK, RESET_INDEX, RESET_INDEX_NAME,
    SLEEP_BEFORE_NEXT_MEILISEARCH_BATCH, SLEEP_BEFORE_NEXT_TASK, SLEEP_BEFORE_NEXT_VIRTUOSO_QUERY,
};
use swarm_common::domain::index_config::{
    CONSTRUCT_PREFIX, INDEX_ID_KEY, IndexConfiguration, PREFIXES, SUBJECT_BINDING, VAR_BINDING,
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
use swarm_common::{chunk_drain, retry_fs, retryable_fut, trace};
use swarm_meilisearch_client::MeilisearchClient;
use swarm_meilisearch_client::domain::{ContentType, Encoding, PaginationSetting, TaskInfo};
use tokio::fs::File;
use tokio::io::{BufReader, Lines};
use tokio::{io::AsyncBufReadExt, task::JoinSet};
use tortank::turtle::turtle_doc::{Node, Statement, TurtleDoc};
use tortank::utils::{
    DATE_FORMATS, XSD_BOOLEAN, XSD_DATE, XSD_DATE_TIME, XSD_DECIMAL, XSD_DOUBLE, XSD_INTEGER,
};

pub const NS_TYPE: Node = Node::Iri(Cow::Borrowed(
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
));

#[derive(Debug)]
pub enum MeiliSearchUpdateType {
    Deletes(HashMap<String, HashSet<String>>), // idx => uuids
    Inserts(HashMap<String, HashSet<BTreeMap<String, Value>>>),
}
#[derive(Clone)]
struct Config {
    nc: NatsClient,
    sparql_client: SparqlClient,
    search_client: MeilisearchClient,
    uuid_predicate: String,
    index_config: Arc<Vec<IndexConfiguration>>,
    index_max_wait_for_task: Option<Duration>,
    index_interval_wait_for_task: Option<Duration>,
    sleep_before_next_virtuoso_query: Duration,
    sleep_before_next_task: Duration,
    sleep_before_next_meilisearch_task: Duration,
    index_max_retry: u64,
    index_delay_before_next_retry: u64,
    chunk_size: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "index".into());
    let uuid_predicate = var(UUID_COMPLEMENT_PREDICATE)?;
    let meilisearch_url = var(MEILISEARCH_URL)?;
    let meilisearch_key = var(MEILISEARCH_KEY)?;
    let index_config_path = var(INDEX_CONFIG_PATH)?;
    let chunk_size = var(CHUNK_SIZE) //INDEX_MAX_TOTAL_HITS
        .iter()
        .flat_map(|r| r.parse::<usize>())
        .last()
        .unwrap_or(255);
    let index_max_total_hits = var(INDEX_MAX_TOTAL_HITS)
        .iter()
        .flat_map(|r| r.parse::<usize>())
        .last()
        .unwrap_or(50_000);
    let reset_index = var(RESET_INDEX)
        .iter()
        .flat_map(|r| r.parse::<bool>())
        .last()
        .unwrap_or(false);

    let sleep_before_next_virtuoso_query = var(SLEEP_BEFORE_NEXT_VIRTUOSO_QUERY)
        .iter()
        .flat_map(|r| r.parse::<u64>())
        .last()
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_millis(30));

    let sleep_before_next_task = var(SLEEP_BEFORE_NEXT_TASK)
        .iter()
        .flat_map(|r| r.parse::<u64>())
        .last()
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_millis(30));
    let sleep_before_next_meilisearch_task = var(SLEEP_BEFORE_NEXT_MEILISEARCH_BATCH)
        .iter()
        .flat_map(|r| r.parse::<u64>())
        .last()
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_millis(60_000));

    let index_max_wait_for_task = var(INDEX_MAX_WAIT_FOR_TASK)
        .iter()
        .flat_map(|r| r.parse::<u64>())
        .last()
        .or(Some(3600))
        .map(Duration::from_secs);
    let index_interval_wait_for_task = var(INDEX_INTERVAL_WAIT_FOR_TASK)
        .iter()
        .flat_map(|r| r.parse::<u64>())
        .last()
        .or(Some(30))
        .map(Duration::from_secs);
    let index_max_retry = var(INDEX_MAX_RETRY)
        .iter()
        .flat_map(|r| r.parse::<u64>())
        .last()
        .unwrap_or(5);
    let index_delay_before_next_retry = var(INDEX_DELAY_BEFORE_NEXT_RETRY)
        .iter()
        .flat_map(|r| r.parse::<u64>())
        .last()
        .unwrap_or(30);
    let reset_index_name = var(RESET_INDEX_NAME).ok();

    let index_config = {
        info!("reading index config file {index_config_path}...");
        let config_str = retry_fs::read_to_string(&index_config_path).await?;
        let ic: Vec<IndexConfiguration> = serde_json::from_str(&config_str)?;
        for c in ic.iter() {
            for p in c.properties.iter() {
                p.validate()?;
            }
        }
        Arc::new(ic)
    };

    let nc = nats_client::connect().await?;
    let search_client = MeilisearchClient::new(meilisearch_url, meilisearch_key)?;

    while search_client.health().await.is_err() {
        error!("Meilisearch is not available yet. Sleeping for a sec before retrying");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // initialize the index with filterable attributes
    info!("initializing index with filterable attributes");
    for ic in index_config.iter() {
        search_client
            .set_filterable_attributes(&ic.name, ic.properties.iter().map(|p| &p.name))
            .await?;
        search_client
            .set_pagination(
                &ic.name,
                PaginationSetting {
                    max_total_hits: index_max_total_hits,
                },
            )
            .await?;
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
        index_interval_wait_for_task,
        chunk_size,
        index_max_retry,
        index_delay_before_next_retry,
        index_max_wait_for_task,
        sleep_before_next_task,
        sleep_before_next_virtuoso_query,
        sleep_before_next_meilisearch_task,
        sparql_client: SparqlClient::new()?,
    };

    if reset_index {
        info!(
            "reset set to true. processing...This will take some time, once finished, you will be asked to restart the service (you MUST unset the {RESET_INDEX} variable)"
        );
        for ic in config
            .index_config
            .iter()
            .filter(|ic| match reset_index_name.as_ref() {
                Some(index) => &ic.name == index,
                None => true,
            })
        {
            info!("reseting {}...", ic.name);
            let delete_task_info: TaskInfo = retryable_fut(
                config.index_max_retry,
                config.index_delay_before_next_retry,
                async || config.search_client.delete_all_documents(&ic.name).await,
            )
            .await?;
            info!("deleting. this might take a while. {delete_task_info:?}");
            retryable_fut(
                config.index_max_retry,
                config.index_delay_before_next_retry,
                async || {
                    config
                        .search_client
                        .wait_for_task(
                            delete_task_info.task_uid,
                            index_interval_wait_for_task,
                            index_max_wait_for_task,
                        )
                        .await
                },
            )
            .await?;
            info!("deleting done. Start reindexing...");
            let rdf_types = ic.rdf_type.iter().map(|t| format!("<{t}>")).join("\n");
            let mut res = config
                .sparql_client
                .query(format!(
                    r#"
                       SELECT DISTINCT ?s ?uuid WHERE {{
                            GRAPH ?g  {{
                                VALUES ?t {{
                                    {rdf_types}
                                }}
                                ?s a ?t;
                                   <{}> ?uuid.
                            }}
                       }}
            "#,
                    config.uuid_predicate
                ))
                .await?;
            info!("found {} subjects to reindex.", res.results.bindings.len());
            let mut documents = Vec::with_capacity(res.results.bindings.len());
            'sub: for binding in res
                .results
                .bindings
                .drain(..)
                .filter(|b| b.contains_key("s") && b.contains_key("uuid"))
            {
                let (subject, uuid) = (&binding["s"].value, &binding["uuid"].value);
                let mut doc_data = BTreeMap::new();

                doc_data.insert(
                    INDEX_ID_KEY.to_string(),
                    Value::from_str(uuid).unwrap_or_else(|_| Value::String(uuid.to_string())),
                );
                if !gather_properties(&config, subject, ic, &mut doc_data).await? {
                    continue 'sub;
                }
                documents.push(doc_data);
            }

            add_or_replace_documents(&config, &ic.name, documents).await?;
        }

        info!(
            "reset done. Please restart the service with the reset flag set to false. NO EVENT will be consumed until then."
        );
        loop {
            // noop
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

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
            retry_fs::remove_dir_all(&task.output_dir).await?;
        }
        retry_fs::create_dir_all(&task.output_dir).await?;
        let manifest =
            tokio::io::BufReader::new(retry_fs::open_file(diff_manifest_file_path).await?).lines();
        if let Err(e) = index(manifest, config).await {
            task.status = Status::Failed(vec![format!("error during indexing!  error: {e}")]);
        } else {
            task.status = Status::Success;
        }

        task.modified_date = Some(Local::now());

        task.result = None; // FIXME, just pure laziness

        return Ok(Some(()));
    }
    Ok(None)
}

async fn index(mut manifest: Lines<BufReader<File>>, config: &Config) -> anyhow::Result<()> {
    // OPTIMIZATION
    // virtuoso_tasks will gather properties from virtuoso in parallel (by default, 255 tasks)
    // we then reduce it to a single meilisearch task.
    // So 255 virtuoso task = 1 meilisearch task (255*255 lines)
    // We can thus do more work in parallel without (hopefully) choking neither meilisearch nor
    // virtuoso
    let mut virtuoso_tasks = JoinSet::new();
    let mut meilisearch_tasks_batch = Vec::with_capacity(config.chunk_size);
    let virtuoso_tasks_consumer =
        async |tasks: &mut JoinSet<anyhow::Result<Vec<MeiliSearchUpdateType>>>,
               meilisearch_tasks: &mut Vec<MeiliSearchUpdateType>| {
            let mut delete_ops = HashMap::new();
            let mut insert_ops = HashMap::new();
            while let Some(task) = tasks.join_next().await {
                let ops = task??;
                for op in ops {
                    match op {
                        MeiliSearchUpdateType::Deletes(deletes) => {
                            delete_ops.extend(deletes.into_iter())
                        }
                        MeiliSearchUpdateType::Inserts(inserts) => {
                            insert_ops.extend(inserts.into_iter())
                        }
                    }
                }
            }
            meilisearch_tasks.extend([
                MeiliSearchUpdateType::Deletes(delete_ops),
                MeiliSearchUpdateType::Inserts(insert_ops),
            ]);

            Ok(()) as anyhow::Result<()>
        };

    while let Ok(Some(line)) = manifest.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        debug!("handling line {line}");

        let payload = DiffResult::deserialize(&line).map_err(|e| anyhow!("{e}"))?;
        let config_clone = config.clone();
        virtuoso_tasks.spawn(async move {
            if let Some(to_remove) = payload.to_remove_path.as_ref() {
                let to_remove = to_remove.clone();
                prepare_update_for_meilisearch_task(
                    &config_clone,
                    to_remove,
                    SparqlUpdateType::Delete,
                )
                .await
            } else if let Some(to_insert) = payload.new_insert_path.as_ref() {
                let to_insert = to_insert.clone();
                prepare_update_for_meilisearch_task(
                    &config_clone,
                    to_insert,
                    SparqlUpdateType::Insert,
                )
                .await
            } else {
                Ok(vec![]) as anyhow::Result<Vec<MeiliSearchUpdateType>>
            }
        });
        tokio::time::sleep(config.sleep_before_next_task).await;
        if virtuoso_tasks.len() >= config.chunk_size {
            virtuoso_tasks_consumer(&mut virtuoso_tasks, &mut meilisearch_tasks_batch).await?;
        }
    }

    virtuoso_tasks_consumer(&mut virtuoso_tasks, &mut meilisearch_tasks_batch).await?;
    let mut meilisearch_tasks = JoinSet::new();
    for batch in chunk_drain(&mut meilisearch_tasks_batch, config.chunk_size) {
        let (deletes, inserts) = batch.into_iter().fold(
            (HashMap::new(), HashMap::new()),
            |(mut deletes, mut inserts), a| {
                match a {
                    MeiliSearchUpdateType::Deletes(d) => deletes.extend(d.into_iter()),
                    MeiliSearchUpdateType::Inserts(i) => inserts.extend(i.into_iter()),
                };
                (deletes, inserts)
            },
        );
        let config_clone = config.clone();

        meilisearch_tasks.spawn(async move {
            for (idx, delete_op) in deletes {
                let task = config_clone
                    .search_client
                    .delete_documents(&idx, &delete_op.into_iter().collect_vec())
                    .await?;
                debug!("{task:?}");
                debug!("waiting for task to complete...");
                config_clone
                    .search_client
                    .wait_for_task(
                        task.task_uid,
                        config_clone.index_interval_wait_for_task,
                        config_clone.index_max_wait_for_task,
                    )
                    .await?;
            }
            for (idx, insert_op) in inserts {
                add_or_replace_documents(&config_clone, &idx, insert_op.into_iter().collect())
                    .await?
            }
            Ok(()) as anyhow::Result<()>
        });
        debug!(
            "sleeping {} millis before next meilisearch task.",
            config.sleep_before_next_meilisearch_task.as_millis()
        );
        tokio::time::sleep(config.sleep_before_next_meilisearch_task).await;
    }
    while let Some(task) = meilisearch_tasks.join_next().await {
        task??;
    }

    Ok(())
}

async fn prepare_update_for_meilisearch_task(
    config: &Config,
    triples_path: PathBuf,
    update_type: SparqlUpdateType,
) -> anyhow::Result<Vec<MeiliSearchUpdateType>> {
    debug!("index {triples_path:?} with operation type {update_type:?}");

    let turtle_str = ungzip(&triples_path).await?;
    let doc = TurtleDoc::try_from((turtle_str.as_str(), None)).map_err(|e| anyhow!("{e}"))?;
    let mut meilisearch_deletes = HashMap::new();
    let mut meilisearch_inserts = HashMap::new();

    for ic in config.index_config.iter() {
        let idx = &ic.name;
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
                    .collect_vec();
                debug!(
                    "deleting the following documents in index {}: {uuids:?}",
                    idx
                );
                let entry = meilisearch_deletes
                    .entry(idx.clone())
                    .or_insert(HashSet::with_capacity(uuids.len()));

                entry.extend(uuids.into_iter());
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
                    let mut doc_data = BTreeMap::new();
                    debug!("processing {subject}");

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
                    if !gather_properties(config, subject, ic, &mut doc_data).await?
                        || doc_data.len() == 1
                    {
                        continue 'sub;
                    }
                    documents.push(doc_data);
                    debug!("sleep before next virtuoso query...");
                    tokio::time::sleep(config.sleep_before_next_virtuoso_query).await;
                }

                let entry = meilisearch_inserts
                    .entry(idx.clone())
                    .or_insert(HashSet::with_capacity(documents.len()));

                entry.extend(documents.into_iter());
            }
            SparqlUpdateType::NoOp => info!("index update: no op"),
        }
    }

    Ok(vec![
        MeiliSearchUpdateType::Deletes(meilisearch_deletes),
        MeiliSearchUpdateType::Inserts(meilisearch_inserts),
    ])
}
async fn add_or_replace_documents(
    config: &Config,
    index: &str,
    mut documents: Vec<BTreeMap<String, Value>>,
) -> anyhow::Result<()> {
    if documents.is_empty() {
        return Ok(());
    }
    let documents = documents
        .drain(..)
        .filter_map(|d| serde_json::to_string(&d).ok())
        .join("\n");
    trace!("documents:{documents}");
    let gzipped_documents = gzip_str(&documents).await?;
    drop(documents);
    let task: TaskInfo = config
        .search_client
        .add_or_replace_documents(
            index,
            INDEX_ID_KEY,
            gzipped_documents,
            Some(ContentType::ApplicationNdJson),
            Some(Encoding::Gzip),
        )
        .await?;
    debug!("{task:?}");
    debug!("waiting for task to complete...");
    config
        .search_client
        .wait_for_task(
            task.task_uid,
            config.index_interval_wait_for_task,
            config.index_max_wait_for_task,
        )
        .await?;
    Ok(())
}

async fn gather_properties(
    config: &Config,
    subject: &str,
    ic: &IndexConfiguration,
    doc_data: &mut BTreeMap<String, Value>,
) -> anyhow::Result<bool> {
    let dummy_pred = "cst:p";
    let construct_properties = ic
        .properties
        .iter()
        .enumerate()
        .map(|(idx, p)| (format!("cst:{idx}"), p))
        .collect::<HashMap<_, _>>();
    let construct_block = format!(
        r#"PREFIX cst: <{CONSTRUCT_PREFIX}>
        {}
        CONSTRUCT {{ {} }}"#,
        PREFIXES
            .iter()
            .map(|(p, uri)| format!("PREFIX {p} <{uri}>"))
            .collect_vec()
            .join("\n"),
        construct_properties
            .iter()
            .enumerate()
            .map(|(idx, (subject_prop, _))| {
                format!("{subject_prop} {dummy_pred} ?{VAR_BINDING}{idx}")
            })
            .collect_vec()
            .join(".")
    );
    let where_block = format!(
        r#"WHERE {{VALUES ?{SUBJECT_BINDING} {{<{subject}>}} {}}}"#,
        ic.properties
            .iter()
            .enumerate()
            .map(|(idx, p)| p.to_query_op(idx))
            .collect_vec()
            .join(" UNION ")
    );
    let construct_query = format!("{construct_block} {where_block}");

    let res = config.sparql_client.query(construct_query).await?;
    let bindings = res.results.bindings;

    // validate
    for (construct_subject, p) in construct_properties.iter() {
        if !bindings.iter().any(|b| &b["s"].value == construct_subject) && !p.optional {
            debug!(
                "{} is not optional in {}. skipping indexing document {subject}",
                p.name, ic.name
            );
            return Ok(false);
        }
    }
    let parse_from_str = DateTime::parse_from_str;
    let parse_from_str_no_tz = NaiveDateTime::parse_from_str;
    // make doc
    for (construct_subject, p) in construct_properties.iter() {
        let mut values = bindings
            .iter()
            .filter(|b| &b["s"].value == construct_subject)
            .map(|b| {
                let o = &b["o"];
                let v = o.value.trim();
                match o.datatype.as_deref() {
                    Some(XSD_DATE) | Some(XSD_DATE_TIME) => DATE_FORMATS
                        .iter()
                        .find_map(|f| match parse_from_str(v, f) {
                            Ok(v) => Some(v),
                            Err(e) => {
                                debug!("could not parse {}. err: {}", v, e);
                                None
                            }
                        })
                        .or_else(|| DateTime::parse_from_rfc3339(v).ok())
                        .or_else(|| {
                            DATE_FORMATS
                                .iter()
                                .find_map(|f| parse_from_str_no_tz(v, f).ok())
                                .and_then(|f| {
                                    f.and_local_timezone(Local::now().timezone())
                                        .map(|f| f.fixed_offset())
                                        .latest()
                                })
                        })
                        .map(|d| d.timestamp())
                        .and_then(|n| Number::from_i128(n as i128))
                        .map(Value::Number)
                        .unwrap_or_else(|| Value::String(v.to_string())),
                    Some(XSD_DECIMAL) | Some(XSD_DOUBLE) => o
                        .value
                        .parse::<f64>()
                        .ok()
                        .and_then(Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or_else(|| Value::String(v.to_string())),
                    Some(XSD_INTEGER) => o
                        .value
                        .parse::<i128>()
                        .ok()
                        .and_then(Number::from_i128)
                        .map(Value::Number)
                        .unwrap_or_else(|| Value::String(v.to_string())),
                    Some(XSD_BOOLEAN) => o
                        .value
                        .parse::<bool>()
                        .map(Value::Bool)
                        .unwrap_or_else(|_| Value::String(v.to_string())),
                    _ => Value::String(v.to_string()),
                }
            })
            .dedup()
            .collect_vec();
        if values.is_empty() {
            continue;
        }
        let value = if values.len() == 1 {
            values.remove(0)
        } else {
            Value::Array(values)
        };
        doc_data.insert(p.name.clone(), value);
    }

    Ok(true)
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
