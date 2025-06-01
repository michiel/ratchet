use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, QuerySelect, ColumnTrait, QueryFilter, QueryOrder, PaginatorTrait};

use crate::{
    database::{
        entities::{
            jobs::{self, Entity as Jobs, JobStatus, JobPriority as Priority},
            tasks::{Entity as Tasks},
        },
        repositories::RepositoryFactory,
    },
    output::OutputDeliveryManager,
    rest::{
        middleware::error_handler::RestError,
        extractors::ListQueryExtractor,
        middleware::pagination::WithPaginationHeaders,
        models::{
            common::ApiResponse,
            jobs::{
                JobCreateRequest, JobDetailResponse, JobFilters, JobQueueStats, JobResponse,
                JobUpdateRequest, PriorityStats, TestOutputDestinationsRequest,
                TestOutputDestinationsResponse, TestDestinationResult,
            },
        },
    },
};

#[derive(Clone)]
pub struct JobsContext {
    pub repository: RepositoryFactory,
}

pub async fn list_jobs(
    State(ctx): State<JobsContext>,
    ListQueryExtractor(query): ListQueryExtractor,
) -> Result<impl IntoResponse, RestError> {
    let db = ctx.repository.database().get_connection();
    
    let mut jobs_query = Jobs::find();

    // Apply filters
    if !query.filters.is_empty() {
        let filters_value = serde_json::to_value(&query.filters).unwrap_or_default();
        if let Ok(job_filters) = serde_json::from_value::<JobFilters>(filters_value) {
            if let Some(status) = job_filters.status {
                if let Ok(status) = status.parse::<JobStatus>() {
                    jobs_query = jobs_query.filter(jobs::Column::Status.eq(status));
                }
            }
            if let Some(priority) = job_filters.priority {
                if let Ok(priority) = priority.parse::<Priority>() {
                    jobs_query = jobs_query.filter(jobs::Column::Priority.eq(priority));
                }
            }
            if let Some(task_id) = job_filters.task_id {
                jobs_query = jobs_query.filter(jobs::Column::TaskId.eq(task_id));
            }
            if let Some(schedule_id) = job_filters.schedule_id {
                jobs_query = jobs_query.filter(jobs::Column::ScheduleId.eq(schedule_id));
            }
        }
    }

    // Apply sorting
    let sort_query = query.sort();
    if let Some(sort_field) = sort_query.sort_field() {
        let column = match sort_field {
            "id" => jobs::Column::Id,
            "priority" => jobs::Column::Priority,
            "status" => jobs::Column::Status,
            "queued_at" => jobs::Column::QueuedAt,
            "process_at" => jobs::Column::ProcessAt,
            "started_at" => jobs::Column::StartedAt,
            "completed_at" => jobs::Column::CompletedAt,
            _ => jobs::Column::Id,
        };

        if matches!(sort_query.sort_direction(), crate::rest::models::common::SortDirection::Desc) {
            jobs_query = jobs_query.order_by_desc(column);
        } else {
            jobs_query = jobs_query.order_by_asc(column);
        }
    }

    // Get total count
    let total = jobs_query
        .clone()
        .count(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?;

    // Apply pagination
    let pagination = query.pagination();
    let paginated_query = jobs_query
        .offset(pagination.offset())
        .limit(pagination.limit());

    let jobs = paginated_query
        .all(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?;

    let job_responses: Vec<JobResponse> = jobs.into_iter().map(Into::into).collect();

    Ok(Json(ApiResponse { data: job_responses })
        .with_pagination_headers(total, pagination.offset(), pagination.limit(), "jobs"))
}

pub async fn get_job(
    State(ctx): State<JobsContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, RestError> {
    let db = ctx.repository.database().get_connection();
    
    let job = Jobs::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Job not found".to_string()))?;

    // Get task name
    let task = Tasks::find_by_id(job.task_id)
        .one(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?;

    // Calculate queue position if job is queued
    let queue_position = if job.status == JobStatus::Queued {
        let position = Jobs::find()
            .filter(jobs::Column::Status.eq(JobStatus::Queued))
            .filter(jobs::Column::Priority.gte(job.priority.clone()))
            .filter(jobs::Column::QueuedAt.lt(job.queued_at))
            .count(db)
            .await
            .map_err(|e| RestError::InternalError(e.to_string()))?;
        Some((position + 1) as i32)
    } else {
        None
    };

    let output_destinations = job.output_destinations
        .and_then(|json| serde_json::from_value(json.into()).ok());
    
    let response = JobDetailResponse {
        id: job.id,
        uuid: job.uuid,
        task_id: job.task_id,
        task_name: task.map(|t| t.name),
        execution_id: job.execution_id,
        schedule_id: job.schedule_id,
        priority: job.priority,
        status: job.status,
        input_data: job.input_data.clone(),
        retry_count: job.retry_count,
        max_retries: job.max_retries,
        retry_delay_seconds: job.retry_delay_seconds,
        error_message: job.error_message,
        error_details: job.error_details.clone(),
        queued_at: job.queued_at,
        process_at: job.process_at,
        started_at: job.started_at,
        completed_at: job.completed_at,
        metadata: job.metadata.clone(),
        output_destinations,
        queue_position,
    };

    Ok(Json(response))
}

pub async fn create_job(
    State(ctx): State<JobsContext>,
    Json(payload): Json<JobCreateRequest>,
) -> Result<impl IntoResponse, RestError> {
    let _job_repo = ctx.repository.job_repository();

    // Verify task exists
    let db = ctx.repository.database().get_connection();
    let _task = Tasks::find_by_id(payload.task_id)
        .one(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?
        .ok_or_else(|| RestError::BadRequest("Task not found".to_string()))?;

    // Create job
    let priority = payload.priority.unwrap_or(Priority::Normal);
    let new_job = jobs::Model::new(payload.task_id, payload.input_data.clone(), priority);

    let mut active_job: jobs::ActiveModel = new_job.into();
    // Unset the ID to let the database auto-generate it
    active_job.id = ActiveValue::NotSet;

    // Set optional fields
    if let Some(process_at) = payload.process_at {
        active_job.process_at = ActiveValue::Set(Some(process_at));
    }
    if let Some(max_retries) = payload.max_retries {
        active_job.max_retries = ActiveValue::Set(max_retries);
    }
    if let Some(retry_delay) = payload.retry_delay_seconds {
        active_job.retry_delay_seconds = ActiveValue::Set(retry_delay);
    }
    if let Some(metadata) = payload.metadata {
        active_job.metadata = ActiveValue::Set(Some(sea_orm::prelude::Json::from(metadata)));
    }
    if let Some(destinations) = payload.output_destinations {
        active_job.output_destinations = ActiveValue::Set(Some(sea_orm::prelude::Json::from(serde_json::to_value(destinations).unwrap())));
    }

    let created_job = active_job
        .insert(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(JobResponse::from(created_job))))
}

pub async fn update_job(
    State(ctx): State<JobsContext>,
    Path(id): Path<i32>,
    Json(payload): Json<JobUpdateRequest>,
) -> Result<impl IntoResponse, RestError> {
    let db = ctx.repository.database().get_connection();
    
    let job = Jobs::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Job not found".to_string()))?;

    // Only allow updates to queued jobs
    if job.status != JobStatus::Queued {
        return Err(RestError::BadRequest(
            "Can only update jobs that are queued".to_string(),
        ));
    }

    let mut active_job: jobs::ActiveModel = job.into();

    if let Some(priority) = payload.priority {
        active_job.priority = ActiveValue::Set(priority);
    }
    if let Some(process_at) = payload.process_at {
        active_job.process_at = ActiveValue::Set(Some(process_at));
    }
    if let Some(max_retries) = payload.max_retries {
        active_job.max_retries = ActiveValue::Set(max_retries);
    }
    if let Some(retry_delay) = payload.retry_delay_seconds {
        active_job.retry_delay_seconds = ActiveValue::Set(retry_delay);
    }

    let updated_job = active_job
        .update(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?;

    Ok(Json(JobResponse::from(updated_job)))
}

pub async fn delete_job(
    State(ctx): State<JobsContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, RestError> {
    let db = ctx.repository.database().get_connection();
    let job_repo = ctx.repository.job_repository();
    
    let job = Jobs::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Job not found".to_string()))?;

    // Only allow deletion of queued or completed/failed/cancelled jobs
    if job.status == JobStatus::Processing || job.status == JobStatus::Retrying {
        return Err(RestError::BadRequest(
            "Cannot delete jobs that are currently processing".to_string(),
        ));
    }

    job_repo
        .delete(id)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancel_job(
    State(ctx): State<JobsContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, RestError> {
    let db = ctx.repository.database().get_connection();
    let job_repo = ctx.repository.job_repository();
    
    let job = Jobs::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Job not found".to_string()))?;

    // Only allow cancellation of queued or retrying jobs
    if job.status != JobStatus::Queued && job.status != JobStatus::Retrying {
        return Err(RestError::BadRequest(
            "Can only cancel jobs that are queued or retrying".to_string(),
        ));
    }

    job_repo
        .update_status(id, JobStatus::Cancelled)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?;

    let updated_job = Jobs::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Job not found".to_string()))?;

    Ok(Json(JobResponse::from(updated_job)))
}

pub async fn retry_job(
    State(ctx): State<JobsContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, RestError> {
    let db = ctx.repository.database().get_connection();
    
    let job = Jobs::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Job not found".to_string()))?;

    // Only allow retry of failed or cancelled jobs
    if job.status != JobStatus::Failed && job.status != JobStatus::Cancelled {
        return Err(RestError::BadRequest(
            "Can only retry failed or cancelled jobs".to_string(),
        ));
    }

    let mut active_job: jobs::ActiveModel = job.into();
    active_job.status = ActiveValue::Set(JobStatus::Queued);
    active_job.retry_count = ActiveValue::Set(0);
    active_job.error_message = ActiveValue::Set(None);
    active_job.error_details = ActiveValue::Set(None);
    active_job.started_at = ActiveValue::Set(None);
    active_job.completed_at = ActiveValue::Set(None);

    let updated_job = active_job
        .update(db)
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?;

    Ok(Json(JobResponse::from(updated_job)))
}

pub async fn get_queue_stats(
    State(ctx): State<JobsContext>,
) -> Result<impl IntoResponse, RestError> {
    let job_repo = ctx.repository.job_repository();
    
    let stats = job_repo
        .get_queue_stats()
        .await
        .map_err(|e| RestError::InternalError(e.to_string()))?;

    let response = JobQueueStats {
        total: stats.total,
        queued: stats.queued,
        processing: stats.processing,
        completed: stats.completed,
        failed: stats.failed,
        cancelled: 0, // TODO: Add cancelled count to repository stats
        retrying: stats.retrying,
        by_priority: PriorityStats {
            urgent: 0, // TODO: Add priority breakdown to repository stats
            high: 0,
            normal: 0,
            low: 0,
        },
    };

    Ok(Json(response))
}

pub async fn test_output_destinations(
    Json(payload): Json<TestOutputDestinationsRequest>,
) -> Result<impl IntoResponse, RestError> {
    // Test each destination configuration
    let test_results = OutputDeliveryManager::test_configurations(&payload.destinations)
        .await
        .map_err(|e| RestError::BadRequest(format!("Invalid destination configurations: {}", e)))?;
    
    let mut results = Vec::new();
    let mut overall_success = true;
    
    for test_result in test_results {
        let success = test_result.success;
        if !success {
            overall_success = false;
        }
        
        results.push(TestDestinationResult {
            index: test_result.index,
            destination_type: test_result.destination_type,
            success,
            error: test_result.error,
            estimated_time_ms: test_result.estimated_time.as_millis() as u64,
        });
    }
    
    let response = TestOutputDestinationsResponse {
        results,
        overall_success,
    };
    
    Ok(Json(response))
}