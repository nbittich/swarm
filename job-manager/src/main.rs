/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use manager::JobManagerState;

use std::{env::var, time::Duration};
use swarm_common::{
    constant::{
        APPLICATION_NAME, BODY_SIZE_LIMIT, SCHEDULE_START_DELAY, SERVICE_HOST, SERVICE_PORT,
    },
    error, info, setup_tracing,
};

mod api;
mod domain;
mod manager;
const JOB_DEFINITIONS_PATH: &str = "JOB_DEFINITIONS_PATH";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();
    let body_size_limit = (std::env::var(BODY_SIZE_LIMIT).unwrap_or_else(|_| "50".to_string())) // 50mb
        .parse::<usize>()?
        * 1024
        * 1024;
    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "job-manager".into());
    let host = var(SERVICE_HOST).unwrap_or_else(|_| String::from("127.0.0.1"));
    let port = var(SERVICE_PORT).unwrap_or_else(|_| String::from("80"));
    let schedule_start_delay = var(SCHEDULE_START_DELAY)
        .iter()
        .flat_map(|d| d.parse::<u64>())
        .map(Duration::from_secs)
        .last()
        .unwrap_or_else(|| Duration::from_secs(300));
    let job_definitions_path = var(JOB_DEFINITIONS_PATH).unwrap_or_else(|_| "/job_def.json".into());
    let manager_state = JobManagerState::new(&app_name, &job_definitions_path).await?;
    let cloned_state = manager_state.clone();
    let serve_handle =
        tokio::spawn(async move { api::serve(&host, &port, body_size_limit, cloned_state).await });
    let task_consumer_handle = {
        let manager_state = manager_state.clone();
        tokio::spawn(async move { manager_state.start_consuming_task().await })
    };

    let schedule_executor_handle = {
        let manager_state = manager_state.clone();
        tokio::spawn(async move {
            info!(
                "job scheduler will start in {} seconds...",
                schedule_start_delay.as_secs()
            );
            tokio::time::sleep(schedule_start_delay).await;
            manager_state.start_scheduled_job_executor().await
        })
    };
    let sub_task_consumer_handle =
        tokio::spawn(async move { manager_state.start_consuming_sub_task().await });
    tokio::select! {
        result = serve_handle => {
            if let Err(err) = result {
                error!("Serve task failed: {err:?}");
            }
            error!("Shutting down because the serve task ended.");
        },
        result = task_consumer_handle => {
            if let Err(err) = result {
                error!("Consumer task failed: {err:?}");
            }
            error!("Shutting down because the consumer task ended.");
        },
        result = sub_task_consumer_handle => {
            if let Err(err) = result {
                error!("Consumer sub task failed: {err:?}");
            }
            error!("Shutting down because the consumer subtask ended.");
        },
        result = schedule_executor_handle => {
            if let Err(err) = result {
                error!("Schedule executor failed: {err:?}");
            }
            error!("Shutting down because the schedule executor ended.");
        },
    };
    Ok(())
}
