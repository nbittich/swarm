use std::{path::PathBuf, sync::LazyLock};

use axum::{
    Json, RequestPartsExt,
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::{DateTime, Duration, Local};
use jsonwebtoken::{DecodingKey, EncodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use swarm_common::{
    constant::{JWT_EXPIRATION_TIME_SEC, JWT_SECRET, ROOT_OUTPUT_DIR},
    domain::TaskDefinition,
};
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    pub sub: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub exp: usize,
}

#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
}
#[derive(Debug)]
pub enum ApiError {
    AllJobs(String),
    AllScheduledJobs(String),
    SparqlError(String),
    SearchError(String),
    GetLastPublications(String),
    DeleteJob(String),
    GetJob(String),
    DeleteScheduledJob(String),
    AllTasks(String),
    AllSubTasks(String),
    Download(String),
    NewJob(String),
    CronExpression(String),
    UpsertScheduledJob(String),
    RunScheduledJob(String),
    JobDefinitionNotFound,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewJobPayload {
    pub definition_id: String,
    pub job_name: Option<String>,
    pub task_definition: TaskDefinition,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSubTasksPayload {
    pub last_element_id: Option<String>,
    pub limit: i64,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewScheduledJobPayload {
    pub name: Option<String>,
    pub definition_id: String,
    pub task_definition: TaskDefinition,
    pub cron_expr: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SparqlQueryPayload {
    pub query: Option<String>,
    pub update: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadPayload {
    pub path: PathBuf,
}
pub static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let secret = std::env::var(JWT_SECRET).unwrap_or_else(|_| panic!("{JWT_SECRET} must be set"));
    Keys::new(secret.as_bytes())
});

pub static ROOT_OUTPUT_DIR_PB: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::var(ROOT_OUTPUT_DIR)
        .map(PathBuf::from)
        .unwrap_or_else(|_| "/share".into())
});
pub fn exp_from_now(service_account: bool) -> usize {
    if service_account {
        return DateTime::<Local>::MAX_UTC.timestamp() as usize;
    }
    std::env::var(JWT_EXPIRATION_TIME_SEC)
        .unwrap_or_else(|_| "30".into())
        .parse::<i64>()
        .map(|sec| (Local::now() + Duration::seconds(sec)).timestamp() as usize)
        .expect("could not create exp time")
}

pub struct Keys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

impl<S> OptionalFromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        match <Claims as FromRequestParts<S>>::from_request_parts(parts, state).await {
            Ok(res) => Ok(Some(res)),
            Err(_) => Ok(None),
        }
    }
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;
        // Decode the user data
        let token_data = decode::<Claims>(bearer.token(), &KEYS.decoding, &Validation::default())
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::FORBIDDEN, "Invalid token"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::AllJobs(e)
            | ApiError::AllScheduledJobs(e)
            | ApiError::AllTasks(e)
            | ApiError::DeleteJob(e)
            | ApiError::GetJob(e)
            | ApiError::DeleteScheduledJob(e)
            | ApiError::AllSubTasks(e)
            | ApiError::Download(e)
            | ApiError::GetLastPublications(e)
            | ApiError::SparqlError(e)
            | ApiError::SearchError(e)
            | ApiError::NewJob(e)
            | ApiError::UpsertScheduledJob(e)
            | ApiError::RunScheduledJob(e)
            | ApiError::CronExpression(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            ApiError::JobDefinitionNotFound => {
                (StatusCode::NOT_FOUND, "job definition not found".to_owned())
            }
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
