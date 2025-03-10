/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use chrono::Local;
use std::env::var;
use swarm_common::{
    StreamExt,
    constant::{
        APPLICATION_NAME, ARCHIVE_CONSUMER, JOB_COLLECTION, PUBLIC_TENANT, SUB_TASK_COLLECTION,
        SUB_TASK_EVENT_STREAM, SUB_TASK_STATUS_CHANGE_SUBJECT, TASK_COLLECTION, TASK_EVENT_STREAM,
        TASK_STATUS_CHANGE_EVENT, TASK_STATUS_CHANGE_SUBJECT,
    },
    debug,
    domain::{Job, JsonMapper, Payload, Status, SubTask, Task},
    error, info,
    mongo::{Repository, StoreClient, StoreRepository, doc},
    nats_client::{self, NatsClient},
    setup_tracing,
};

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

    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "archive".into());
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
        .create_durable_consumer(ARCHIVE_CONSUMER, &task_event_stream)
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
                    if matches!(&task.payload, Payload::Archive)
                        && task.status == Status::Scheduled =>
                {
                    let config = config.clone();

                    tokio::spawn(async move {
                        if let Err(e) = message.ack().await {
                            error!("{e}");
                        }
                        task.status = Status::Busy;
                        task.modified_date = Some(Local::now());
                        let _ = config
                            .nc
                            .publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task)
                            .await;

                        match handle_task(&config, &mut task).await {
                            Ok(_) => {
                                let _ = config
                                    .nc
                                    .publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task)
                                    .await;
                            }
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

async fn handle_task(config: &Config, task: &mut Task) -> anyhow::Result<()> {
    let Some(current_job) = config.job_repository.find_by_id(&task.job_id).await? else {
        debug!("current job not found {task:?}");
        return Ok(());
    };
    let old_jobs = config
        .job_repository
        .find_by_query(
            doc! {
                 "_id": { "$ne": &current_job.id },
                 "targetUrl": &current_job.target_url,
                 "definition.id": &current_job.definition.id, // this is to make sure steps are the same
                 "status.type": "success",
            },
            None,
        )
        .await
        .unwrap_or_else(|e| {
            error!("{e}");
            vec![]
        });
    for mut old_job in old_jobs {
        let old_tasks = config
            .task_repository
            .find_by_query(
                doc! {
                  "jobId": &old_job.id
                },
                None,
            )
            .await?;
        for mut ot in old_tasks {
            config
                .sub_task_repository
                .update_many(
                    doc! {
                        "taskId": &ot.id,
                        "status.type": "success",
                    },
                    doc! { "$set": { "status.type": "archived" } },
                )
                .await?;
            ot.status = Status::Archived;
            config.task_repository.upsert(&ot.id, &ot).await?;
        }
        old_job.status = Status::Archived;
        config.job_repository.upsert(&old_job.id, &old_job).await?;
    }

    task.modified_date = Some(Local::now());

    task.status = Status::Success;
    Ok(())
}
