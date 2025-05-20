pub mod domain;
use std::{fmt::Debug, sync::Arc, time::Duration};

use domain::{
    ContentType, Encoding, HealthStatus, IndexStats, PaginationSetting, SearchQuery, SearchResults,
    Task, TaskInfo,
};
use http::Extensions;
use reqwest::{
    Client, Request, Response,
    header::{AUTHORIZATION, CONTENT_ENCODING, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT},
};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Middleware, Next, Result};
use serde::de::DeserializeOwned;
use tracing::debug;

pub static MEILISEARCH_CLIENT_MAX_RETRY: &str = "MEILISEARCH_CLIENT_MAX_RETRY";
pub static MEILISEARCH_CLIENT_RETRY_DELAY_MILLIS: &str = "MEILISEARCH_CLIENT_RETRY_DELAY_MILLIS";
pub static MEILISEARCH_CLIENT_REQUEST_TIMEOUT_SEC: &str = "MEILISEARCH_CLIENT_REQUEST_TIMEOUT_SEC";

struct LoggingMiddleware;

#[async_trait::async_trait]
impl Middleware for LoggingMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        debug!("Request started {req:?}");
        let res = next.run(req, extensions).await;
        debug!("Result: {res:?}");
        res
    }
}
#[derive(Debug, Clone)]
pub struct MeilisearchClient {
    client: ClientWithMiddleware,
    pub endpoint: Arc<String>,
    pub max_retry: usize,
    pub delay_before_next_retry: Duration,
}
impl MeilisearchClient {
    pub fn new(
        host: impl Into<String>,
        key: impl Into<String>,
    ) -> anyhow::Result<MeilisearchClient> {
        let max_retry = std::env::var(MEILISEARCH_CLIENT_MAX_RETRY)
            .unwrap_or_else(|_| "5".into())
            .parse::<usize>()?;
        let delay_before_next_retry = std::env::var(MEILISEARCH_CLIENT_RETRY_DELAY_MILLIS)
            .unwrap_or_else(|_| "5000".into())
            .parse::<u64>()
            .map(Duration::from_millis)?;
        let client = ClientBuilder::new(
            Client::builder()
                .default_headers(HeaderMap::from_iter(
                    [
                        (USER_AGENT, HeaderValue::from_static("Swarm")),
                        (
                            AUTHORIZATION,
                            HeaderValue::from_str(&format!("Bearer {}", key.into()))?,
                        ),
                        (CONTENT_TYPE, HeaderValue::from_static("application/json")),
                    ]
                    .into_iter(),
                ))
                .build()?,
        )
        .with(LoggingMiddleware)
        .build();
        Ok(MeilisearchClient {
            client,
            endpoint: Arc::new(host.into()),
            max_retry,
            delay_before_next_retry,
        })
    }
    pub async fn health(&self) -> anyhow::Result<HealthStatus> {
        let response = self
            .client
            .get(format!("{}/health", self.endpoint))
            .send()
            .await?;
        let response = response.error_for_status()?;
        let body = response.json().await?;
        Ok(body)
    }
    pub async fn set_filterable_attributes(
        &self,
        index: &str,
        attributes: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> anyhow::Result<()> {
        let response = self
            .client
            .put(format!(
                "{}/indexes/{}/settings/filterable-attributes",
                self.endpoint, index
            ))
            .body(serde_json::to_string(
                &attributes
                    .into_iter()
                    .map(|m| m.as_ref().to_string())
                    .collect::<Vec<_>>(),
            )?)
            .send()
            .await?;
        response.error_for_status()?;
        Ok(())
    }
    pub async fn set_pagination(
        &self,
        index: &str,
        pagination: PaginationSetting,
    ) -> anyhow::Result<()> {
        let response = self
            .client
            .patch(format!(
                "{}/indexes/{}/settings/pagination",
                self.endpoint, index
            ))
            .body(serde_json::to_string(&pagination)?)
            .send()
            .await?;
        response.error_for_status()?;
        Ok(())
    }
    pub async fn delete_all_documents(&self, index: &str) -> anyhow::Result<TaskInfo> {
        let res = self
            .client
            .delete(format!("{}/indexes/{}/documents", self.endpoint, index))
            .send()
            .await?;
        let res = res.error_for_status()?;

        let body = res.json().await?;
        Ok(body)
    }
    pub async fn get_task(&self, task_id: usize) -> anyhow::Result<Task> {
        let res = self
            .client
            .get(format!("{}/tasks/{}", self.endpoint, task_id))
            .send()
            .await?;
        let res = res.error_for_status()?;
        let body = res.json().await?;
        Ok(body)
    }
    pub async fn wait_for_task(
        &self,
        task_id: usize,
        interval: Option<Duration>,
        timeout: Option<Duration>,
    ) -> anyhow::Result<()> {
        let interval = interval.unwrap_or_else(|| Duration::from_millis(100));
        let timeout = timeout.unwrap_or_else(|| Duration::from_millis(30_000));

        let mut elapsed_time = Duration::new(0, 0);

        let mut ticker = tokio::time::interval(interval);
        while timeout > elapsed_time {
            match self.get_task(task_id).await {
                Ok(ref task) => match task.status.as_str() {
                    "failed" => return Err(anyhow::anyhow!(serde_json::to_string(&task.error)?)),
                    "succeeded" => return Ok(()),
                    "enqueued" | "processing" => {
                        elapsed_time += interval;
                        ticker.tick().await;
                    }
                    status => return Err(anyhow::anyhow!("invalid status {status}")),
                },
                Err(error) => return Err(error),
            };
        }

        Err(anyhow::anyhow!("meilisearch timed out"))
    }
    pub async fn add_or_replace_documents(
        &self,
        index: &str,
        primary_key: &str,
        payload: Vec<u8>,
        content_type: Option<ContentType>,
        encoding: Option<Encoding>,
    ) -> anyhow::Result<TaskInfo> {
        let content_type = match content_type.unwrap_or(ContentType::ApplicationJson) {
            ContentType::ApplicationJson => "application/json",
            ContentType::ApplicationNdJson => "application/x-ndjson",
        };
        let encoding = encoding.map(|e| match e {
            Encoding::Gzip => "gzip",
        });
        let mut req = self
            .client
            .post(format!(
                "{}/indexes/{}/documents?primaryKey={}",
                self.endpoint, index, primary_key
            ))
            .header(CONTENT_TYPE, HeaderValue::from_static(content_type));

        if let Some(encoding) = encoding {
            req = req.header(CONTENT_ENCODING, HeaderValue::from_static(encoding));
        }

        let res = req.body(payload).send().await?;
        let res = res.error_for_status()?;

        let body = res.json().await?;
        Ok(body)
    }
    pub async fn delete_documents(&self, index: &str, uids: &[String]) -> anyhow::Result<TaskInfo> {
        let res = self
            .client
            .post(format!(
                "{}/indexes/{}/documents/delete-batch",
                self.endpoint, index
            ))
            .body(serde_json::to_string(uids)?)
            .send()
            .await?;
        let res = res.error_for_status()?;

        let body = res.json().await?;
        Ok(body)
    }
    pub async fn search<T: 'static + DeserializeOwned + Send + Sync>(
        &self,
        index: &str,
        body: &SearchQuery,
    ) -> anyhow::Result<SearchResults<T>> {
        let res = self
            .client
            .post(format!("{}/indexes/{}/search", self.endpoint, index))
            .body(serde_json::to_string(body)?)
            .send()
            .await?;
        let res = res.error_for_status()?;

        let body = res.json().await?;
        Ok(body)
    }
    pub async fn get_stats(&self, index: &str) -> anyhow::Result<IndexStats> {
        let res = self
            .client
            .get(format!("{}/indexes/{}/stats", self.endpoint, index))
            .send()
            .await?;
        let res = res.error_for_status()?;

        let body = res.json().await?;
        Ok(body)
    }
}
