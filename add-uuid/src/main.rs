/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use anyhow::anyhow;
use chrono::Local;
use moka::future::Cache;
use std::{borrow::Cow, env::var, path::Path};
use swarm_common::{
    constant::{
        ADD_UUID_CONSUMER, APPLICATION_NAME, MANIFEST_FILE_NAME, PUBLIC_TENANT,
        SUB_TASK_EVENT_STREAM, SUB_TASK_STATUS_CHANGE_EVENT, SUB_TASK_STATUS_CHANGE_SUBJECT,
        TASK_EVENT_STREAM, TASK_STATUS_CHANGE_EVENT, TASK_STATUS_CHANGE_SUBJECT, UUID_COLLECTION,
        UUID_COMPLEMENT_PREDICATE,
    },
    debug,
    domain::{
        JsonMapper, NTripleResult, Payload, ScrapeResult, Status, SubTask, SubTaskResult, Task,
        TaskResult, UuidSubject,
    },
    error, info,
    mongo::{doc, Repository, StoreClient, StoreRepository},
    nats_client::{self, NatsClient},
    setup_tracing, IdGenerator, StreamExt,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tortank::turtle::turtle_doc::TurtleDoc;

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
    let cache = moka::future::Cache::new(10_000);

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
                        task.modified_date = Some(Local::now().to_utc());
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
                                task.modified_date = Some(Local::now().to_utc());
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
    let mut manifest_file = tokio::fs::File::options()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    manifest_file.write_all(line.as_bytes()).await?;

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
            tokio::fs::remove_dir_all(&task.output_dir).await?;
        }
        tokio::fs::create_dir_all(&task.output_dir).await?;
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut manifest =
            tokio::io::BufReader::new(tokio::fs::File::open(manifest_file_path).await?).lines();

        while let Ok(Some(line)) = manifest.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            let mut sub_task = SubTask {
                id: IdGenerator.get(),
                task_id: task.id.clone(),
                creation_date: Local::now().to_utc(),
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
            sub_task.modified_date = Some(Local::now().to_utc());
            let _ = nc
                .publish(SUB_TASK_STATUS_CHANGE_EVENT(&sub_task.id), &sub_task)
                .await;
        }

        task.modified_date = Some(Local::now().to_utc());
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

async fn get_id_from_cache_or_insert(
    subject_str: String,
    cache: &Cache<String, String>,
    repository: &StoreRepository<UuidSubject>,
) -> anyhow::Result<String> {
    match cache.get(&subject_str).await {
        Some(id) => Ok(id),
        None => {
            let id = match repository
                .find_one(Some(doc! {
                    "subject": &subject_str
                }))
                .await?
            {
                Some(UuidSubject { id, .. }) => id,
                None => {
                    let id = IdGenerator.get();
                    repository
                        .insert_one(&UuidSubject {
                            id: id.to_string(),
                            subject: subject_str.clone(),
                        })
                        .await?;
                    id
                }
            };
            cache.insert(subject_str, id.to_string()).await;
            Ok(id)
        }
    }
}
async fn complement(
    line: &str,
    cache: &Cache<String, String>,
    repository: &StoreRepository<UuidSubject>,
    predicate: &str,
    output_dir: &Path,
) -> anyhow::Result<NTripleResult> {
    let payload = ScrapeResult::deserialize(line)?;
    let ttl_file = tokio::fs::read_to_string(payload.path).await?;

    let doc = TurtleDoc::try_from((ttl_file.as_str(), None)).map_err(|e| anyhow::anyhow!("{e}"))?;
    let subjects = doc.all_subjects();
    let mut triples = doc
        .difference(&TurtleDoc::default())
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    for subject in subjects {
        let subject_str = subject.to_string();
        let id = get_id_from_cache_or_insert(subject_str, cache, repository).await?;
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

    let path = output_dir.join(format!("complemented-{id}.ttl"));
    tokio::fs::write(&path, triples.to_string())
        .await
        .map_err(|e| anyhow!("{e}"))?;
    Ok(NTripleResult {
        base_url: payload.base_url,
        len: triples.len(),
        path,
        creation_date: Local::now().to_utc(),
    })
}