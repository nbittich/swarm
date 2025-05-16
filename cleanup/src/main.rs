/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use chrono::Local;
use std::env::var;
use swarm_common::{
    StreamExt,
    constant::{
        APPLICATION_NAME, CLEANUP_CONSUMER, JOB_COLLECTION, PUBLIC_TENANT, SUB_TASK_COLLECTION,
        SUB_TASK_EVENT_STREAM, SUB_TASK_STATUS_CHANGE_SUBJECT, TASK_COLLECTION, TASK_EVENT_STREAM,
        TASK_STATUS_CHANGE_EVENT, TASK_STATUS_CHANGE_SUBJECT,
    },
    debug,
    domain::{Job, JsonMapper, Payload, Status, SubTask, Task},
    error, info,
    mongo::{Repository, StoreClient, StoreRepository, doc},
    nats_client::{self, NatsClient},
    retry_fs, setup_tracing,
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

    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "cleanup".into());
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
        .create_durable_consumer(CLEANUP_CONSUMER, &task_event_stream)
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
                    if matches!(&task.payload, Payload::Cleanup(_))
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
    let Payload::Cleanup(status) = &task.payload else {
        return Err(anyhow::anyhow!("{task:?} is not a cleanup task!"));
    };
    let jobs_to_clean = config
        .job_repository
        .find_by_query(
            doc! {
                 "_id": { "$ne": &task.job_id },
                 "status.type": status.get_type(),
            },
            None,
        )
        .await
        .unwrap_or_else(|e| {
            error!("{e}");
            vec![]
        });
    for job_to_clean in jobs_to_clean {
        debug!("deleting job {}...", job_to_clean.id);
        let tasks_to_clean = config
            .task_repository
            .find_by_query(
                doc! {
                  "jobId": &job_to_clean.id
                },
                None,
            )
            .await?;
        for task_to_clean in tasks_to_clean {
            debug!("deleting task {}...", task_to_clean.id);
            config
                .sub_task_repository
                .delete_many(Some(doc! {
                    "taskId": &task_to_clean.id
                }))
                .await?;
            config
                .task_repository
                .delete_by_id(&task_to_clean.id)
                .await?;
            debug!("deleting task directory {:?}...", task_to_clean.output_dir);
            if let Err(e) = retry_fs::remove_dir_all(&task_to_clean.output_dir).await {
                error!("could not delete task directory: {e}");
            }
            debug!("directory task {:?} deleted.", task_to_clean.output_dir);
        }
        debug!("deleting job directory {:?}...", job_to_clean.root_dir);
        if let Err(e) = retry_fs::remove_dir_all(&job_to_clean.root_dir).await {
            error!("could not delete job directory: {e}");
        }
        debug!("job directory {:?} deleted.", job_to_clean.root_dir);
        config.job_repository.delete_by_id(&job_to_clean.id).await?;
    }

    task.modified_date = Some(Local::now());

    task.status = Status::Success;
    Ok(())
}
