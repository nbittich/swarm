use std::{net::SocketAddr, str::FromStr};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{
        HeaderMap, StatusCode,
        header::{self, ACCEPT, CONTENT_TYPE},
    },
    response::{AppendHeaders, IntoResponse},
    routing::{delete, get, post},
};
use jsonwebtoken::Header;
use mime_guess::mime::APPLICATION_OCTET_STREAM;
use serde_json::{Value, json};
use sparql_client::{Head, SparqlClient, SparqlResponse, SparqlResult};
use swarm_common::{
    TryFutureExt,
    constant::TASK_STATUS_CHANGE_EVENT,
    debug,
    domain::{
        AuthBody, AuthPayload, Job, JobDefinition, ScheduledJob, Status, SubTask, Task, User,
        index_config::{
            IndexConfiguration, IndexStatistics, SearchQueryRequest, SearchQueryResponse,
        },
    },
    info,
    mongo::{FindOptions, Page, Pageable, Repository, doc},
    retry_fs,
};
use swarm_meilisearch_client::domain::{BatchResponse, BatchStatus};
use tokio_util::io::ReaderStream;

use crate::{
    domain::{
        ApiBatchResponse, ApiError, AuthError, Claims, DownloadPayload, GetSubTasksPayload, KEYS,
        MeilisearchGetBatchPayload, NewJobPayload, NewScheduledJobPayload, SparqlQueryPayload,
        exp_from_now,
    },
    manager::JobManagerState,
};

use bcrypt::verify;

pub async fn serve(
    host: &str,
    port: &str,
    body_size_limit: usize,
    manager_state: JobManagerState,
) -> anyhow::Result<()> {
    let addr = SocketAddr::from_str(&format!("{host}:{port}")).unwrap();
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let app = Router::new()
        .route("/sparql", get(sparql).post(sparql))
        .route("/search/batches", post(search_batches))
        .route("/search/{index}", post(search))
        .route("/search/{index}/stats", get(index_stats))
        .route("/search-configuration", get(search_configuration))
        .route("/login", post(authorize))
        .route("/scheduled-jobs/new", post(new_scheduled_job))
        .route("/scheduled-jobs/status", post(job_scheduler_status))
        .route("/scheduled-jobs/start", post(start_job_scheduler))
        .route("/scheduled-jobs/pause", post(pause_job_scheduler))
        .route("/scheduled-jobs/{id}/update", post(update_scheduled_job))
        .route("/scheduled-jobs/{id}/run", post(run_scheduled_job))
        .route("/scheduled-jobs/{id}", delete(delete_scheduled_job))
        .route("/scheduled-jobs", post(all_scheduled_jobs))
        .route("/jobs/{job_id}", get(get_job))
        .route("/jobs/{job_id}", delete(delete_job))
        .route("/jobs/{job_id}/download", get(download))
        .route("/jobs/{job_id}/tasks/{task_id}", post(restart_task))
        .route("/jobs/{job_id}/tasks/{task_id}/subtasks", get(all_subtasks))
        .route("/jobs/{job_id}/tasks", get(all_tasks))
        .route("/jobs/new", post(new_job))
        .route("/jobs", post(all_jobs))
        .route("/job-definitions", get(all_job_definitions))
        .route("/publications", post(get_last_publications))
        .layer(DefaultBodyLimit::max(body_size_limit))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(manager_state)
        .fallback(fallback);
    info!("listening on {:?}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}
async fn authorize(
    State(manager): State<JobManagerState>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthBody>, AuthError> {
    // Check if the user sent the credentials
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    let User {
        username,
        password,
        first_name,
        last_name,
        service_account,
        email,
        ..
    } = match manager
        .user_repository
        .find_one(Some(doc! {"username": &payload.username}))
        .await
    {
        Ok(None) => {
            debug!("not found in mongo but ok");
            return Err(AuthError::WrongCredentials);
        }
        Ok(Some(user)) => user,
        Err(e) => {
            debug!("{e}");
            return Err(AuthError::WrongCredentials);
        }
    };

    let Ok(true) = verify(&payload.password, &password) else {
        debug!("invalid hash!");
        return Err(AuthError::WrongCredentials);
    };

    let claims = Claims {
        sub: username,
        email,
        first_name,
        last_name,
        exp: exp_from_now(service_account),
    };
    // Create the authorization token
    let token = jsonwebtoken::encode(&Header::default(), &claims, &KEYS.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    // Send the authorized token
    Ok(Json(AuthBody::new(token)))
}

async fn all_jobs(
    State(manager): State<JobManagerState>,
    _: Option<Claims>,
    Json(pageable): Json<Pageable>,
) -> Result<Json<Page<Job>>, ApiError> {
    let jobs = manager
        .job_repository
        .find_page(pageable)
        .await
        .map_err(|e| ApiError::AllJobs(e.to_string()))?;
    Ok(Json(jobs))
}

async fn all_scheduled_jobs(
    State(manager): State<JobManagerState>,
    _: Option<Claims>,
    Json(pageable): Json<Pageable>,
) -> Result<Json<Page<ScheduledJob>>, ApiError> {
    let scheduled_jobs = manager
        .scheduled_job_repository
        .find_page(pageable)
        .await
        .map_err(|e| ApiError::AllScheduledJobs(e.to_string()))?;
    Ok(Json(scheduled_jobs))
}

async fn all_job_definitions(
    State(manager): State<JobManagerState>,
    _: Option<Claims>,
) -> Result<Json<Vec<JobDefinition>>, ApiError> {
    Ok(Json(manager.job_definitions.iter().cloned().collect()))
}

// #[axum::debug_handler]
async fn sparql(
    State(manager): State<JobManagerState>,
    claims: Option<Claims>,
    headers: HeaderMap,
    axum::extract::Form(SparqlQueryPayload { query, update }): axum::extract::Form<
        SparqlQueryPayload,
    >,
) -> impl IntoResponse {
    let query = query
        .or(update)
        .ok_or_else(|| ApiError::SparqlError("missing query param".to_string()))?;
    let is_update = spargebra::Update::parse(&query, None).is_ok();
    if claims.is_none() && is_update {
        return Err(ApiError::SparqlError("illegal access".into()));
    }
    if is_update {
        // i was lazy
        SparqlClient::_update(
            &manager.sparql_client.client,
            &manager.sparql_client.endpoint,
            query.as_str(),
        )
        .await
        .map_err(|e| ApiError::SparqlError(e.to_string()))?;
        Ok(Json(SparqlResponse {
            head: Head {
                link: None,
                vars: vec![],
            },
            results: SparqlResult {
                distinct: Some(true),
                bindings: vec![],
            },
        })
        .into_response())
    } else {
        spargebra::Query::parse(&query, None).map_err(|e| ApiError::SparqlError(e.to_string()))?;
        let accept_header = headers
            .get(ACCEPT)
            .and_then(|h| h.to_str().ok())
            .map(String::from);
        let (content_type, res) = manager
            .sparql_client
            .query_with_accept_header(query, accept_header)
            .await
            .map_err(|e| ApiError::SparqlError(e.to_string()))?;
        headers.contains_key(ACCEPT);
        let content_type = (CONTENT_TYPE, content_type);

        let headers = AppendHeaders([content_type]);

        Ok((headers, res).into_response())
    }
}
async fn new_job(
    State(manager): State<JobManagerState>,
    _: Claims,
    Json(NewJobPayload {
        definition_id,
        job_name,
        task_definition,
    }): Json<NewJobPayload>,
) -> Result<Json<Job>, ApiError> {
    let job = manager
        .new_job(definition_id, job_name, task_definition)
        .await?;
    Ok(Json(job))
}
async fn pause_job_scheduler(State(manager): State<JobManagerState>, _: Claims) {
    manager.toggle_pause_scheduler(true);
}
async fn start_job_scheduler(State(manager): State<JobManagerState>, _: Claims) {
    manager.toggle_pause_scheduler(false);
}
async fn job_scheduler_status(
    State(manager): State<JobManagerState>,
    _: Option<Claims>,
) -> Json<Value> {
    match manager
        .pause_scheduler
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        true => Json(json!({"status": "paused"})),
        false => Json(json!({"status": "running"})),
    }
}
async fn new_scheduled_job(
    State(manager): State<JobManagerState>,
    _: Claims,
    Json(NewScheduledJobPayload {
        name,
        definition_id,
        task_definition,
        cron_expr,
    }): Json<NewScheduledJobPayload>,
) -> Result<Json<ScheduledJob>, ApiError> {
    let sj = manager
        .upsert_scheduled_job(None, name, definition_id, task_definition, cron_expr)
        .await?;
    Ok(Json(sj))
}
async fn update_scheduled_job(
    State(manager): State<JobManagerState>,
    _: Claims,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(NewScheduledJobPayload {
        name,
        definition_id,
        task_definition,
        cron_expr,
    }): Json<NewScheduledJobPayload>,
) -> Result<Json<ScheduledJob>, ApiError> {
    let sj = manager
        .upsert_scheduled_job(Some(id), name, definition_id, task_definition, cron_expr)
        .await?;
    Ok(Json(sj))
}
async fn run_scheduled_job(
    State(manager): State<JobManagerState>,
    _: Claims,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Value>, ApiError> {
    manager.run_scheduled_job_manually(id).await?;
    Ok(Json(json!({"message": "ok usa"})))
}

async fn get_job(
    State(manager): State<JobManagerState>,
    _: Option<Claims>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<Json<Job>, ApiError> {
    manager
        .get_job(&job_id)
        .await
        .map(Json)
        .map_err(|e| ApiError::GetJob(e.to_string()))
}

async fn delete_job(
    State(manager): State<JobManagerState>,
    _: Claims,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    manager
        .delete_job(&job_id)
        .await
        .map_err(|e| ApiError::DeleteJob(e.to_string()))?;
    Ok(())
}
async fn delete_scheduled_job(
    State(manager): State<JobManagerState>,
    _: Claims,
    axum::extract::Path(scheduled_job_id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    manager
        .scheduled_job_repository
        .delete_by_id(&scheduled_job_id)
        .await
        .map_err(|e| ApiError::DeleteScheduledJob(e.to_string()))?;
    Ok(())
}
async fn download(
    State(manager): State<JobManagerState>,
    _: Claims,
    axum::extract::Path(job_id): axum::extract::Path<String>,
    axum::extract::Query(DownloadPayload { path }): axum::extract::Query<DownloadPayload>,
) -> Result<impl IntoResponse, ApiError> {
    let job = manager
        .job_repository
        .find_by_id(&job_id)
        .await
        .map_err(|e| ApiError::Download(e.to_string()))?;
    if job
        .filter(|j| path.starts_with(&j.root_dir) && path.is_file())
        .is_none()
    {
        return Err(ApiError::Download("Unauthorized".to_string()));
    }
    let f = retry_fs::open_file(&path)
        .map_err(|e| ApiError::Download(e.to_string()))
        .await?;
    let stream = ReaderStream::new(f);
    let body = axum::body::Body::from_stream(stream);
    let content_disposition = (
        header::CONTENT_DISPOSITION,
        format!(
            r#"attachment; filename="{}""#,
            &path.file_name().and_then(|f| f.to_str()).unwrap_or("file")
        ),
    );
    let ct = mime_guess::from_path(&path)
        .first_raw()
        .map(|c| c.to_string())
        .unwrap_or_else(|| APPLICATION_OCTET_STREAM.to_string());
    let content_type = (CONTENT_TYPE, ct);

    let headers = AppendHeaders([content_type, content_disposition]);
    Ok((headers, body))
}
async fn all_subtasks(
    State(manager): State<JobManagerState>,
    _: Option<Claims>,
    axum::extract::Path((job_id, task_id)): axum::extract::Path<(String, String)>,
    axum::extract::Query(GetSubTasksPayload {
        last_element_id,
        limit,
    }): axum::extract::Query<GetSubTasksPayload>,
) -> Result<Json<Vec<SubTask>>, ApiError> {
    let task = manager
        .task_repository
        .find_one(Some(doc! {
            "jobId": &job_id,
            "_id": &task_id,
        }))
        .await
        .map_err(|e| ApiError::AllSubTasks(e.to_string()))?;
    debug!("jobId {job_id}, taskId {task_id}, task {task:?}");
    match task {
        Some(task) => {
            let st = manager
                .sub_task_repository
                .find_page_large_collection(
                    Some(doc! {
                        "taskId": task.id,
                    }),
                    last_element_id,
                    limit,
                )
                .await
                .map_err(|e| ApiError::AllSubTasks(e.to_string()))?;
            Ok(Json(st))
        }
        None => Ok(Json(vec![])),
    }
}
async fn all_tasks(
    State(manager): State<JobManagerState>,
    _: Option<Claims>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<Json<Vec<Task>>, ApiError> {
    let tasks = manager
        .task_repository
        .find_by_query(
            doc! {"jobId": job_id},
            Some(
                FindOptions::builder()
                    .sort(Some(doc! { "creationDate": -1 }))
                    .build(),
            ),
        )
        .await
        .map_err(|e| ApiError::AllTasks(e.to_string()))?;
    Ok(Json(tasks))
}
async fn restart_task(
    State(manager): State<JobManagerState>,
    _: Option<Claims>,
    axum::extract::Path((job_id, task_id)): axum::extract::Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let task = manager
        .task_repository
        .find_one(Some(doc! {
            "jobId": &job_id,
            "_id": &task_id,
        }))
        .await
        .map_err(|e| ApiError::RestartTask(e.to_string()))?;
    debug!("jobId {job_id}, taskId {task_id}, task {task:?}");
    match task {
        Some(mut task) if matches!(task.status, Status::Failed(_)) => {
            let message = format!("task with id {} will be restarted", task.id);

            task.status = Status::Scheduled;
            debug!("update status...");
            manager
                .task_repository
                .upsert(&task.id, &task)
                .await
                .map_err(|e| ApiError::RestartTask(e.to_string()))?;
            manager
                .job_repository
                .update_many(
                    doc! {
                        "_id": &task.job_id
                    },
                    doc! {
                        "$set": {
                            "status": {"type": "busy"}
                        }
                    },
                )
                .await
                .map_err(|e| ApiError::RestartTask(e.to_string()))?;

            tokio::spawn(async move {
                debug!("delete all sub tasks...");
                manager
                    .sub_task_repository
                    .delete_many(Some(doc! {
                        "taskId": &task.id
                    }))
                    .await?;
                debug!("delete all sub tasks done");
                if task.output_dir.exists() {
                    debug!("recreate output dir for task...");
                    retry_fs::remove_dir_all(&task.output_dir).await?;
                    retry_fs::create_dir_all(&task.output_dir).await?;
                    debug!("recreate output dir for task done");
                }
                debug!("notify tasks handlers...");
                manager
                    .nc
                    .publish(TASK_STATUS_CHANGE_EVENT(&task.id), &task)
                    .await?;
                Ok(()) as anyhow::Result<()>
            });

            Ok(Json(json! ({"message": message})))
        }
        e => {
            let m = format!("cannot restart the task: {e:?}");

            Ok(Json(json! ({"message": m})))
        }
    }
}
async fn get_last_publications(
    State(manager): State<JobManagerState>,
    _: Claims,
) -> Result<Json<Vec<Task>>, ApiError> {
    let job_filter = doc! {
        "targetUrl" :{ "$ne" : null },
        "status.type": "success"
    };
    // could be a projection tbh
    let jobs = manager
        .job_repository
        .find_by_query(job_filter, None)
        .await
        .map_err(|e| ApiError::GetLastPublications(e.to_string()))?;
    let job_ids = jobs.into_iter().map(|j| j.id).collect::<Vec<_>>();
    let tasks = manager
        .task_repository
        .find_by_query(
            doc! {
                "status.type": "success",
                "result.type": "publish",
                "jobId": {
                    "$in": job_ids
                }
            },
            None,
        )
        .await
        .map_err(|e| ApiError::GetLastPublications(e.to_string()))?;
    Ok(Json(tasks))
}

async fn search_configuration(
    State(manager): State<JobManagerState>,
) -> Result<Json<Vec<IndexConfiguration>>, ApiError> {
    Ok(Json(manager.index_config.iter().cloned().collect()))
}

async fn search(
    State(manager): State<JobManagerState>,
    axum::extract::Path(index): axum::extract::Path<String>,
    Json(mut req): Json<SearchQueryRequest>,
) -> Result<Json<SearchQueryResponse>, ApiError> {
    manager
        .search(&index, &mut req)
        .await
        .map(Json)
        .map_err(|e| ApiError::SearchError(e.to_string()))
}
async fn index_stats(
    State(manager): State<JobManagerState>,
    axum::extract::Path(index): axum::extract::Path<String>,
) -> Result<Json<IndexStatistics>, ApiError> {
    manager
        .search_client
        .get_stats(index.as_str())
        .await
        .map(|st| {
            Json(IndexStatistics {
                number_of_documents: st.number_of_documents,
            })
        })
        .map_err(|e| ApiError::SearchError(e.to_string()))
}

async fn search_batches(
    State(manager): State<JobManagerState>,
    _: Claims,
    Json(MeilisearchGetBatchPayload {
        statuses,
        next,
        current: prev,
    }): Json<MeilisearchGetBatchPayload>,
) -> Result<Json<ApiBatchResponse>, ApiError> {
    let BatchResponse {
        results,
        from: current,
        next,
        ..
    } = manager
        .search_client
        .get_batches_paginated_response(
            &statuses.unwrap_or_else(|| {
                vec![
                    BatchStatus::Enqueued,
                    BatchStatus::Failed,
                    BatchStatus::Succeeded,
                    BatchStatus::Processing,
                    BatchStatus::Canceled,
                ]
            }),
            &next,
        )
        .await
        .map_err(|e| ApiError::SearchError(e.to_string()))?;
    Ok(Json(ApiBatchResponse {
        batches: results,
        current,
        next,
        prev,
    }))
}
