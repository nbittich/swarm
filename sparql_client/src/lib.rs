use std::{collections::BTreeMap, fmt::Debug, sync::Arc, time::Duration};

use anyhow::anyhow;
use reqwest::{
    Client, Response,
    header::{ACCEPT, CONTENT_TYPE},
};
use serde::{Deserialize, Serialize};
use swarm_retryable_fut::retryable_fut;
use tracing::{debug, instrument};

pub static SPARQL_ENDPOINT: &str = "SPARQL_ENDPOINT";
pub static TARGET_GRAPH: &str = "TARGET_GRAPH";
pub static SPARQL_MAX_RETRY: &str = "SPARQL_MAX_RETRY";
pub static SPARQL_RETRY_DELAY_MILLIS: &str = "SPARQL_RETRY_DELAY_MILLIS";
pub static SPARQL_RESULT_JSON: &str = "application/sparql-results+json";
pub static SPARQL_UPDATE: &str = "application/sparql-update; charset=utf-8";
pub static REQUEST_TIMEOUT_SEC: &str = "REQUEST_TIMEOUT_SEC";
#[derive(Debug, Serialize, Deserialize)]
pub struct SparqlResponse {
    pub head: Head,
    pub results: SparqlResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Head {
    pub link: Option<Vec<String>>,
    pub vars: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SparqlResult {
    pub distinct: Option<bool>,
    pub bindings: Vec<BTreeMap<String, Binding>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Binding {
    pub datatype: Option<String>,
    #[serde(rename = "type")]
    pub rdf_type: String,
    pub value: String,
    #[serde(rename = "xml:lang")]
    pub lang: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SparqlClient {
    pub client: Client,
    pub endpoint: Arc<String>,
    pub max_retry: u32,
    pub delay_before_next_retry: Duration,
}
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SparqlUpdateType {
    Insert,
    Delete,
    NoOp,
}
impl SparqlClient {
    pub fn new() -> anyhow::Result<SparqlClient> {
        let endpoint = std::env::var(SPARQL_ENDPOINT)?;
        let max_retry = std::env::var(SPARQL_MAX_RETRY)
            .unwrap_or_else(|_| "5".into())
            .parse::<u32>()?;
        let delay_before_next_retry = std::env::var(SPARQL_RETRY_DELAY_MILLIS)
            .unwrap_or_else(|_| "5000".into())
            .parse::<u64>()
            .map(Duration::from_millis)?;
        let client = get_sparql_client()?;
        Ok(SparqlClient {
            client,
            endpoint: Arc::new(endpoint),
            max_retry,
            delay_before_next_retry,
        })
    }
    pub async fn _query<T>(
        &self,
        query: String,
        accept_header: Option<String>,
        transform: impl AsyncFn(Response) -> anyhow::Result<T> + Send + Sync,
    ) -> anyhow::Result<T> {
        debug!("{query}");
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        retryable_fut(
            self.max_retry as u64,
            self.delay_before_next_retry.as_secs(),
            async move || {
                let client = &client;
                let accept_header = accept_header.as_deref();
                let res = client
                    .post(endpoint.as_str())
                    .header(ACCEPT, accept_header.unwrap_or(SPARQL_RESULT_JSON))
                    .query(&[
                        ("query", query.as_str()),
                        ("format", accept_header.unwrap_or(SPARQL_RESULT_JSON)),
                    ])
                    .send()
                    .await
                    .and_then(|response| response.error_for_status())?;
                transform(res).await
            },
        )
        .await
    }

    #[instrument(level = "debug")]
    pub async fn query(&self, query: String) -> anyhow::Result<SparqlResponse> {
        self._query(query, None, async |response| {
            let r = response.json::<SparqlResponse>().await?;
            Ok(r)
        })
        .await
    }

    #[instrument(level = "debug")]
    pub async fn query_with_accept_header(
        &self,
        query: String,
        accept_header: Option<String>,
    ) -> anyhow::Result<(String, String)> {
        self._query(query, accept_header, async |response| {
            let ct = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|s| s.to_str().ok())
                .map(|s| s.to_string())
                .unwrap_or("text/plain".to_string());
            let r = response.text().await?;
            Ok((ct, r))
        })
        .await
    }
    async fn _update(&self, query: String) -> anyhow::Result<()> {
        debug!("{query}");
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        retryable_fut(
            self.max_retry as u64,
            self.delay_before_next_retry.as_secs(),
            async move || {
                let client = &client;
                let _ = client
                    .post(endpoint.as_str())
                    .header(ACCEPT, SPARQL_RESULT_JSON)
                    .header(CONTENT_TYPE, SPARQL_UPDATE)
                    .body(query.to_string())
                    .send()
                    .await
                    .and_then(|response| response.error_for_status())?;
                Ok(())
            },
        )
        .await
    }

    #[instrument(level = "debug")]
    pub async fn update(&self, query: String) -> anyhow::Result<()> {
        self._update(query).await
    }

    #[instrument(level = "debug")]
    pub async fn bulk_update(
        &self,
        target_graph: &str,
        triples: &[String],
        update_type: SparqlUpdateType,
    ) -> anyhow::Result<()> {
        if triples.is_empty() {
            debug!("no triples to update");
            return Ok(());
        }
        let operation = match update_type {
            SparqlUpdateType::Insert => "INSERT DATA",
            SparqlUpdateType::Delete => "DELETE DATA",
            SparqlUpdateType::NoOp => return Ok(()),
        };

        let q = make_update_query(target_graph, operation, triples);
        debug!("Executing query: \n{q}\n");
        match self._update(q).await {
            Ok(_) => Ok(()),
            Err(_) if triples.len() == 1 => {
                Err(anyhow!("Could not execute bulk update for {triples:?}"))
            }
            Err(e) => {
                debug!("could not execute sparql bulk update: {e}");
                let (a, b) = triples.split_at(triples.len() / 2);
                tokio::time::sleep(self.delay_before_next_retry).await;
                match tokio::join!(
                    Box::pin(self.bulk_update(target_graph, a, update_type)),
                    Box::pin(self.bulk_update(target_graph, b, update_type))
                ) {
                    (Ok(_), Ok(_)) => Ok(()),
                    (Ok(_), Err(q)) | (Err(q), Ok(_)) => Err(q),
                    (Err(q1), Err(q2)) => Err(anyhow!("{q1}\n;\n{q2}")),
                }
            }
        }
    }
}
fn make_update_query(target_graph: &str, operation: &str, triples: &[String]) -> String {
    let triples_str = format!("{}\n", triples.join("\n"));
    format!(
        r#"
                {operation} {{
                    GRAPH <{target_graph}> {{
                        {triples_str}
                    }}

                }}
        "#
    )
}
fn get_sparql_client() -> anyhow::Result<Client> {
    let timeout = std::env::var(REQUEST_TIMEOUT_SEC)
        .unwrap_or_else(|_| "30".into())
        .parse::<u64>()?;
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()?;
    Ok(client)
}
