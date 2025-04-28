use std::{collections::BTreeMap, ops::Mul, time::Duration};

use anyhow::anyhow;
use reqwest::{
    Client,
    header::{ACCEPT, CONTENT_TYPE},
};
use serde::{Deserialize, Serialize};
use tracing::debug;

pub static SPARQL_ENDPOINT: &str = "SPARQL_ENDPOINT";
pub static TARGET_GRAPH: &str = "TARGET_GRAPH";
pub static SPARQL_MAX_RETRY: &str = "SPARQL_MAX_RETRY";
pub static SPARQL_RETRY_DELAY_MILLIS: &str = "SPARQL_RETRY_DELAY_MILLIS";
pub static SPARQL_RESULT_JSON: &str = "application/sparql-results+json";
pub static SPARQL_UPDATE: &str = "application/sparql-update; charset=utf-8";
static REQUEST_TIMEOUT_SEC: &str = "REQUEST_TIMEOUT_SEC";

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
    pub endpoint: String,
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
            endpoint,
            max_retry,
            delay_before_next_retry,
        })
    }
    pub async fn query(&self, query: &str) -> anyhow::Result<SparqlResponse> {
        debug!("{query}");
        let mut retry_count = 0;
        let mut err = Err(anyhow!("unexpected error"));
        while retry_count < self.max_retry {
            if retry_count > 0 {
                let wait_before_next_retry = self.delay_before_next_retry.mul(retry_count);
                debug!("sleeping for {wait_before_next_retry:?} millis before trying again...");
                tokio::time::sleep(wait_before_next_retry).await;
            }
            match self
                .client
                .post(&self.endpoint)
                .header(ACCEPT, SPARQL_RESULT_JSON)
                .query(&[("query", query), ("format", SPARQL_RESULT_JSON)])
                .send()
                .await
                .and_then(|response| response.error_for_status())
            {
                Ok(response) => match response.json::<SparqlResponse>().await {
                    Ok(sr) => return Ok(sr),
                    Err(e) => {
                        retry_count += 1;
                        err = Err(anyhow!("{e}"));
                    }
                },

                Err(e) => {
                    retry_count += 1;
                    err = Err(anyhow!("{e}"));
                }
            }
        }
        err
    }
    pub async fn update(&self, query: &str) -> anyhow::Result<()> {
        debug!("{query}");
        let mut retry_count = 0;
        let mut err = Err(anyhow!("unexpected error"));
        while retry_count < self.max_retry {
            if retry_count > 0 {
                let wait_before_next_retry = self.delay_before_next_retry.mul(retry_count);
                debug!("sleeping for {wait_before_next_retry:?} millis before trying again...");
                tokio::time::sleep(wait_before_next_retry).await;
            }
            match self
                .client
                .post(&self.endpoint)
                .header(ACCEPT, SPARQL_RESULT_JSON)
                .header(CONTENT_TYPE, SPARQL_UPDATE)
                .body(query.to_string())
                .send()
                .await
                .and_then(|response| response.error_for_status())
            {
                Ok(_) => return Ok(()),

                Err(e) => {
                    retry_count += 1;
                    err = Err(anyhow!("{e}"));
                }
            }
        }
        err
    }
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
        match self.update(&q).await {
            Ok(_) => Ok(()),
            Err(_) if triples.len() == 1 => Err(anyhow!("{q}")),
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
