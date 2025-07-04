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

    pub static SUBJECT_BINDING_TYPE: &str = "$type";
    pub static SUBJECT_BINDING: &str = "_sub_";
    pub static VAR_BINDING: &str = "_var_";
    pub static INDEX_ID_KEY: &str = "_id";
    pub static CONSTRUCT_PREFIX_URI: &str = "http://c.com/cst/";
    pub static CONSTRUCT: fn(&str) -> String = |s| format!("{CONSTRUCT_PREFIX_URI}{s}");
    pub static PREFIXES: &[(&str, &str)] = &[
        ("rdf:", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
        ("org:", "http://www.w3.org/ns/org#"),
        ("rdfs:", "http://www.w3.org/2000/01/rdf-schema#"),
        ("xsd:", "http://www.w3.org/2001/XMLSchema#"),
        ("foaf:", "http://xmlns.com/foaf/0.1/"),
        ("dc:", "http://purl.org/dc/elements/1.1/"),
        ("dcterms:", "http://purl.org/dc/terms/"),
        ("skos:", "http://www.w3.org/2004/02/skos/core#"),
        ("prov:", "http://www.w3.org/ns/prov#"),
        ("schema:", "http://schema.org/"),
        ("dcat:", "http://www.w3.org/ns/dcat#"),
        ("adms:", "http://www.w3.org/ns/adms#"),
        ("mu:", "http://mu.semte.ch/vocabularies/core/"),
        ("besluit:", "http://data.vlaanderen.be/ns/besluit#"),
        ("mandaat:", "http://data.vlaanderen.be/ns/mandaat#"),
        ("eli:", "http://data.europa.eu/eli/ontology#"),
        ("euvoc:", "http://publications.europa.eu/ontology/euvoc#"),
        ("mobiliteit:", "https://data.vlaanderen.be/ns/mobiliteit#"),
        ("ldes:", "http://w3id.org/ldes#"),
    ];
    pub static PREFIX_OR_NONE: fn(&str) -> Option<String> = |s| {
        PREFIXES.iter().find_map(|(p, uri)| {
            if s.contains(uri) {
                Some(s.replace(uri, p))
            } else {
                None
            }
        })
    };

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
        pub fn to_query_op(&self, idx: usize) -> String {
            let path = self
                .paths
                .iter()
                .map(|p| {
                    if let Some(prefixed) = PREFIX_OR_NONE(p) {
                        prefixed
                    } else if let Some(stripped) = p.strip_prefix('^') {
                        format!("^<{}>", stripped)
                    } else {
                        format!("<{p}>")
                    }
                })
                .collect::<Vec<_>>()
                .join("/");

            format!("{{?{SUBJECT_BINDING} {path} ?{VAR_BINDING}{idx}}}")
        }

        pub fn validate(&self) -> anyhow::Result<()> {
            if self.name == SUBJECT_BINDING_TYPE
                || self.name == SUBJECT_BINDING
                || self.name == INDEX_ID_KEY
                || self.name == VAR_BINDING
            {
                return Err(anyhow!(
                    "you cannot name a property with {SUBJECT_BINDING_TYPE} or {SUBJECT_BINDING} or {INDEX_ID_KEY} or {VAR_BINDING} in your config, because it's used internally."
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
        pub offset: Option<usize>,
        pub page: Option<usize>,
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
