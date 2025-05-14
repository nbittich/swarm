/* CUSTOM ALLOC, disabled as it consumes more memory */
// pub use swarm_common::alloc;

use chrono::Local;
use rand::distr::{Distribution, Uniform};
use reqwest::{Client, Url, header::CONTENT_TYPE};
use std::{env::var, path::Path, str::FromStr, time::Duration};
use swarm_common::{
    IdGenerator, REGEX_CLEAN_JSESSIONID, REGEX_CLEAN_S_UUID, StreamExt, chunk_drain,
    compress::gzip,
    constant::{
        APPLICATION_NAME, CRAWLER_CONSUMER, MANIFEST_FILE_NAME, SUB_TASK_EVENT_STREAM,
        SUB_TASK_STATUS_CHANGE_EVENT, SUB_TASK_STATUS_CHANGE_SUBJECT, TASK_EVENT_STREAM,
        TASK_STATUS_CHANGE_EVENT, TASK_STATUS_CHANGE_SUBJECT,
    },
    debug,
    domain::{JsonMapper, Payload, ScrapeResult, Status, SubTask, SubTaskResult, Task, TaskResult},
    error, info,
    nats_client::{self, NatsClient},
    setup_tracing,
};
use tokio::{io::AsyncWriteExt, task::JoinSet};
use util::{Configuration, get_reqwest_client, make_config};

mod util;

#[tokio::main()]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let app_name = var(APPLICATION_NAME).unwrap_or_else(|_| "scraper".into());
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
        .create_durable_consumer(CRAWLER_CONSUMER, &task_event_stream)
        .await?;
    let client = get_reqwest_client()?;

    let mut messages = task_event_consumer.messages().await?;

    info!("app {app_name} started and ready to consume messages.");
    while let Some(message) = messages.next().await {
        match message {
            Ok(message) => match Task::deserialize_bytes(&message.payload) {
                Ok(mut task)
                    if matches!(task.payload, Payload::ScrapeUrl { .. })
                        && task.status == Status::Scheduled =>
                {
                    let nc = nc.clone();
                    let client = client.clone();
                    tokio::spawn(async move {
                        if let Err(e) = message.ack().await {
                            error!("{e}");
                        }
                        task.has_sub_task = true;
                        task.status = Status::Busy;
                        task.modified_date = Some(Local::now());
                        let _ = nc.publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task).await;
                        if handle_task(&nc, client, &mut task).await.is_some() {
                            let _ = nc.publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task).await;
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
pub async fn handle_task(nc: &NatsClient, client: Client, task: &mut Task) -> Option<()> {
    if let Payload::ScrapeUrl(url) = &task.payload {
        let res = crawl_website(&task.id, url, &task.output_dir, nc, client).await;
        task.modified_date = Some(Local::now());
        match res {
            Ok(
                r @ TaskResult::ScrapeWebsite {
                    success_count,
                    failure_count,
                    ..
                },
            ) => {
                if success_count == 0 {
                    task.status = Status::Failed(vec![format!(
                        "task did not succeed: success: {success_count}, failure: {failure_count}"
                    )]);
                } else {
                    task.result = Some(r);
                    task.status = Status::Success;
                }
            }
            Ok(other) => {
                task.status =
                    Status::Failed(vec![format!("task did not succeed: bad result! {other:?}")]);
            }
            Err(e) => {
                task.status = Status::Failed(vec![format!("task did not succeed: errored! {e}")]);
            }
        }
        Some(())
    } else {
        error!("task {task:?} didn't not contain any useful subject!");
        None
    }
}

pub async fn append_entry_manifest_file(
    dir_path: &Path,
    page_res: &ScrapeResult,
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

pub async fn crawl_website(
    task_id: &str,
    url: &str,
    path: &Path,
    nc: &NatsClient,
    client: Client,
) -> anyhow::Result<TaskResult> {
    let config = make_config(client, path.to_path_buf()).await?;
    let url = REGEX_CLEAN_JSESSIONID
        .replace_all(REGEX_CLEAN_S_UUID.replace_all(url, "").trim(), "")
        .trim()
        .to_string();
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut visited_urls = Vec::with_capacity(1500);
    let mut urls = Vec::with_capacity(1500);
    let mut tasks = JoinSet::new();
    urls.push(url);

    while !urls.is_empty() {
        for url_jobs in chunk_drain(&mut urls, config.max_concurrent_job) {
            for url in url_jobs {
                if visited_urls.iter().any(|k| k == &url) {
                    debug!("skipping {url} as it was already visited");
                    continue;
                }

                debug!("got url = {}", url);

                let config = config.clone();
                visited_urls.push(url.clone());

                tasks.spawn(async move {
                    debug!("sleeping before crawling {url}");

                    tokio::time::sleep(random_delay_millis(
                        config.min_delay_millis,
                        config.max_delay_millis,
                    )?)
                    .await;
                    crawl(&url, config).await
                });
            }
            //flush
            while let Some(handle) = tasks.join_next().await {
                match handle? {
                    Ok(result) => {
                        if let UrlProcessingResult::Processed((page_res, mut next_urls)) = result {
                            append_entry_manifest_file(path, &page_res).await?;
                            let st = SubTask {
                                id: IdGenerator.get(),
                                task_id: task_id.into(),
                                creation_date: Local::now(),
                                status: Status::Success,
                                result: Some(SubTaskResult::ScrapeUrl(page_res)),
                                ..Default::default()
                            };
                            let _ = nc.publish(SUB_TASK_STATUS_CHANGE_EVENT(&st.id), &st).await;
                            success_count += 1;
                            for nu in next_urls.drain(..) {
                                if !visited_urls.iter().any(|k| k == &nu) {
                                    urls.push(nu);
                                }
                            }
                        } else if let UrlProcessingResult::Ignored(url) = result {
                            // we don't want to persist these, they will be filtered later
                            // as their status will be success
                            debug!("{url} was ignored");
                        }
                    }
                    Err(e) => {
                        error!("{e}");
                        failure_count += 1;
                        let st = SubTask {
                            id: IdGenerator.get(),
                            task_id: task_id.into(),
                            creation_date: Local::now(),
                            status: Status::Failed(vec![format!("could not be visited: {e}")]),
                            ..Default::default()
                        };
                        let _ = nc.publish(SUB_TASK_STATUS_CHANGE_EVENT(&st.id), &st).await;
                    }
                }
            }
        }
    }
    visited_urls.clear();
    Ok(TaskResult::ScrapeWebsite {
        success_count,
        failure_count,
        manifest_file_path: path.join(MANIFEST_FILE_NAME),
    })
}

fn random_delay_millis(min_delay: u64, max_delay: u64) -> anyhow::Result<Duration> {
    let range = Uniform::new_inclusive(min_delay, max_delay)?;
    let mut rng = rand::rng();
    Ok(Duration::from_millis(range.sample(&mut rng)))
}
async fn crawl(url: &str, configuration: Configuration) -> anyhow::Result<UrlProcessingResult> {
    let task_url = REGEX_CLEAN_JSESSIONID
        .replace_all(REGEX_CLEAN_S_UUID.replace_all(url, "").trim(), "")
        .trim()
        .to_string();
    debug!("processing {task_url}");

    let base_iri = {
        let base = Url::parse(&task_url)?;
        let base = if let Some(domain) = base.host_str() {
            format!("{}://{}", base.scheme(), domain)
        } else {
            base.to_string()
        };
        if base.ends_with("/") {
            debug!("{}", &base[0..base.len() - 1]);
            Url::from_str(&base[0..base.len() - 1])?
        } else {
            Url::from_str(&base)?
        }
    };

    for ignore_extension in configuration.ignore_extensions.iter() {
        if task_url.ends_with(ignore_extension) {
            return Ok(UrlProcessingResult::Ignored(task_url));
        }
    }

    let mut attempt = 0;

    while attempt < configuration.max_retry {
        attempt += 1;
        match configuration.client.get(&task_url).send().await {
            Ok(response)
                if response.status().is_success() || response.status().is_redirection() =>
            {
                if response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|h| h.to_str().ok())
                    .filter(|h| {
                        configuration
                            .allowed_content_types
                            .iter()
                            .any(|ac| h.contains(ac))
                    })
                    .is_none()
                {
                    debug!(
                        "bad content type {:?}",
                        response.headers().get(CONTENT_TYPE)
                    );
                    return Ok(UrlProcessingResult::Ignored(task_url));
                }

                let html = response.text_with_charset("utf-8").await?;
                if html.trim().is_empty() {
                    return Ok(UrlProcessingResult::Ignored(task_url));
                }

                // debug!("saving url {task_url} to file");
                let file_name = format!("{}.html", IdGenerator.get());

                let path = {
                    let path = configuration.folder_path.join(file_name);
                    tokio::fs::write(&path, &html).await?;
                    gzip(&path, true).await?
                };

                let document = scraper::Html::parse_document(&html);

                let mut next_urls = Vec::with_capacity(configuration.buffer);
                // html redirect using meta refresh
                if let Some(element) = document.select(&configuration.redirect_selector).next() {
                    if let Some(content) = element.value().attr("content") {
                        if let Some(url_part) = content.split("url=").nth(1) {
                            let redirect_url = url_part.trim();
                            debug!("Redirect URL found: {}", redirect_url);
                            next_urls.push(redirect_url.to_string());
                        }
                    }
                }

                for a_property in document.select(&configuration.href_selector) {
                    if let Some(interesting_properties) =
                        configuration.interesting_properties.as_ref()
                    {
                        if a_property
                            .attr("property")
                            .map(|s| s.to_lowercase())
                            .inspect(|p| debug!("property {p}"))
                            .filter(|p| interesting_properties.iter().any(|ip| p.contains(ip)))
                            .is_none()
                        {
                            // debug!("{a_property:?} doesn't have interesting properties");
                            continue;
                        }
                    }
                    match a_property.attr("href").map(str::to_owned) {
                        Some(url) => match Url::parse(&url) {
                            Ok(iri) if iri.scheme() == "https" || iri.scheme() == "http" => {
                                next_urls.push(url);
                            }
                            Ok(iri) if !iri.has_host() => {
                                debug!("does not have a host {url}");
                                if let Ok(iri) = base_iri.join(&url) {
                                    next_urls.push(iri.to_string());
                                }
                            }
                            Ok(iri) => {
                                debug!("scheme {}", iri.scheme());
                            }
                            Err(e) => {
                                debug!(
                                    "could not parse url {url}, attempt to join with base {base_iri} {e}"
                                );
                                if let Ok(iri) = base_iri.join(&url) {
                                    debug!("url could be resolved as {iri:?}");
                                    next_urls.push(iri.to_string());
                                }
                            }
                        },
                        None => debug!("could not extract url {a_property:?}"),
                    }
                }

                return Ok(UrlProcessingResult::Processed((
                    ScrapeResult {
                        base_url: task_url,
                        path,
                        creation_date: Local::now(),
                    },
                    next_urls,
                )));
            }
            Ok(response) if response.status().is_server_error() => {
                debug!("error response {}", response.status());
            }
            Ok(response) if response.status().is_client_error() => {
                debug!("client error response {}", response.status());
                break;
            }
            Ok(response) => {
                debug!("weird response {}", response.status());
            }
            Err(e) => {
                debug!("errored {e}")
            }
        };
        debug!("delay before next attempt");
        tokio::time::sleep(random_delay_millis(
            configuration.min_delay_before_next_retry_millis,
            configuration.max_delay_before_next_retry_millis,
        )?)
        .await;
    }
    Ok(UrlProcessingResult::Ignored(task_url))
}

enum UrlProcessingResult {
    Processed((ScrapeResult, Vec<String>)),
    Ignored(String),
}
