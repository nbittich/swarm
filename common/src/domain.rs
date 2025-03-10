use std::path::PathBuf;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UuidSubject {
    #[serde(rename = "_id")]
    pub subject_hash: String,
    pub id: String,
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
