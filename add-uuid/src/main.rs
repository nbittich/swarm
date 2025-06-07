/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use anyhow::anyhow;
use chrono::Local;
use moka::{future::Cache, policy::EvictionPolicy};
use std::{borrow::Cow, collections::HashMap, env::var, path::Path, sync::Arc};
use swarm_common::{
    IdGenerator, StreamExt,
    compress::{gzip, ungzip},
    constant::{
        ADD_UUID_CONSUMER, APPLICATION_NAME, CACHE_SIZE, MANIFEST_FILE_NAME, PUBLIC_TENANT,
        SUB_TASK_EVENT_STREAM, SUB_TASK_STATUS_CHANGE_EVENT, SUB_TASK_STATUS_CHANGE_SUBJECT,
        TASK_EVENT_STREAM, TASK_STATUS_CHANGE_EVENT, TASK_STATUS_CHANGE_SUBJECT, UUID_COLLECTION,
        UUID_COMPLEMENT_PREDICATE,
    },
    debug,
    domain::{
        JsonMapper, NTripleResult, Payload, Status, SubTask, SubTaskResult, Task, TaskResult,
        UuidSubject,
    },
    error, info,
    mongo::{Repository, StoreClient, StoreRepository, doc},
    nats_client::{self, NatsClient},
    retry_fs, setup_tracing,
};
use tokio::io::AsyncBufReadExt;
use tortank::turtle::turtle_doc::{Node, TurtleDoc};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "add-uuid".into());
    let uuid_predicate = var(UUID_COMPLEMENT_PREDICATE)?;
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
        .create_durable_consumer(ADD_UUID_CONSUMER, &task_event_stream)
        .await?;

    let mongo_client = StoreClient::new(app_name.to_string()).await?;
    let uuid_repository: StoreRepository<UuidSubject> =
        StoreRepository::get_repository(&mongo_client, UUID_COLLECTION, PUBLIC_TENANT);
    let cache_size = var(CACHE_SIZE)
        .ok()
        .and_then(|cs| cs.parse::<u64>().ok())
        .unwrap_or(100_000);
    let cache = moka::future::CacheBuilder::new(cache_size)
        .eviction_policy(EvictionPolicy::tiny_lfu())
        .build();
    info!(
        "prepopulate at most 80% of the cache capacity ({}) with data from mongo...",
        cache_size * 80 / 100
    );

    let data_in_db = uuid_repository
        .find_page_large_collection_batched(
            None,
            None,
            (cache_size * 70 / 100) as i64,
            Some(100_000),
        )
        .await?;
    for (subject_hash, id) in data_in_db
        .content
        .into_iter()
        .map(|u| (u.subject_hash, u.id))
    {
        cache.insert(subject_hash, id).await;
    }
    info!("cache populated");

    let mut messages = task_event_consumer.messages().await?;

    info!("app {app_name} started and ready to consume messages.");
    while let Some(message) = messages.next().await {
        match message {
            Ok(message) => match Task::deserialize_bytes(&message.payload) {
                Ok(mut task)
                    if matches!(
                        &task.payload,
                        Payload::FromPreviousStep {
                            payload: Some(TaskResult::FilterSHACL { .. }),
                            ..
                        }
                    ) && task.status == Status::Scheduled =>
                {
                    let nc = nc.clone();
                    let uuid_repository = uuid_repository.clone();
                    let predicate = uuid_predicate.clone();
                    let cache = cache.clone();

                    tokio::spawn(async move {
                        if let Err(e) = message.ack().await {
                            error!("{e}");
                            return;
                        }
                        task.has_sub_task = true;
                        task.status = Status::Busy;
                        task.modified_date = Some(Local::now());
                        let _ = nc.publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task).await;
                        match handle_task(&nc, &cache, &uuid_repository, &predicate, &mut task)
                            .await
                        {
                            Ok(Some(_)) => {
                                let _ = nc.publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task).await;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                task.status =
                                    Status::Failed(vec![format!("unexpected error: {e}")]);
                                task.modified_date = Some(Local::now());
                                let _ = nc.publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task).await;
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
            Err(e) => {
                error!("could not get message {e}");
            }
        }
    }
    info!("closing service...BYE");
    Ok(())
}

pub async fn append_entry_manifest_file(
    dir_path: &Path,
    page_res: &NTripleResult,
) -> anyhow::Result<()> {
    let mut line = page_res.serialize()?;
    line += "\n";
    let path = dir_path.join(MANIFEST_FILE_NAME);
    retry_fs::append_to_file(path, line).await?;

    Ok(())
}

async fn handle_task(
    nc: &NatsClient,
    cache: &Cache<String, String>,
    repository: &StoreRepository<UuidSubject>,
    predicate: &str,
    task: &mut Task,
) -> anyhow::Result<Option<()>> {
    if let Payload::FromPreviousStep {
        payload: Some(TaskResult::FilterSHACL {
            manifest_file_path, ..
        }),
        ..
    } = &task.payload
    {
        if task.output_dir.exists() {
            retry_fs::remove_dir_all(&task.output_dir).await?;
        }
        retry_fs::create_dir_all(&task.output_dir).await?;
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut manifest =
            tokio::io::BufReader::new(retry_fs::open_file(manifest_file_path).await?).lines();

        while let Ok(Some(line)) = manifest.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            let mut sub_task = SubTask {
                id: IdGenerator.get(),
                task_id: task.id.clone(),
                creation_date: Local::now(),
                modified_date: None,
                status: Status::Busy,
                result: None,
            };
            let out_dir = task.output_dir.clone();
            let _ = nc
                .publish(SUB_TASK_STATUS_CHANGE_EVENT(&sub_task.id), &sub_task)
                .await;

            // we do it one by one to avoid inconsistency
            match complement(&line, cache, repository, predicate, &out_dir).await {
                Ok(NTripleResult { len: 0, .. }) => {
                    sub_task.status = Status::Failed(vec!["did not complement any data".into()]);
                    failure_count += 1;
                }
                Ok(triples) => {
                    append_entry_manifest_file(&task.output_dir, &triples).await?;
                    success_count += 1;
                    sub_task.status = Status::Success;
                    sub_task.result = Some(SubTaskResult::NTriple(triples));
                }

                Err(e) => {
                    failure_count += 1;
                    sub_task.status = Status::Failed(vec![format!("error during add uuid! {e:?}")])
                }
            }
            sub_task.modified_date = Some(Local::now());
            let _ = nc
                .publish(SUB_TASK_STATUS_CHANGE_EVENT(&sub_task.id), &sub_task)
                .await;
        }

        task.modified_date = Some(Local::now());
        if success_count == 0 && failure_count > 0 {
            task.status = Status::Failed(vec![format!(
                "task did not succeed: success: {success_count}, failure: {failure_count}"
            )]);
        } else {
            task.result = Some(TaskResult::ComplementWithUuid {
                success_count,
                failure_count,
                manifest_file_path: task.output_dir.join(MANIFEST_FILE_NAME),
            });
            task.status = Status::Success;
        }
        return Ok(Some(()));
    }
    Ok(None)
}
async fn get_ids_from_cache_or_insert<'a>(
    mut subject_nodes: Vec<Node<'a>>,
    cache: &Cache<String, String>,
    repository: &StoreRepository<UuidSubject>,
) -> anyhow::Result<HashMap<String, Node<'a>>> {
    // FIXME optimization:
    // assuming the uuid is in mongo, we can infer that it's also already in the db
    // so we don't need to insert extra triples
    // but to be sure just leave it like that for now
    if subject_nodes.is_empty() {
        return Ok(HashMap::with_capacity(0));
    }
    let mut result = HashMap::with_capacity(subject_nodes.len());
    let mut to_check_in_db = HashMap::with_capacity(subject_nodes.len());
    while let Some(subject) = subject_nodes.pop() {
        let subject_iri = subject.get_iri().map_err(|e| anyhow!("{e}"))?;
        let subject_hash = xxhash_rust::xxh3::xxh3_64(subject_iri.as_bytes()).to_string();
        if let Some(id) = cache.get(&subject_hash).await {
            result.insert(id, subject);
        } else {
            to_check_in_db.insert(subject_hash, subject);
        }
    }
    if !to_check_in_db.is_empty() {
        let in_db = repository
            .find_by_query(
                doc! {
                    "_id": {
                      "$in": &to_check_in_db.keys()
                        .collect::<Vec<_>>()
                    }
                },
                None,
            )
            .await?;
        let (found_in_db, to_inserts): (Vec<_>, Vec<_>) = to_check_in_db
            .into_iter()
            .map(|(subject_hash, node)| {
                if let Some(u) = in_db.iter().find(|u| u.subject_hash == subject_hash) {
                    (u.id.to_string(), node)
                } else {
                    (IdGenerator.get(), node)
                }
            })
            .partition(|(id, _)| in_db.iter().any(|u| &u.id == id));

        let to_add_in_cache = to_inserts
            .iter()
            .map(
                |(id, node)| match node.get_iri().map_err(|e| anyhow!("{e}")) {
                    Ok(subject_iri) => Ok(UuidSubject {
                        id: id.clone(),
                        subject_hash: xxhash_rust::xxh3::xxh3_64(subject_iri.as_bytes())
                            .to_string(),
                    }),
                    Err(e) => Err(e),
                },
            )
            .collect::<Result<Vec<_>, _>>()?;
        if !to_add_in_cache.is_empty() {
            repository.insert_many(&to_add_in_cache).await?;
            // now we add to cache
            for us in to_add_in_cache {
                cache.insert(us.subject_hash, us.id).await;
            }
        }
        result = result
            .into_iter()
            .chain(found_in_db.into_iter())
            .chain(to_inserts.into_iter())
            .collect();
    }
    Ok(result)
}
async fn complement(
    line: &str,
    cache: &Cache<String, String>,
    repository: &StoreRepository<UuidSubject>,
    predicate: &str,
    output_dir: &Path,
) -> anyhow::Result<NTripleResult> {
    let payload = NTripleResult::deserialize(line)?;
    let ttl_file = ungzip(&payload.path).await?;

    let doc = TurtleDoc::try_from((ttl_file.as_str(), None)).map_err(|e| anyhow::anyhow!("{e}"))?;
    let subjects = doc.all_subjects();
    let mut triples = doc
        .difference(&TurtleDoc::default())
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let uuid_complement = get_ids_from_cache_or_insert(subjects, cache, repository).await?;
    for (id, subject) in uuid_complement {
        triples.add_statement(
            subject,
            tortank::turtle::turtle_doc::Node::Iri(Cow::Borrowed(predicate)),
            tortank::turtle::turtle_doc::Node::Literal(
                tortank::turtle::turtle_doc::Literal::Quoted {
                    value: Cow::Owned(id),
                    lang: None,
                    datatype: None,
                },
            ),
        );
    }
    let id = IdGenerator.get();

    let path = {
        let path = output_dir.join(format!("complemented-{id}.ttl"));
        retry_fs::write(&path, Arc::new(triples.to_string())).await?;
        gzip(&path, true).await?
    };
    Ok(NTripleResult {
        base_url: payload.base_url,
        len: triples.len(),
        path,
        creation_date: Local::now(),
    })
}
