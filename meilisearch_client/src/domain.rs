use serde::{Deserialize, Serialize};

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
    pub uid: u64,
    pub index_uid: String,
    pub status: String,
    pub task_type: String,
    pub canceled_by: Option<u64>,
    pub details: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
    pub duration: String,
    pub enqueued_at: String,
    pub started_at: String,
    pub finished_at: String,
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
