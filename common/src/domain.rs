use std::path::PathBuf;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::mongo::Identifiable;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobDefinition {
    pub id: String,
    pub name: String,
    pub allow_concurrent_run: bool, // similar to singleton-job
    pub tasks: Vec<TaskDefinition>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct AuthBody {
    pub access_token: String,
    pub token_type: String,
}

impl AuthBody {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthPayload {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskDefinition {
    pub name: String,
    pub order: usize,
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub target_url: Option<String>, // initial url
    pub root_dir: PathBuf,
    pub creation_date: DateTime<Local>,
    pub modified_date: Option<DateTime<Local>>,
    pub status: Status,
    pub definition: JobDefinition,
}
impl Identifiable for Job {
    fn get_id(&self) -> &str {
        &self.id
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledJob {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: Option<String>,
    pub creation_date: DateTime<Local>,
    pub next_execution: Option<DateTime<Local>>,
    pub task_definition: TaskDefinition,
    pub definition_id: String,
    pub cron_expr: String,
}
impl Identifiable for ScheduledJob {
    fn get_id(&self) -> &str {
        &self.id
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(rename = "_id")]
    pub id: String,
    pub order: usize,
    pub job_id: String,
    pub name: String,
    pub creation_date: DateTime<Local>,
    pub modified_date: Option<DateTime<Local>>,
    pub payload: Payload,
    pub result: Option<TaskResult>,
    pub has_sub_task: bool,
    pub status: Status,
    pub output_dir: PathBuf,
}

impl Identifiable for Task {
    fn get_id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubTask {
    #[serde(rename = "_id")]
    pub id: String,
    pub task_id: String,
    pub creation_date: DateTime<Local>,
    pub modified_date: Option<DateTime<Local>>,
    pub status: Status,
    pub result: Option<SubTaskResult>,
}

impl Identifiable for SubTask {
    fn get_id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "value")]
pub enum Payload {
    #[default]
    None,
    Archive,
    Cleanup(Status),
    ScrapeUrl(String),
    #[serde(rename_all = "camelCase")]
    FromPreviousStep {
        task_id: String,
        payload: Option<TaskResult>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "value")]
pub enum TaskResult {
    #[serde(rename_all = "camelCase")]
    ScrapeWebsite {
        success_count: usize,
        failure_count: usize,
        manifest_file_path: PathBuf,
    },
    #[serde(rename_all = "camelCase")]
    ExtractRDFa {
        success_count: usize,
        failure_count: usize,
        manifest_file_path: PathBuf,
    },
    #[serde(rename_all = "camelCase")]
    FilterSHACL {
        success_count: usize,
        failure_count: usize,
        manifest_file_path: PathBuf,
    },
    #[serde(rename_all = "camelCase")]
    ComplementWithUuid {
        success_count: usize,
        failure_count: usize,
        manifest_file_path: PathBuf,
    },
    #[serde(rename_all = "camelCase")]
    Diff {
        success_count: usize,
        failure_count: usize,
        manifest_file_path: PathBuf,
    },
    #[serde(rename_all = "camelCase")]
    Publish {
        removed_triple_file_path: PathBuf,
        intersect_triple_file_path: PathBuf,
        inserted_triple_file_path: PathBuf,
        failed_query_file_path: PathBuf,
        diff_manifest_file_path: PathBuf,
    },
    Json(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "value")]
pub enum SubTaskResult {
    ScrapeUrl(ScrapeResult),
    NTriple(NTripleResult),
    Diff(DiffResult),
    Json(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScrapeResult {
    pub base_url: String,
    pub path: PathBuf,
    pub creation_date: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub base_url: String,
    pub new_insert_path: Option<PathBuf>,
    pub intersect_path: Option<PathBuf>,
    pub to_remove_path: Option<PathBuf>,
    pub creation_date: DateTime<Local>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NTripleResult {
    pub base_url: String,
    pub len: usize,
    pub path: PathBuf,
    pub creation_date: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "value")]
pub enum Status {
    #[default]
    Pending,
    Scheduled,
    Archived,
    Busy,
    Success,
    Failed(Vec<String>),
}
impl Status {
    pub fn get_type(&self) -> &'static str {
        match self {
            Status::Pending => "pending",
            Status::Scheduled => "scheduled",
            Status::Archived => "archived",
            Status::Busy => "busy",
            Status::Success => "success",
            Status::Failed(_) => "failed",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id")]
    pub id: String,
    pub username: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub service_account: bool,
}

impl Identifiable for User {
    fn get_id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UuidSubject {
    #[serde(rename = "_id")]
    pub subject_hash: String,
    pub id: String,
}

impl Identifiable for UuidSubject {
    fn get_id(&self) -> &str {
        &self.id
    }
}

pub trait JsonMapper: Serialize + DeserializeOwned + Unpin + Send + Sync {
    fn serialize(&self) -> anyhow::Result<String> {
        let r = serde_json::to_string(&self)?;
        Ok(r)
    }
    fn serialize_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let r = serde_json::to_vec(&self)?;
        Ok(r)
    }
    fn deserialize(o: &str) -> anyhow::Result<Self> {
        let r = serde_json::from_str(o)?;
        Ok(r)
    }
    fn deserialize_bytes(o: &[u8]) -> anyhow::Result<Self> {
        let r = serde_json::from_slice(o)?;
        Ok(r)
    }
}

impl JsonMapper for Job {}
impl JsonMapper for UuidSubject {}
impl JsonMapper for Task {}
impl JsonMapper for Status {}
impl JsonMapper for ScheduledJob {}
impl JsonMapper for ScrapeResult {}
impl JsonMapper for NTripleResult {}
impl JsonMapper for Payload {}
impl JsonMapper for TaskResult {}
impl JsonMapper for DiffResult {}
impl JsonMapper for SubTaskResult {}
impl JsonMapper for SubTask {}
impl JsonMapper for TaskDefinition {}
impl JsonMapper for JobDefinition {}
impl JsonMapper for Vec<JobDefinition> {}
impl JsonMapper for User {}

pub mod index_config {
    use std::collections::BTreeMap;

    use anyhow::anyhow;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    const SUBJECT_BINDING_TYPE: &str = "$type";
    pub static INDEX_ID_KEY: &str = "_id";

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct IndexConfiguration {
        pub name: String,
        pub rdf_type: Vec<String>,
        pub on_path: String,
        pub properties: Vec<RdfProperty>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct RdfProperty {
        pub name: String,
        pub paths: Vec<String>,
        pub optional: bool,
        pub config: Option<RdfPropertyConfig>,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct RdfPropertyConfig {
        pub visible: bool,
        pub js_type: Option<JsType>,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub enum JsType {
        Date,
        String,
        Number,
        Url,
    }

    impl RdfProperty {
        pub fn to_query_op(&self, subject: &str) -> String {
            let path = self
                .paths
                .iter()
                .map(|p| {
                    if let Some(stripped) = p.strip_prefix('^') {
                        format!("^<{}>", stripped)
                    } else {
                        format!("<{p}>")
                    }
                })
                .collect::<Vec<_>>()
                .join("/");
            if self.optional {
                format!("OPTIONAL {{ <{subject}> {path} ?{} }}", self.name)
            } else {
                format!("<{subject}> {path} ?{}", self.name)
            }
        }

        pub fn validate(&self) -> anyhow::Result<()> {
            if self.name == SUBJECT_BINDING_TYPE || self.name == INDEX_ID_KEY {
                return Err(anyhow!(
                    "you cannot name a property with {SUBJECT_BINDING_TYPE} or {INDEX_ID_KEY} in your config, because it's used internally."
                ));
            }
            Ok(())
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SearchQueryRequest {
        pub query: Option<SearchQueryType>,
        pub neg: bool,
        pub sort_by: Option<String>,
        pub sort_direction: Option<Order>,
        pub filters: Option<String>,
        pub limit: usize,
        pub page: usize,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[serde(tag = "type", content = "value")]
    pub enum SearchQueryType {
        Word(String),
        Phrase(String),
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum Order {
        Asc,
        Desc,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SearchQueryResponse {
        pub hits: Vec<BTreeMap<String, Value>>,
        pub total_hits: Option<usize>,
        pub total_pages: Option<usize>,
        pub page: Option<usize>,
        pub limit: Option<usize>,
    }
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct IndexStatistics {
        pub number_of_documents: usize,
    }

    impl SearchQueryRequest {
        pub fn get_formatted_query(&self) -> Option<String> {
            self.query.as_ref().map(|query| match query {
                SearchQueryType::Word(w) => format!("{}{w}", if self.neg { "-" } else { "" }),
                SearchQueryType::Phrase(p) => format!("{}\"{p}\"", if self.neg { "-" } else { "" }),
            })
        }
        pub fn get_formatted_sort(&self) -> String {
            if let (Some(sort_by), Some(direction)) = (&self.sort_by, &self.sort_direction) {
                format!(
                    "{sort_by}:{}",
                    if matches!(direction, Order::Asc) {
                        "asc"
                    } else {
                        "desc"
                    }
                )
            } else {
                "".to_string()
            }
        }
    }
}

#[cfg(test)]
mod test {

    use crate::{IdGenerator, domain::JsonMapper};

    use super::{JobDefinition, Payload, TaskDefinition};

    #[test]
    fn jd_test() {
        let jd = JobDefinition {
            id: IdGenerator.get(),
            name: "Harvest".to_owned(),
            allow_concurrent_run: false,
            tasks: vec![TaskDefinition {
                name: "archive".to_string(),
                order: 0,
                payload: Payload::Archive,
            }],
        };
        println!("{}", jd.serialize().unwrap());
    }
}
