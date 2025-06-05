use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hits_per_page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distinct: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes_to_retrieve: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes_to_crop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_marker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes_to_highlight: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_pre_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_post_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_matches_position: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matching_strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_ranking_score: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_ranking_score_details: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_score_threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes_to_search_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieve_vectors: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locales: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults<T> {
    pub hits: Vec<T>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub estimated_total_hits: Option<usize>,
    pub total_hits: Option<usize>,
    pub total_pages: Option<usize>,
    pub hits_per_page: Option<usize>,
    pub page: Option<usize>,
    pub facet_distribution: Option<HashMap<String, HashMap<String, usize>>>,
    pub facet_stats: Option<HashMap<String, FacetStat>>,
    pub processing_time_ms: Option<usize>,
    pub query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacetStat {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
}
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq, Copy)]
#[serde(rename_all = "camelCase")]
pub struct PaginationSetting {
    pub max_total_hits: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum Encoding {
    Gzip,
}
#[derive(Debug, Clone, Copy)]
pub enum ContentType {
    ApplicationJson,
    ApplicationNdJson,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    pub number_of_documents: usize,
    pub is_indexing: bool,
    pub field_distribution: HashMap<String, usize>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub uid: Option<u64>,
    pub index_uid: Option<String>,
    pub status: Option<String>,
    pub batch_uid: Option<u32>,
    #[serde(rename = "type")]
    pub task_type: Option<String>,
    pub canceled_by: Option<u64>,
    pub details: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
    pub duration: Option<String>,
    pub enqueued_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    pub task_uid: usize,
    pub index_uid: String,
    pub status: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub enqueued_at: String,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BatchResponse {
    pub results: Vec<Batch>,
    pub total: Option<u64>, // total number of batches matching the filter or query
    pub limit: Option<u64>, // Number of batches returned
    pub from: Option<u64>,  // uid of the first batch returned
    pub next: Option<u64>, // Value passed to from to view the next “page” of results. When the value of next is null, there are no more tasks to view
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    pub uid: u64,
    pub details: Option<Value>,
    pub stats: Option<Stats>,
    pub duration: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub progress: Option<Progress>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub received_documents: u64,
    pub indexed_documents: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub total_nb_tasks: u64,
    pub status: Status,
    pub types: Option<Value>,
    pub index_uids: Option<HashMap<String, u64>>,
    pub progress_trace: Option<Value>,
    pub write_channel_congestion: Option<Value>,
    pub internal_database_sizes: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub steps: Vec<Step>,
    pub percentage: f64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub current_step: Option<String>,
    pub finished: Option<u64>,
    pub total: Option<u64>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    enqueued: Option<u64>,
    processing: Option<u64>,
    succeeded: Option<u64>,
    failed: Option<u64>,
    canceled: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum BatchStatus {
    Enqueued,
    Processing,
    Succeeded,
    Failed,
    Canceled,
}
impl std::fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchStatus::Enqueued => write!(f, "enqueued"),
            BatchStatus::Processing => write!(f, "processing"),
            BatchStatus::Succeeded => write!(f, "succeeded"),
            BatchStatus::Failed => write!(f, "failed"),
            BatchStatus::Canceled => write!(f, "canceled"),
        }
    }
}
