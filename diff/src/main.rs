/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use anyhow::anyhow;
use chrono::Local;
use std::{env::var, path::Path, time::Duration};
use swarm_common::{
    IdGenerator, StreamExt,
    constant::{
        APPLICATION_NAME, DIFF_CONSUMER, JOB_COLLECTION, MANIFEST_FILE_NAME, PUBLIC_TENANT,
        SUB_TASK_COLLECTION, SUB_TASK_EVENT_STREAM, SUB_TASK_STATUS_CHANGE_EVENT,
        SUB_TASK_STATUS_CHANGE_SUBJECT, TASK_COLLECTION, TASK_EVENT_STREAM,
        TASK_STATUS_CHANGE_EVENT, TASK_STATUS_CHANGE_SUBJECT,
    },
    debug,
    domain::{
        DiffResult, Job, JsonMapper, NTripleResult, Payload, Status, SubTask, SubTaskResult, Task,
        TaskResult,
    },
    error, info,
    mongo::{Repository, StoreClient, StoreRepository, doc},
    nats_client::{self, NatsClient},
    setup_tracing,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    task::JoinSet,
};
use tortank::turtle::turtle_doc::TurtleDoc;

#[derive(Clone)]
struct Config {
    task_repository: StoreRepository<Task>,
    sub_task_repository: StoreRepository<SubTask>,
    job_repository: StoreRepository<Job>,
    nc: NatsClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "diff".into());
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
        .create_durable_consumer(DIFF_CONSUMER, &task_event_stream)
        .await?;

    let mongo_client = StoreClient::new(app_name.to_string()).await?;
    let job_repository: StoreRepository<Job> =
        StoreRepository::get_repository(&mongo_client, JOB_COLLECTION, PUBLIC_TENANT);

    let sub_task_repository: StoreRepository<SubTask> =
        StoreRepository::get_repository(&mongo_client, SUB_TASK_COLLECTION, PUBLIC_TENANT);
    let task_repository: StoreRepository<Task> =
        StoreRepository::get_repository(&mongo_client, TASK_COLLECTION, PUBLIC_TENANT);
    let config = Config {
        job_repository,
        sub_task_repository,
        task_repository,
        nc,
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
                            payload: Some(TaskResult::ComplementWithUuid { .. }),
                            ..
                        }
                    ) && task.status == Status::Scheduled =>
                {
                    let config = config.clone();

                    tokio::spawn(async move {
                        if let Err(e) = message.ack().await {
                            error!("{e}");
                        }
                        task.has_sub_task = true;
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

pub async fn append_entry_manifest_file(
    dir_path: &Path,
    page_res: &DiffResult,
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

async fn handle_task(config: &Config, task: &mut Task) -> anyhow::Result<Option<()>> {
    if let Payload::FromPreviousStep {
        payload:
            Some(TaskResult::ComplementWithUuid {
                manifest_file_path,
                success_count: previous_success_count,
                failure_count: previous_failure_count,
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

        let Some(current_job) = config.job_repository.find_by_id(&task.job_id).await? else {
            return Err(anyhow!("current job not found {task:?}"));
        };
        let Some(old_job) = config
            .job_repository
            .find_one(Some(doc! {
                 "_id": { "$ne": &current_job.id },
                 "status.type": "success",
                 "targetUrl": &current_job.target_url,
                 "definition.id": &current_job.definition.id // this is to make sure steps are the same

            }))
            .await?
        else {
            debug!("could not find old job {current_job:?}");
            // no old job. just copy the manifest from the previous step
            while let Ok(Some(line)) = manifest.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                let diff_result = convert_ntriple_to_diff(&line, false).await?;

                append_entry_manifest_file(&task.output_dir, &diff_result).await?;
                let sub_task = SubTask {
                    id: IdGenerator.get(),
                    task_id: task.id.clone(),
                    creation_date: Local::now(),
                    modified_date: None,
                    status: Status::Success,
                    result: Some(SubTaskResult::Diff(diff_result)),
                };
                let _ = config
                    .nc
                    .publish(SUB_TASK_STATUS_CHANGE_EVENT(&sub_task.id), &sub_task)
                    .await;
            }
            task.modified_date = Some(Local::now());
            task.result = Some(TaskResult::Diff {
                success_count: *previous_success_count,
                failure_count: *previous_failure_count,
                manifest_file_path: task.output_dir.join(MANIFEST_FILE_NAME),
            });
            task.status = Status::Success;
            return Ok(Some(()));
        };
        let Some(old_diff_task) = config
            .task_repository
            .find_one(Some(doc! {
                        "result.type": "diff",
                        "status.type": "success",
                        "jobId": &old_job.id
            }))
            .await?
        else {
            return Err(anyhow!("diff step not found for old job {old_job:?}"));
        };

        let mut tasks = JoinSet::new();
        while let Ok(Some(line)) = manifest.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            let sub_task = SubTask {
                id: IdGenerator.get(),
                task_id: task.id.clone(),
                creation_date: Local::now(),
                modified_date: None,
                status: Status::Busy,
                result: None,
            };
            let out_dir = task.output_dir.clone();
            let old_diff_task_id = old_diff_task.id.clone();
            let config = config.clone();
            let _ = config
                .nc
                .publish(SUB_TASK_STATUS_CHANGE_EVENT(&sub_task.id), &sub_task)
                .await;
            tasks.spawn(async move {
                match diff(&line, &old_diff_task_id, &config, &out_dir).await {
                    Ok(diff) => Ok((sub_task, diff)),
                    Err(e) => Err((sub_task, e)),
                }
            });
            // sleep just a little to avoid using all the cpu
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        while let Some(handle) = tasks.join_next().await {
            let mut sub_task = match handle? {
                Ok((mut sub_task, diff)) => {
                    append_entry_manifest_file(&task.output_dir, &diff).await?;
                    success_count += 1;
                    sub_task.status = Status::Success;
                    sub_task.result = Some(SubTaskResult::Diff(diff));
                    sub_task
                }

                Err((mut sub_task, e)) => {
                    failure_count += 1;
                    sub_task.status = Status::Failed(vec![format!("error during diffing! {e:?}")]);
                    sub_task
                }
            };
            sub_task.modified_date = Some(Local::now());
            let _ = config
                .nc
                .publish(SUB_TASK_STATUS_CHANGE_EVENT(&sub_task.id), &sub_task)
                .await;
        }

        task.modified_date = Some(Local::now());
        if success_count == 0 && failure_count > 0 {
            task.status = Status::Failed(vec![format!(
                "task did not succeed: success: {success_count}, failure: {failure_count}"
            )]);
        } else {
            task.result = Some(TaskResult::Diff {
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

async fn convert_ntriple_to_diff(line: &str, to_remove: bool) -> anyhow::Result<DiffResult> {
    let NTripleResult {
        base_url,
        path,
        creation_date,
        ..
    } = NTripleResult::deserialize(line)?;
    let (to_remove_path, new_insert_path) = if to_remove {
        (Some(path), None)
    } else {
        (None, Some(path))
    };
    Ok(DiffResult {
        base_url,
        new_insert_path,
        intersect_path: None,
        to_remove_path,
        creation_date,
    })
}

async fn diff(
    line: &str,
    old_diff_task_id: &str,
    config: &Config,
    output_dir: &Path,
) -> anyhow::Result<DiffResult> {
    let payload = NTripleResult::deserialize(line)?;
    let ttl_file = tokio::fs::read_to_string(payload.path).await?;

    debug!("old diff task id: {old_diff_task_id:?}");
    let doc = TurtleDoc::try_from((ttl_file.as_str(), None)).map_err(|e| anyhow::anyhow!("{e}"))?;
    let new_ttl_buff;
    let intersect_ttl_buff;

    let old_doc = {
        let Some(SubTaskResult::Diff(DiffResult {
            new_insert_path,
            intersect_path,
            ..
        })) = config
            .sub_task_repository
            .find_one(Some(doc! {
                "taskId": old_diff_task_id,
                "status.type":"success",
                "result.value.baseUrl": &payload.base_url
            }))
            .await?
            .and_then(|st| st.result)
        else {
            debug!("previous task not found {old_diff_task_id}");
            return convert_ntriple_to_diff(line, false).await;
        };

        let new_ttl_doc = if let Some(p) = new_insert_path {
            new_ttl_buff = tokio::fs::read_to_string(p).await?;
            TurtleDoc::try_from((new_ttl_buff.as_str(), None))
                .map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            TurtleDoc::default()
        };

        let intersect_ttl_doc = if let Some(p) = intersect_path {
            intersect_ttl_buff = tokio::fs::read_to_string(p).await?;

            TurtleDoc::try_from((intersect_ttl_buff.as_str(), None))
                .map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            TurtleDoc::default()
        };
        new_ttl_doc + intersect_ttl_doc
    };

    let intersection_doc = doc.intersection(&old_doc).map_err(|e| anyhow!("{e}"))?;
    let new_doc = doc.difference(&old_doc).map_err(|e| anyhow!("{e}"))?;
    let to_remove_doc = old_doc.difference(&doc).map_err(|e| anyhow!("{e}"))?;
    let intersect_path = {
        if intersection_doc.is_empty() {
            None
        } else {
            let id = IdGenerator.get();
            let path = output_dir.join(format!("intersect-triples-{id}.ttl"));
            tokio::fs::write(&path, intersection_doc.to_string())
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Some(path)
        }
    };
    let new_insert_path = {
        if new_doc.is_empty() {
            None
        } else {
            let id = IdGenerator.get();
            let path = output_dir.join(format!("new-triples-{id}.ttl"));
            tokio::fs::write(&path, new_doc.to_string())
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Some(path)
        }
    };
    let to_remove_path = {
        if to_remove_doc.is_empty() {
            None
        } else {
            let id = IdGenerator.get();
            let path = output_dir.join(format!("to-remove-triples-{id}.ttl"));
            tokio::fs::write(&path, to_remove_doc.to_string())
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Some(path)
        }
    };
    Ok(DiffResult {
        base_url: payload.base_url,
        new_insert_path,
        to_remove_path,
        intersect_path,
        creation_date: Local::now(),
    })
}
