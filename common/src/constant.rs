/* ENV VARS */
pub static MONGO_HOST: &str = "MONGO_HOST";
pub static MONGO_PORT: &str = "MONGO_PORT";
pub static MONGO_USERNAME: &str = "MONGO_USERNAME";
pub static MONGO_CONN_TIMEOUT: &str = "MONGO_CONN_TIMEOUT";
pub static MONGO_PASSWORD: &str = "MONGO_PASSWORD";
pub static MONGO_ADMIN_DATABASE: &str = "MONGO_ADMIN_DATABASE";
pub static NATS_CONNECTION_URL: &str = "NATS_CONNECTION_URL";
pub static NATS_USERNAME: &str = "NATS_USERNAME";
pub static NATS_PASSWORD: &str = "NATS_PASSWORD";
pub static NATS_MAX_RECONNECT: &str = "NATS_MAX_RECONNECT";
pub static NATS_ACK_WAIT: &str = "NATS_ACK_WAIT";
pub static APPLICATION_NAME: &str = "APPLICATION_NAME";
pub static BODY_SIZE_LIMIT: &str = "BODY_SIZE_LIMIT";
pub static SERVICE_HOST: &str = "SERVICE_HOST";
pub static SERVICE_PORT: &str = "SERVICE_PORT";
pub static JWT_SECRET: &str = "JWT_SECRET";
pub static ROOT_OUTPUT_DIR: &str = "ROOT_OUTPUT_DIR";
pub static JWT_EXPIRATION_TIME_SEC: &str = "JWT_EXPIRATION_TIME_SEC";
pub static MAX_RETRY: &str = "MAX_RETRY";
pub static DEFAULT_USER_AGENT: &str = "DEFAULT_USER_AGENT";
pub static DEFAULT_ACCEPT: &str = "DEFAULT_ACCEPT";
pub static REQUEST_TIMEOUT_SEC: &str = "REQUEST_TIMEOUT_SEC";
pub static MIN_DELAY_MILLIS: &str = "MIN_DELAY_MILLIS";
pub static MAX_DELAY_MILLIS: &str = "MAX_DELAY_MILLIS";
pub static MIN_DELAY_BEFORE_NEXT_RETRY_MILLIS: &str = "MIN_DELAY_BEFORE_NEXT_RETRY_MILLIS";
pub static MAX_DELAY_BEFORE_NEXT_RETRY_MILLIS: &str = "MAX_DELAY_BEFORE_NEXT_RETRY_MILLIS";
pub static CONNECTION_POOL_MAX_IDLE_PER_HOST: &str = "CONNECTION_POOL_MAX_IDLE_PER_HOST";
pub static BUFFER_BACK_PRESSURE: &str = "BUFFER_BACK_PRESSURE";
pub static INTERESTING_PROPERTIES: &str = "INTERESTING_PROPERTIES";
pub static HTTP_CACHE_PATH: &str = "HTTP_CACHE_PATH";
pub static UUID_COMPLEMENT_PREDICATE: &str = "UUID_COMPLEMENT_PREDICATE";
pub static CHUNK_SIZE: &str = "CHUNK_SIZE";
pub static MEILISEARCH_URL: &str = "MEILISEARCH_URL";
pub static MEILISEARCH_KEY: &str = "MEILISEARCH_KEY";
pub static INDEX_CONFIG_PATH: &str = "INDEX_CONFIG_PATH";
pub static INDEX_MAX_WAIT_FOR_TASK: &str = "INDEX_MAX_WAIT_FOR_TASK";
pub static RESET_INDEX: &str = "RESET_INDEX";
pub static RESET_INDEX_NAME: &str = "RESET_INDEX_NAME";
/* COLLECTION NAME */
pub static TASK_COLLECTION: &str = "tasks";
pub static SUB_TASK_COLLECTION: &str = "subTasks";
pub static SCHEDULED_JOB_COLLECTION: &str = "scheduledJobs";
pub static JOB_COLLECTION: &str = "jobs";
pub static USER_COLLECTION: &str = "users";
pub static UUID_COLLECTION: &str = "uuids";

/* NATS */
pub static TASK_EVENT_STREAM: &str = "EVENTSTASKS";
pub static SUB_TASK_EVENT_STREAM: &str = "EVENTSSUBTASKS";
pub static TASK_STATUS_CHANGE_SUBJECT: &str = "events.task.status.change.>";
pub static TASK_STATUS_CHANGE_EVENT: fn(s: &str) -> String =
    |s| format!("events.task.status.change.{s}");

pub static SUB_TASK_STATUS_CHANGE_SUBJECT: &str = "events.subtask.status.change.>";
pub static SUB_TASK_STATUS_CHANGE_EVENT: fn(s: &str) -> String =
    |s| format!("events.subtask.status.change.{s}");

pub static CRAWLER_CONSUMER: &str = "crawler";
pub static ADD_UUID_CONSUMER: &str = "addUuid";
pub static DIFF_CONSUMER: &str = "diff";
pub static CLEANUP_CONSUMER: &str = "cleanup";
pub static ARCHIVE_CONSUMER: &str = "archive";
pub static EXTRACTOR_CONSUMER: &str = "extractor";
pub static PUBLISH_CONSUMER: &str = "publish";
pub static INDEX_CONSUMER: &str = "index";
pub static JOB_MANAGER_CONSUMER: &str = "jobManager";

/* MISC */
pub static PUBLIC_TENANT: &str = "public";
pub static DEFAULT_BCRYPT_COST: u32 = 10;
pub static MANIFEST_FILE_NAME: &str = "manifest.json";

// NS
pub static PROV: fn(&str) -> String = |suffix| format!("http://www.w3.org/ns/prov#{suffix}");
pub static XSD: fn(&str) -> String = |suffix| format!("http://www.w3.org/2001/XMLSchema#{suffix}");
pub static RDF: fn(&str) -> String =
    |suffix| format!("http://www.w3.org/1999/02/22-rdf-syntax-ns#{suffix}");
pub static RDFS: fn(&str) -> String =
    |suffix| format!("http://www.w3.org/2000/01/rdf-schema#{suffix}");
