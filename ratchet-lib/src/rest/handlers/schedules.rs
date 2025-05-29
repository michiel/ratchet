use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, QuerySelect, ColumnTrait, QueryFilter, QueryOrder, PaginatorTrait};
use std::str::FromStr;

use crate::{
    database::{
        entities::{
            schedules::{self, Entity as Schedules},
            tasks::{Entity as Tasks},
        },
        repositories::RepositoryFactory,
    },
    rest::{
        middleware::error_handler::RestError,
        extractors::ListQueryExtractor,
        middleware::pagination::WithPaginationHeaders,
        models::{
            common::ApiResponse,
            schedules::{
                ScheduleCreateRequest, ScheduleDetailResponse, ScheduleFilters, ScheduleResponse,
                ScheduleUpdateRequest,
            },
        },
    },
};

#[derive(Clone)]
pub struct SchedulesContext {
    pub repository: RepositoryFactory,
}

pub async fn list_schedules(
    State(ctx): State<SchedulesContext>,
    ListQueryExtractor(query): ListQueryExtractor,
) -> Result<impl IntoResponse, RestError> {
    let db = ctx.repository.database().get_connection();
    
    let mut schedules_query = Schedules::find();

    // Apply filters
    let filter = query.filter();
    if !filter.filters.is_empty() {
        let filters_value = serde_json::to_value(&filter.filters).unwrap_or_default();
        if let Ok(schedule_filters) = serde_json::from_value::<ScheduleFilters>(filters_value) {
            if let Some(task_id) = schedule_filters.task_id {
                schedules_query = schedules_query.filter(schedules::Column::TaskId.eq(task_id));
            }
            if let Some(enabled) = schedule_filters.enabled {
                schedules_query = schedules_query.filter(schedules::Column::Enabled.eq(enabled));
            }
            if let Some(name_like) = schedule_filters.name_like {
                schedules_query = schedules_query.filter(schedules::Column::Name.contains(&name_like));
            }
        }
    }

    // Apply sorting
    let sort = query.sort();
    if let Some(sort_field) = sort.sort_field() {
        let column = match sort_field {
            "id" => schedules::Column::Id,
            "name" => schedules::Column::Name,
            "enabled" => schedules::Column::Enabled,
            "next_run_at" => schedules::Column::NextRunAt,
            "last_run_at" => schedules::Column::LastRunAt,
            "execution_count" => schedules::Column::ExecutionCount,
            "created_at" => schedules::Column::CreatedAt,
            "updated_at" => schedules::Column::UpdatedAt,
            _ => schedules::Column::Id,
        };

        if matches!(sort.sort_direction(), crate::rest::models::common::SortDirection::Desc) {
            schedules_query = schedules_query.order_by_desc(column);
        } else {
            schedules_query = schedules_query.order_by_asc(column);
        }
    }

    // Get total count
    let total = schedules_query
        .clone()
        .count(db)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;

    // Apply pagination
    let paginated_query = schedules_query
        .offset(query.pagination().offset())
        .limit(query.pagination().limit());

    let schedules = paginated_query
        .all(db)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;

    let schedule_responses: Vec<ScheduleResponse> = schedules.into_iter().map(Into::into).collect();

    Ok(Json(ApiResponse { data: schedule_responses })
        .with_pagination_headers(total, query.pagination().offset(), query.pagination().limit(), "schedules"))
}

pub async fn get_schedule(
    State(ctx): State<SchedulesContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, RestError> {
    let db = ctx.repository.database().get_connection();
    
    let schedule = Schedules::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Schedule not found".to_string()))?;

    // Get task name
    let task = Tasks::find_by_id(schedule.task_id)
        .one(db)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;

    // Calculate if schedule is exhausted
    let is_exhausted = if let Some(max_executions) = schedule.max_executions {
        schedule.execution_count >= max_executions
    } else {
        false
    };

    let runs_remaining = schedule.max_executions.map(|max| {
        let remaining = max - schedule.execution_count;
        if remaining > 0 { remaining } else { 0 }
    });

    let response = ScheduleDetailResponse {
        id: schedule.id,
        uuid: schedule.uuid,
        task_id: schedule.task_id,
        task_name: task.map(|t| t.name),
        name: schedule.name.clone(),
        cron_expression: schedule.cron_expression.clone(),
        input_data: schedule.input_data.clone(),
        enabled: schedule.enabled,
        next_run_at: schedule.next_run_at,
        last_run_at: schedule.last_run_at,
        execution_count: schedule.execution_count,
        max_executions: schedule.max_executions,
        metadata: schedule.metadata.clone(),
        created_at: schedule.created_at,
        updated_at: schedule.updated_at,
        is_exhausted,
        runs_remaining,
    };

    Ok(Json(response))
}

pub async fn create_schedule(
    State(ctx): State<SchedulesContext>,
    Json(payload): Json<ScheduleCreateRequest>,
) -> Result<impl IntoResponse, RestError> {
    let schedule_repo = ctx.repository.schedule_repository();
    let db = ctx.repository.database().get_connection();

    // Verify task exists
    let _task = Tasks::find_by_id(payload.task_id)
        .one(db)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::BadRequest("Task not found".to_string()))?;

    // Validate cron expression
    if cron::Schedule::from_str(&payload.cron_expression).is_err() {
        return Err(RestError::BadRequest("Invalid cron expression".to_string()));
    }

    // Create schedule
    let new_schedule = schedules::Model::new(
        payload.task_id,
        payload.name,
        payload.cron_expression,
        payload.input_data.clone(),
    );

    let mut active_schedule: schedules::ActiveModel = new_schedule.into();

    // Set optional fields
    if let Some(enabled) = payload.enabled {
        active_schedule.enabled = ActiveValue::Set(enabled);
    }
    if let Some(max_executions) = payload.max_executions {
        active_schedule.max_executions = ActiveValue::Set(Some(max_executions));
    }
    if let Some(metadata) = payload.metadata {
        active_schedule.metadata = ActiveValue::Set(Some(sea_orm::prelude::Json::from(metadata)));
    }

    let created_schedule = active_schedule
        .insert(db)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;

    // Calculate next run time
    let next_run = created_schedule.calculate_next_run()
        .map_err(|e| RestError::BadRequest(e))?
        .ok_or_else(|| RestError::BadRequest("Schedule is exhausted or disabled".to_string()))?;
    
    // Update next run time
    schedule_repo
        .update_next_run(created_schedule.id, Some(next_run))
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;
    
    // Get updated schedule
    let updated_schedule = schedule_repo
        .find_by_id(created_schedule.id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Schedule not found".to_string()))?;

    Ok((StatusCode::CREATED, Json(ScheduleResponse::from(updated_schedule))))
}

pub async fn update_schedule(
    State(ctx): State<SchedulesContext>,
    Path(id): Path<i32>,
    Json(payload): Json<ScheduleUpdateRequest>,
) -> Result<impl IntoResponse, RestError> {
    let schedule_repo = ctx.repository.schedule_repository();
    let db = ctx.repository.database().get_connection();
    
    let schedule = Schedules::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Schedule not found".to_string()))?;

    let mut active_schedule: schedules::ActiveModel = schedule.into();
    let mut cron_changed = false;

    if let Some(name) = payload.name {
        active_schedule.name = ActiveValue::Set(name);
    }
    if let Some(cron_expression) = payload.cron_expression {
        // Validate cron expression
        if cron::Schedule::from_str(&cron_expression).is_err() {
            return Err(RestError::BadRequest("Invalid cron expression".to_string()));
        }
        active_schedule.cron_expression = ActiveValue::Set(cron_expression);
        cron_changed = true;
    }
    if let Some(input_data) = payload.input_data {
        active_schedule.input_data = ActiveValue::Set(sea_orm::prelude::Json::from(input_data));
    }
    if let Some(enabled) = payload.enabled {
        active_schedule.enabled = ActiveValue::Set(enabled);
    }
    if let Some(max_executions) = payload.max_executions {
        active_schedule.max_executions = ActiveValue::Set(Some(max_executions));
    }
    if let Some(metadata) = payload.metadata {
        active_schedule.metadata = ActiveValue::Set(Some(sea_orm::prelude::Json::from(metadata)));
    }

    active_schedule.updated_at = ActiveValue::Set(chrono::Utc::now());

    let updated_schedule = active_schedule
        .update(db)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;

    // If cron expression changed, recalculate next run time
    let final_schedule = if cron_changed {
        let next_run = updated_schedule.calculate_next_run()
            .map_err(|e| RestError::BadRequest(e))?;
        
        schedule_repo
            .update_next_run(id, next_run)
            .await
            .map_err(|e| RestError::DatabaseError(e.to_string()))?;
        
        schedule_repo
            .find_by_id(id)
            .await
            .map_err(|e| RestError::DatabaseError(e.to_string()))?
            .ok_or_else(|| RestError::NotFound("Schedule not found".to_string()))?
    } else {
        updated_schedule
    };

    Ok(Json(ScheduleResponse::from(final_schedule)))
}

pub async fn delete_schedule(
    State(ctx): State<SchedulesContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, RestError> {
    let schedule_repo = ctx.repository.schedule_repository();
    let db = ctx.repository.database().get_connection();
    
    let _schedule = Schedules::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Schedule not found".to_string()))?;

    schedule_repo
        .delete(id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn enable_schedule(
    State(ctx): State<SchedulesContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, RestError> {
    let schedule_repo = ctx.repository.schedule_repository();
    
    schedule_repo
        .set_enabled(id, true)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;

    let updated_schedule = schedule_repo
        .find_by_id(id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Schedule not found".to_string()))?;

    Ok(Json(ScheduleResponse::from(updated_schedule)))
}

pub async fn disable_schedule(
    State(ctx): State<SchedulesContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, RestError> {
    let schedule_repo = ctx.repository.schedule_repository();
    
    schedule_repo
        .set_enabled(id, false)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;

    let updated_schedule = schedule_repo
        .find_by_id(id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Schedule not found".to_string()))?;

    Ok(Json(ScheduleResponse::from(updated_schedule)))
}

pub async fn trigger_schedule(
    State(ctx): State<SchedulesContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, RestError> {
    let schedule_repo = ctx.repository.schedule_repository();
    let job_repo = ctx.repository.job_repository();
    let db = ctx.repository.database().get_connection();
    
    let schedule = Schedules::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound("Schedule not found".to_string()))?;

    if !schedule.enabled {
        return Err(RestError::BadRequest("Schedule is disabled".to_string()));
    }

    // Check if exhausted
    if let Some(max_executions) = schedule.max_executions {
        if schedule.execution_count >= max_executions {
            return Err(RestError::BadRequest("Schedule has reached maximum executions".to_string()));
        }
    }

    // Create a job for this schedule
    // Convert sea_orm::Json to serde_json::Value
    let input_data: serde_json::Value = serde_json::to_value(&schedule.input_data)
        .and_then(|v| serde_json::from_value(v))
        .unwrap_or(serde_json::Value::Null);
    
    let new_job = crate::database::entities::jobs::Model::new_scheduled(
        schedule.task_id,
        schedule.id,
        input_data,
        chrono::Utc::now(),
    );

    let created_job = job_repo
        .create(new_job)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;

    // Update schedule execution info
    schedule_repo
        .record_execution(id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "message": "Schedule triggered successfully",
        "job_id": created_job.id,
        "job_uuid": created_job.uuid
    }))))
}