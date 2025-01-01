use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::anyhow;
use chrono::{Local, Utc};
use cron::Schedule;
use sparql_client::SparqlClient;
use swarm_common::{
    constant::{
        JOB_COLLECTION, JOB_MANAGER_CONSUMER, PUBLIC_TENANT, SCHEDULED_JOB_COLLECTION,
        SUB_TASK_COLLECTION, SUB_TASK_EVENT_STREAM, SUB_TASK_STATUS_CHANGE_SUBJECT,
        TASK_COLLECTION, TASK_EVENT_STREAM, TASK_STATUS_CHANGE_EVENT, TASK_STATUS_CHANGE_SUBJECT,
        USER_COLLECTION,
    },
    debug,
    domain::{
        Job, JobDefinition, JsonMapper, Payload, ScheduledJob, Status, SubTask, Task,
        TaskDefinition, User,
    },
    error,
    mongo::{doc, Repository, StoreClient, StoreRepository},
    nats_client::{self, NatsClient, PullConsumer, Stream},
    warn, IdGenerator, StreamExt, REGEX_CLEAN_URL,
};

use crate::domain::{ApiError, ROOT_OUTPUT_DIR_PB};

#[derive(Debug, Clone)]
pub struct JobManagerState {
    pub nc: NatsClient,
    pub sparql_client: SparqlClient,
    pub task_event_consumer: PullConsumer,
    pub _task_event_stream: Stream,
    pub _sub_task_event_stream: Stream,
    pub sub_task_event_consumer: PullConsumer,
    pub job_definitions: Arc<Vec<JobDefinition>>,
    pub job_repository: StoreRepository<Job>,
    pub scheduled_job_repository: StoreRepository<ScheduledJob>,
    pub task_repository: StoreRepository<Task>,
    pub sub_task_repository: StoreRepository<SubTask>,
    pub user_repository: StoreRepository<User>,
    pub _mongo_client: StoreClient,
}

impl JobManagerState {
    pub async fn new(app_name: &str, job_definitions_path: &str) -> anyhow::Result<Self> {
        type JobDefTypes = Vec<JobDefinition>;
        let def_json = tokio::fs::read_to_string(job_definitions_path).await?;
        let mut job_definitions = JobDefTypes::deserialize(&def_json)?;

        for jd in job_definitions.iter_mut() {
            if jd.tasks.is_empty() {
                return Err(anyhow!("{jd:?} has no tasks"));
            }
            jd.tasks.sort_by(|t1, t2| t1.order.cmp(&t2.order));
        }
        let nc = nats_client::connect().await?;

        let task_event_stream = nc
            .add_stream(
                TASK_EVENT_STREAM,
                vec![TASK_STATUS_CHANGE_SUBJECT.to_string()],
            )
            .await?;

        let task_event_consumer = nc
            .create_durable_consumer(JOB_MANAGER_CONSUMER, &task_event_stream)
            .await?;

        let sub_task_event_stream = nc
            .add_stream(
                SUB_TASK_EVENT_STREAM,
                vec![SUB_TASK_STATUS_CHANGE_SUBJECT.to_string()],
            )
            .await?;

        let sub_task_event_consumer = nc
            .create_durable_consumer(JOB_MANAGER_CONSUMER, &sub_task_event_stream)
            .await?;
        let mongo_client = StoreClient::new(app_name.to_string()).await?;
        let job_repository =
            StoreRepository::get_repository(&mongo_client, JOB_COLLECTION, PUBLIC_TENANT);

        let task_repository =
            StoreRepository::get_repository(&mongo_client, TASK_COLLECTION, PUBLIC_TENANT);

        let sub_task_repository =
            StoreRepository::get_repository(&mongo_client, SUB_TASK_COLLECTION, PUBLIC_TENANT);

        let scheduled_job_repository =
            StoreRepository::get_repository(&mongo_client, SCHEDULED_JOB_COLLECTION, PUBLIC_TENANT);

        let user_repository =
            StoreRepository::get_repository(&mongo_client, USER_COLLECTION, PUBLIC_TENANT);
        Ok(JobManagerState {
            nc,
            task_event_consumer,
            job_definitions: Arc::new(job_definitions),
            job_repository,
            task_repository,
            scheduled_job_repository,
            user_repository,
            sub_task_repository,
            _mongo_client: mongo_client,
            _task_event_stream: task_event_stream,
            _sub_task_event_stream: sub_task_event_stream,
            sub_task_event_consumer,
            sparql_client: SparqlClient::new()?,
        })
    }

    pub async fn delete_job(&self, job_id: &str) -> anyhow::Result<()> {
        let Some(job) = self.job_repository.find_by_id(job_id).await? else {
            debug!("");
            return Err(anyhow!("job not found {job_id:?}"));
        };
        let tasks = self
            .task_repository
            .find_by_query(
                doc! {
                  "jobId": &job.id
                },
                None,
            )
            .await?;
        for ot in tasks {
            self.sub_task_repository
                .delete_many(Some(doc! {
                    "taskId": &ot.id
                }))
                .await?;
            self.task_repository.delete_by_id(&ot.id).await?;
        }
        if let Err(e) = tokio::fs::remove_dir_all(job.root_dir).await {
            return Err(anyhow!("could not delete directory {e}"));
        }
        self.job_repository.delete_by_id(&job.id).await?;
        Ok(())
    }

    pub async fn start_scheduled_job_executor(&self) -> anyhow::Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let scheduled_jobs = self.scheduled_job_repository.find_all().await?;
            let now = Local::now().to_utc();
            for mut sj in scheduled_jobs {
                let schedule = Schedule::from_str(&sj.cron_expr)?;
                let mut upcomings = schedule.upcoming(Utc);
                if sj.next_execution.is_none() {
                    sj = ScheduledJob {
                        next_execution: upcomings.next(),
                        ..sj
                    };
                    self.scheduled_job_repository.upsert(&sj.id, &sj).await?;
                }
                let Some(upcoming) = sj.next_execution else {
                    warn!("no upcoming schedule for expression {schedule}");
                    continue;
                };
                if upcoming <= now {
                    sj = ScheduledJob {
                        next_execution: upcomings.next(),
                        ..sj
                    };
                    self.scheduled_job_repository.upsert(&sj.id, &sj).await?;
                    self.new_job(sj.definition_id, sj.name, sj.target_url)
                        .await
                        .map_err(|e| anyhow!("{e:?}"))?;
                }
            }
        }
    }

    pub async fn start_consuming_task(&self) -> anyhow::Result<()> {
        let mut messages = self.task_event_consumer.messages().await?;
        while let Some(message) = messages.next().await {
            match message {
                Ok(message) => {
                    match Task::deserialize_bytes(&message.payload) {
                        Ok(mut task) => {
                            // at this point we are not sure the database or jetstream is available, so we do
                            // our best to update the task
                            self.task_repository.upsert(&task.id, &task).await?;
                            if matches!(
                                task.status,
                                Status::Success | Status::Failed(_) | Status::Pending
                            ) {
                                if let Ok(Some(mut job)) =
                                    self.job_repository.find_by_id(&task.job_id).await
                                {
                                    job.modified_date = Some(Local::now().to_utc());
                                    let mut allow_running_job = true;
                                    if !job.definition.allow_concurrent_run {
                                        if let Ok(result) = self
                                        .job_repository
                                        .find_by_query(
                                            doc! {
                                                 "_id": { "$ne": &job.id },
                                                 "status.type": { "$in": [ "busy", "scheduled","pending"] },
                                                 "targetUrl": &job.target_url,

                                            },
                                            None,
                                        )
                                        .await
                                    {
                                        if !result.is_empty() {
                                            allow_running_job = false;
                                            job.status = Status::Failed(vec![
                                                "only one concurrent job".into(),
                                            ]);
                                            self.task_repository.delete_by_id(&task.id).await?;
                                        }
                                    }
                                    }

                                    if allow_running_job {
                                        if matches!(task.status, Status::Failed(_)) {
                                            job.status = task.status;
                                        } else {
                                            job.status = Status::Busy;
                                            let mut task_id = IdGenerator.get();
                                            match job.definition.tasks.iter().find(|t| {
                                                if matches!(task.status, Status::Pending) {
                                                    task_id = task.id.to_string();
                                                    t.order == task.order
                                                } else {
                                                    t.order > task.order
                                                }
                                            }) {
                                                Some(td) => {
                                                    let output_dir = job.root_dir.join(&td.name);

                                                    let next_task = Task {
                                                        id: task_id,
                                                        order: td.order,
                                                        job_id: job.id.clone(),
                                                        name: td.name.clone(),
                                                        creation_date: Local::now().to_utc(),
                                                        modified_date: None,
                                                        output_dir,
                                                        payload: match &td.payload {
                                                            Payload::FromPreviousStep {
                                                                ..
                                                            } => Payload::FromPreviousStep {
                                                                payload: task.result.take(),
                                                                task_id: task.id,
                                                            },
                                                            payload => payload.clone(),
                                                        },
                                                        result: None,
                                                        has_sub_task: false,
                                                        status: Status::Scheduled,
                                                    };
                                                    self.task_repository
                                                        .upsert(&next_task.id, &next_task)
                                                        .await?;
                                                    self.nc
                                                        .publish(
                                                            TASK_STATUS_CHANGE_EVENT(&next_task.id),
                                                            &next_task,
                                                        )
                                                        .await?;
                                                }
                                                None => {
                                                    job.status = task.status;
                                                }
                                            }
                                        }
                                    }
                                    self.job_repository.upsert(&job.id, &job).await?;
                                } else {
                                    error!("could not extract job, it's probably a problem!")
                                }
                            }
                        }
                        Err(e) => error!("could not extract task {e}"),
                    }
                    message.ack().await.map_err(|e| anyhow!("{e}"))?;
                }
                Err(e) => error!("could not get message {e}"),
            }
        }
        Ok(())
    }
    pub async fn start_consuming_sub_task(&self) -> anyhow::Result<()> {
        let mut messages = self.sub_task_event_consumer.messages().await?;
        while let Some(message) = messages.next().await {
            match message {
                Ok(message) => {
                    if let Ok(sub_task) = SubTask::deserialize_bytes(&message.payload) {
                        self.sub_task_repository
                            .upsert(&sub_task.id, &sub_task)
                            .await?;
                    }
                    message.ack().await.map_err(|e| anyhow!("{e}"))?;
                }
                Err(e) => error!("could not get message {e}"),
            }
        }
        Ok(())
    }
    pub async fn new_scheduled_job(
        &self,
        name: Option<String>,
        definition_id: String,
        target_url: Option<String>,
        cron_expr: String,
    ) -> Result<ScheduledJob, ApiError> {
        // validation stuff
        let Some(_) = self
            .job_definitions
            .iter()
            .find(|jd| jd.id == definition_id)
        else {
            return Err(ApiError::JobDefinitionNotFound);
        };
        let schedule = cron::Schedule::from_str(&cron_expr)
            .map_err(|e| ApiError::CronExpression(e.to_string()))?;
        let next_execution = schedule.upcoming(Utc).next();

        let scheduled_job = ScheduledJob {
            id: IdGenerator.get(),
            creation_date: Local::now().to_utc(),
            target_url,
            name,
            definition_id,
            next_execution,
            cron_expr,
        };
        self.scheduled_job_repository
            .insert_one(&scheduled_job)
            .await
            .map_err(|e| ApiError::NewScheduledJob(e.to_string()))?;
        Ok(scheduled_job)
    }
    pub async fn new_job(
        &self,
        definition_id: String,
        job_name: Option<String>,
        target_url: Option<String>,
    ) -> Result<Job, ApiError> {
        let Some(mut jd) = self
            .job_definitions
            .iter()
            .find(|jd| jd.id == definition_id)
            .cloned()
        else {
            return Err(ApiError::JobDefinitionNotFound);
        };
        let target_url = target_url.map(|t| REGEX_CLEAN_URL.replace_all(&t, "").trim().to_string());
        let job_id = IdGenerator.get();
        let job_root_dir = ROOT_OUTPUT_DIR_PB.join(&job_id);
        tokio::fs::create_dir_all(&job_root_dir)
            .await
            .map_err(|e| ApiError::NewJob(e.to_string()))?;
        // it has to be a scrape job
        if let Some(target_url) = &target_url {
            let Some(td) = jd
                .tasks
                .iter_mut()
                .find(|t| matches!(t.payload, Payload::ScrapeUrl(_)))
            else {
                return Err(ApiError::TaskDefinitionNotFound);
            };
            let Payload::ScrapeUrl(url) = &mut td.payload else {
                return Err(ApiError::TaskDefinitionNotFound);
            };
            *url = target_url.into();
        }
        let TaskDefinition {
            name,
            order,
            payload,
        } = jd.tasks[0].clone();
        let job = Job {
            id: job_id,
            name: if let Some(job_name) = job_name {
                job_name
            } else {
                jd.name.to_owned()
            },
            root_dir: job_root_dir,
            creation_date: Local::now().to_utc(),
            modified_date: None,
            status: Status::Pending,
            definition: jd,
            target_url,
        };
        self.job_repository
            .insert_one(&job)
            .await
            .map_err(|e| ApiError::NewJob(e.to_string()))?;

        let first_task = Task {
            id: IdGenerator.get(),
            order,
            job_id: job.id.clone(),
            name,
            creation_date: Local::now().to_utc(),
            modified_date: None,
            payload,
            result: None,
            status: Status::Pending,
            has_sub_task: false,
            output_dir: Default::default(),
        };

        self.task_repository
            .insert_one(&first_task)
            .await
            .map_err(|e| ApiError::NewJob(e.to_string()))?;
        self.nc
            .publish(TASK_STATUS_CHANGE_EVENT(&first_task.id), &first_task)
            .await
            .map_err(|e| ApiError::NewJob(e.to_string()))?;
        Ok(job)
    }
}
