/* CUSTOM ALLOC, disabled as it consumes more memory */
//pub use swarm_common::alloc;

use std::{borrow::Cow, env::var, error::Error, fmt::Display, path::Path, time::Duration};
mod fix_stmt;
use chrono::Local;
use fix_stmt::fix_triples;
use graph_rdfa_processor::RdfaGraph;
use swarm_common::{
    constant::{
        APPLICATION_NAME, EXTRACTOR_CONSUMER, MANIFEST_FILE_NAME, PROV, SUB_TASK_EVENT_STREAM,
        SUB_TASK_STATUS_CHANGE_EVENT, SUB_TASK_STATUS_CHANGE_SUBJECT, TASK_EVENT_STREAM,
        TASK_STATUS_CHANGE_EVENT, TASK_STATUS_CHANGE_SUBJECT,
    },
    debug,
    domain::{
        JsonMapper, NTripleResult, Payload, ScrapeResult, Status, SubTask, SubTaskResult, Task,
        TaskResult,
    },
    error, info,
    nats_client::{self, NatsClient},
    setup_tracing, IdGenerator, StreamExt,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    task::JoinSet,
};
use tortank::turtle::turtle_doc::TurtleDoc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "extractor".into());
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
        .create_durable_consumer(EXTRACTOR_CONSUMER, &task_event_stream)
        .await?;

    let mut messages = task_event_consumer.messages().await?;

    info!("app {app_name} started and ready to consume messages.");
    while let Some(message) = messages.next().await {
        match message {
            Ok(message) => match Task::deserialize_bytes(&message.payload) {
                Ok(mut task)
                    if matches!(
                        &task.payload,
                        Payload::FromPreviousStep {
                            payload: Some(TaskResult::ScrapeWebsite { .. }),
                            ..
                        }
                    ) && task.status == Status::Scheduled =>
                {
                    let nc = nc.clone();

                    tokio::spawn(async move {
                        if let Err(e) = message.ack().await {
                            error!("{e}");
                            return;
                        }
                        task.has_sub_task = true;
                        task.status = Status::Busy;
                        task.modified_date = Some(Local::now());
                        let _ = nc.publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task).await;
                        match handle_task(&nc, &mut task).await {
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
            Err(e) => error!("could not get message {e}"),
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

async fn handle_task(nc: &NatsClient, task: &mut Task) -> anyhow::Result<Option<()>> {
    if let Payload::FromPreviousStep {
        payload: Some(TaskResult::ScrapeWebsite {
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
        let mut tasks = JoinSet::new();

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
            tasks.spawn(tokio::spawn(async move {
                match extract_rdfa(&line, &out_dir).await {
                    Ok(o) => Ok((sub_task, o)),
                    Err(ExtractRDFaError { base_url, error }) => {
                        sub_task.result = Some(SubTaskResult::NTriple(NTripleResult {
                            base_url,
                            len: 0,
                            path: Default::default(),
                            creation_date: Local::now(),
                        }));
                        Err((sub_task, error))
                    }
                }
            }));
            // sleep just a little to avoid using all the cpu
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        while let Some(handle) = tasks.join_next().await {
            let mut sub_task = match handle?? {
                Ok((mut sub_task, res @ NTripleResult { len: 0, .. })) => {
                    sub_task.status = Status::Failed(vec!["did not extract any data".into()]);
                    sub_task.result = Some(SubTaskResult::NTriple(res));
                    failure_count += 1;
                    sub_task
                }
                Ok((mut sub_task, triples)) => {
                    append_entry_manifest_file(&task.output_dir, &triples).await?;
                    success_count += 1;
                    sub_task.status = Status::Success;
                    sub_task.result = Some(SubTaskResult::NTriple(triples));
                    sub_task
                }

                Err((mut sub_task, e)) => {
                    failure_count += 1;
                    sub_task.status =
                        Status::Failed(vec![format!("error during extraction! {e:?}")]);
                    sub_task
                }
            };
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
            task.result = Some(TaskResult::ExtractRDFa {
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
#[derive(Debug)]
struct ExtractRDFaError {
    base_url: String,
    error: String,
}
impl Display for ExtractRDFaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {}", self.base_url, self.error)
    }
}
impl Error for ExtractRDFaError {}
async fn extract_rdfa(line: &str, output_dir: &Path) -> Result<NTripleResult, ExtractRDFaError> {
    let payload = ScrapeResult::deserialize(line).map_err(|e| ExtractRDFaError {
        base_url: "N/A".into(),
        error: e.to_string(),
    })?;
    let html_file =
        tokio::fs::read_to_string(payload.path)
            .await
            .map_err(|e| ExtractRDFaError {
                base_url: payload.base_url.to_string(),
                error: e.to_string(),
            })?;

    let ttl = RdfaGraph::parse_str(&html_file, &payload.base_url, None).map_err(|e| {
        ExtractRDFaError {
            base_url: payload.base_url.to_string(),
            error: e.to_string(),
        }
    })?;
    let doc = TurtleDoc::try_from((ttl.as_str(), None)).map_err(|e| ExtractRDFaError {
        base_url: payload.base_url.to_string(),
        error: e.to_string(),
    })?;
    let subjects = doc.all_subjects();
    let mut doc = doc
        .difference(&TurtleDoc::default())
        .map_err(|e| ExtractRDFaError {
            base_url: payload.base_url.to_string(),
            error: e.to_string(),
        })?;

    for subject in subjects {
        doc.add_statement(
            subject,
            tortank::turtle::turtle_doc::Node::Iri(Cow::Owned(PROV("wasDerivedFrom"))),
            tortank::turtle::turtle_doc::Node::Iri(Cow::Owned(payload.base_url.clone())),
        );
    }
    doc = fix_triples(doc).map_err(|e| ExtractRDFaError {
        base_url: payload.base_url.to_string(),
        error: e.to_string(),
    })?;
    let id = IdGenerator.get();

    let path = output_dir.join(format!("{id}.ttl"));
    tokio::fs::write(&path, doc.to_string())
        .await
        .map_err(|e| ExtractRDFaError {
            base_url: payload.base_url.to_string(),
            error: e.to_string(),
        })?;
    Ok(NTripleResult {
        base_url: payload.base_url,
        len: doc.len(),
        path,
        creation_date: Local::now(),
    })
}
