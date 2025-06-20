//! GraphQL mutation resolvers

use async_graphql::{Object, Context, Result};
use crate::{
    context::GraphQLContext,
    types::*,
};
use ratchet_api_types::ApiError;
use ratchet_core::validation::{InputValidator, ErrorSanitizer};
use serde_json::Value as JsonValue;
use tracing::warn;

/// Root mutation resolver
pub struct Mutation;

#[Object]
impl Mutation {
    /// Create a new task
    async fn create_task(
        &self,
        ctx: &Context<'_>,
        input: CreateTaskInput,
    ) -> Result<Task> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Validate input
        let validator = InputValidator::new();
        let sanitizer = ErrorSanitizer::default();
        
        // Validate task name
        if let Err(validation_err) = validator.validate_task_name(&input.name) {
            warn!("Invalid task name in GraphQL create_task: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(ApiError::bad_request(&sanitized_error.message).into());
        }
        
        // Validate description if provided
        if let Some(ref description) = input.description {
            if let Err(validation_err) = validator.validate_string(description, "description") {
                warn!("Invalid description in GraphQL create_task: {}", validation_err);
                let sanitized_error = sanitizer.sanitize_error(&validation_err);
                return Err(ApiError::bad_request(&sanitized_error.message).into());
            }
        }
        
        // Create UnifiedTask from input
        let unified_task = ratchet_api_types::UnifiedTask {
            id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
            uuid: uuid::Uuid::new_v4(),
            name: input.name,
            description: input.description,
            version: "1.0.0".to_string(), // Default version
            enabled: input.enabled.unwrap_or(true),
            registry_source: false, // Tasks created via API are not from registry
            available_versions: vec!["1.0.0".to_string()],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            validated_at: None,
            in_sync: true,
            input_schema: input.input_schema,
            output_schema: input.output_schema,
            metadata: input.metadata,
        };
        
        // Create the task using the repository
        let task_repo = context.repositories.task_repository();
        let created_task = task_repo.create(unified_task).await
            .map_err(|e| ApiError::internal_error(format!("Failed to create task: {}", e)))?;
        
        Ok(created_task)
    }

    /// Update an existing task
    async fn update_task(
        &self,
        ctx: &Context<'_>,
        id: GraphQLApiId,
        input: UpdateTaskInput,
    ) -> Result<Task> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Validate input if provided
        let validator = InputValidator::new();
        let sanitizer = ErrorSanitizer::default();
        
        if let Some(ref name) = input.name {
            if let Err(validation_err) = validator.validate_task_name(name) {
                warn!("Invalid task name in GraphQL update_task: {}", validation_err);
                let sanitized_error = sanitizer.sanitize_error(&validation_err);
                return Err(ApiError::bad_request(&sanitized_error.message).into());
            }
        }
        
        if let Some(ref description) = input.description {
            if let Err(validation_err) = validator.validate_string(description, "description") {
                warn!("Invalid description in GraphQL update_task: {}", validation_err);
                let sanitized_error = sanitizer.sanitize_error(&validation_err);
                return Err(ApiError::bad_request(&sanitized_error.message).into());
            }
        }
        
        // Get the existing task
        let task_repo = context.repositories.task_repository();
        let mut existing_task = task_repo.find_by_id(id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch task: {}", e)))?
            .ok_or_else(|| ApiError::not_found("Task", &id.0.to_string()))?;
        
        // Apply updates
        if let Some(name) = input.name {
            existing_task.name = name;
        }
        if let Some(description) = input.description {
            existing_task.description = Some(description);
        }
        if let Some(enabled) = input.enabled {
            existing_task.enabled = enabled;
        }
        if let Some(input_schema) = input.input_schema {
            existing_task.input_schema = Some(input_schema);
        }
        if let Some(output_schema) = input.output_schema {
            existing_task.output_schema = Some(output_schema);
        }
        if let Some(metadata) = input.metadata {
            existing_task.metadata = Some(metadata);
        }
        
        // Update timestamp
        existing_task.updated_at = chrono::Utc::now();
        
        // Update the task using the repository
        let updated_task = task_repo.update(existing_task).await
            .map_err(|e| ApiError::internal_error(format!("Failed to update task: {}", e)))?;
        
        Ok(updated_task)
    }

    /// Delete a task
    async fn delete_task(
        &self,
        ctx: &Context<'_>,
        id: GraphQLApiId,
    ) -> Result<bool> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check if task exists before deletion
        let task_repo = context.repositories.task_repository();
        let existing_task = task_repo.find_by_id(id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch task: {}", e)))?;
        
        if existing_task.is_none() {
            return Err(ApiError::not_found("Task", &id.0.to_string()).into());
        }
        
        // Delete the task using the repository
        task_repo.delete(id.0.as_i32().unwrap_or(0)).await
            .map_err(|e| ApiError::internal_error(format!("Failed to delete task: {}", e)))?;
        
        Ok(true)
    }

    /// Create a new execution
    async fn create_execution(
        &self,
        ctx: &Context<'_>,
        input: CreateExecutionInput,
    ) -> Result<Execution> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Validate that task exists
        let task_repo = context.repositories.task_repository();
        let task = task_repo.find_by_id(input.task_id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch task: {}", e)))?
            .ok_or_else(|| ApiError::bad_request("Task not found"))?;
        
        // Validate input JSON
        let validator = InputValidator::new();
        let input_str = serde_json::to_string(&input.input)
            .map_err(|e| ApiError::bad_request(format!("Invalid input JSON: {}", e)))?;
        if let Err(validation_err) = validator.validate_json(&input_str) {
            warn!("Invalid input JSON in GraphQL create_execution: {}", validation_err);
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(ApiError::bad_request(&sanitized_error.message).into());
        }
        
        // Create UnifiedExecution from input
        let unified_execution = ratchet_api_types::UnifiedExecution {
            id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
            uuid: uuid::Uuid::new_v4(),
            task_id: input.task_id.0,
            input: input.input,
            output: None,
            status: ratchet_api_types::ExecutionStatus::Pending,
            error_message: None,
            error_details: None,
            queued_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            http_requests: None,
            recording_path: None,
            can_retry: false,
            can_cancel: true,
            progress: None,
        };
        
        // Create the execution using the repository
        let execution_repo = context.repositories.execution_repository();
        let created_execution = execution_repo.create(unified_execution).await
            .map_err(|e| ApiError::internal_error(format!("Failed to create execution: {}", e)))?;
        
        Ok(created_execution)
    }

    /// Create a new job
    async fn create_job(
        &self,
        ctx: &Context<'_>,
        input: CreateJobInput,
    ) -> Result<Job> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Validate that task exists
        let task_repo = context.repositories.task_repository();
        let _task = task_repo.find_by_id(input.task_id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch task: {}", e)))?
            .ok_or_else(|| ApiError::bad_request("Task not found"))?;
        
        // Create UnifiedJob from input
        let unified_job = ratchet_api_types::UnifiedJob {
            id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
            task_id: input.task_id.0,
            priority: input.priority.unwrap_or(ratchet_api_types::JobPriority::Normal),
            status: ratchet_api_types::JobStatus::Queued,
            retry_count: 0,
            max_retries: input.max_retries.unwrap_or(3),
            queued_at: chrono::Utc::now(),
            scheduled_for: input.scheduled_for,
            error_message: None,
            output_destinations: None, // TODO: Add support for output destinations in input
        };
        
        // Create the job using the repository
        let job_repo = context.repositories.job_repository();
        let created_job = job_repo.create(unified_job).await
            .map_err(|e| ApiError::internal_error(format!("Failed to create job: {}", e)))?;
        
        Ok(created_job.into())
    }

    /// Create a new schedule
    async fn create_schedule(
        &self,
        ctx: &Context<'_>,
        input: CreateScheduleInput,
    ) -> Result<Schedule> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Validate that task exists
        let task_repo = context.repositories.task_repository();
        let _task = task_repo.find_by_id(input.task_id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch task: {}", e)))?
            .ok_or_else(|| ApiError::bad_request("Task not found"))?;
        
        // Validate input
        let validator = InputValidator::new();
        let sanitizer = ErrorSanitizer::default();
        
        // Validate schedule name
        if let Err(validation_err) = validator.validate_string(&input.name, "name") {
            warn!("Invalid schedule name in GraphQL create_schedule: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(ApiError::bad_request(&sanitized_error.message).into());
        }
        
        // Validate cron expression format (basic validation)
        if input.cron_expression.trim().is_empty() {
            return Err(ApiError::bad_request("Cron expression cannot be empty").into());
        }
        
        // Validate description if provided
        if let Some(ref description) = input.description {
            if let Err(validation_err) = validator.validate_string(description, "description") {
                warn!("Invalid description in GraphQL create_schedule: {}", validation_err);
                let sanitized_error = sanitizer.sanitize_error(&validation_err);
                return Err(ApiError::bad_request(&sanitized_error.message).into());
            }
        }
        
        // Create UnifiedSchedule from input
        let unified_schedule = ratchet_api_types::UnifiedSchedule {
            id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
            task_id: input.task_id.0,
            name: input.name,
            description: input.description,
            cron_expression: input.cron_expression,
            enabled: input.enabled.unwrap_or(true),
            next_run: None, // Will be calculated by the scheduler
            last_run: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            output_destinations: None, // GraphQL doesn't support output destinations yet
        };
        
        // Create the schedule using the repository
        let schedule_repo = context.repositories.schedule_repository();
        let created_schedule = schedule_repo.create(unified_schedule).await
            .map_err(|e| ApiError::internal_error(format!("Failed to create schedule: {}", e)))?;
        
        Ok(created_schedule)
    }

    /// Update an existing schedule
    async fn update_schedule(
        &self,
        ctx: &Context<'_>,
        id: GraphQLApiId,
        input: UpdateScheduleInput,
    ) -> Result<Schedule> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Validate input if provided
        let validator = InputValidator::new();
        let sanitizer = ErrorSanitizer::default();
        
        if let Some(ref name) = input.name {
            if let Err(validation_err) = validator.validate_string(name, "name") {
                warn!("Invalid schedule name in GraphQL update_schedule: {}", validation_err);
                let sanitized_error = sanitizer.sanitize_error(&validation_err);
                return Err(ApiError::bad_request(&sanitized_error.message).into());
            }
        }
        
        if let Some(ref cron_expression) = input.cron_expression {
            if cron_expression.trim().is_empty() {
                return Err(ApiError::bad_request("Cron expression cannot be empty").into());
            }
        }
        
        if let Some(ref description) = input.description {
            if let Err(validation_err) = validator.validate_string(description, "description") {
                warn!("Invalid description in GraphQL update_schedule: {}", validation_err);
                let sanitized_error = sanitizer.sanitize_error(&validation_err);
                return Err(ApiError::bad_request(&sanitized_error.message).into());
            }
        }
        
        // Get the existing schedule
        let schedule_repo = context.repositories.schedule_repository();
        let mut existing_schedule = schedule_repo.find_by_id(id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch schedule: {}", e)))?
            .ok_or_else(|| ApiError::not_found("Schedule", &id.0.to_string()))?;
        
        // Apply updates
        if let Some(name) = input.name {
            existing_schedule.name = name;
        }
        if let Some(description) = input.description {
            existing_schedule.description = Some(description);
        }
        if let Some(cron_expression) = input.cron_expression {
            existing_schedule.cron_expression = cron_expression;
            // Reset next_run when cron expression changes (will be recalculated by scheduler)
            existing_schedule.next_run = None;
        }
        if let Some(enabled) = input.enabled {
            existing_schedule.enabled = enabled;
        }
        
        // Update timestamp
        existing_schedule.updated_at = chrono::Utc::now();
        
        // Update the schedule using the repository
        let updated_schedule = schedule_repo.update(existing_schedule).await
            .map_err(|e| ApiError::internal_error(format!("Failed to update schedule: {}", e)))?;
        
        Ok(updated_schedule)
    }

    /// MCP task development - create a new task with full JavaScript code and testing
    async fn mcp_create_task(
        &self,
        ctx: &Context<'_>,
        input: McpCreateTaskInput,
    ) -> Result<JsonValue> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check if MCP adapter is available
        let mcp_adapter = context.mcp_adapter.as_ref()
            .ok_or_else(|| ApiError::internal_error("MCP service not available"))?;
        
        // Convert GraphQL input to MCP request format
        let mcp_request = ratchet_mcp::server::task_dev_tools::CreateTaskRequest {
            name: input.name.clone(),
            description: input.description,
            code: input.code,
            input_schema: input.input_schema,
            output_schema: input.output_schema,
            tags: input.tags.unwrap_or_default(),
            version: input.version.unwrap_or_else(|| "1.0.0".to_string()),
            enabled: input.enabled.unwrap_or(true),
            test_cases: input.test_cases.unwrap_or_default().into_iter().map(|tc| {
                ratchet_mcp::server::task_dev_tools::TaskTestCase {
                    name: tc.name,
                    input: tc.input,
                    expected_output: tc.expected_output,
                    should_fail: tc.should_fail.unwrap_or(false),
                    description: tc.description,
                }
            }).collect(),
            metadata: std::collections::HashMap::new(),
        };
        
        // Use MCP adapter to create the task (this would need to be implemented in the adapter)
        // For now, return a success response indicating the request structure is valid
        Ok(serde_json::json!({
            "status": "success",
            "message": "MCP task creation request received - implementation in progress",
            "task_name": input.name,
            "version": mcp_request.version,
            "enabled": mcp_request.enabled,
            "test_cases_count": mcp_request.test_cases.len()
        }))
    }

    /// MCP task development - edit an existing task
    async fn mcp_edit_task(
        &self,
        ctx: &Context<'_>,
        input: McpEditTaskInput,
    ) -> Result<JsonValue> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check if MCP adapter is available
        let _mcp_adapter = context.mcp_adapter.as_ref()
            .ok_or_else(|| ApiError::internal_error("MCP service not available"))?;
        
        // Convert GraphQL input to MCP request format
        let _mcp_request = ratchet_mcp::server::task_dev_tools::EditTaskRequest {
            task_id: input.name.clone(),
            code: input.code,
            input_schema: input.input_schema,
            output_schema: input.output_schema,
            description: input.description,
            tags: input.tags,
            validate_changes: true,
            create_backup: true,
        };
        
        // Implementation would use TaskDevelopmentService::edit_task
        Ok(serde_json::json!({
            "status": "success",
            "message": "MCP task editing request received - implementation in progress",
            "task_name": input.name
        }))
    }

    /// MCP task development - delete a task
    async fn mcp_delete_task(
        &self,
        ctx: &Context<'_>,
        task_name: String,
    ) -> Result<bool> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check if MCP adapter is available
        let _mcp_adapter = context.mcp_adapter.as_ref()
            .ok_or_else(|| ApiError::internal_error("MCP service not available"))?;
        
        // Convert to MCP request format
        let _mcp_request = ratchet_mcp::server::task_dev_tools::DeleteTaskRequest {
            task_id: task_name.clone(),
            create_backup: true,
            force: false,
            delete_files: false,
        };
        
        // Implementation would use TaskDevelopmentService::delete_task
        warn!("MCP task deletion not yet fully implemented for task: {}", task_name);
        Ok(true) // Return true to indicate the request was accepted
    }

    /// MCP task development - test a task
    async fn mcp_test_task(
        &self,
        ctx: &Context<'_>,
        task_name: String,
    ) -> Result<McpTaskTestResults> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check if MCP adapter is available
        let _mcp_adapter = context.mcp_adapter.as_ref()
            .ok_or_else(|| ApiError::internal_error("MCP service not available"))?;
        
        // Convert to MCP request format
        let _mcp_request = ratchet_mcp::server::task_dev_tools::RunTaskTestsRequest {
            task_id: task_name.clone(),
            test_names: vec![], // Run all tests
            stop_on_failure: false,
            include_traces: true,
            parallel: false,
        };
        
        // Implementation would use TaskDevelopmentService::run_task_tests
        warn!("MCP task testing not yet fully implemented for task: {}", task_name);
        
        // Return a placeholder result
        Ok(McpTaskTestResults {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            test_results: vec![],
        })
    }

    /// MCP task development - store execution result
    async fn mcp_store_result(
        &self,
        ctx: &Context<'_>,
        input: McpStoreResultInput,
    ) -> Result<JsonValue> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check if MCP adapter is available
        let _mcp_adapter = context.mcp_adapter.as_ref()
            .ok_or_else(|| ApiError::internal_error("MCP service not available"))?;
        
        // Store execution result - this would typically create an execution record
        warn!("MCP result storage not yet fully implemented for task: {}", input.task_id);
        
        // Return success response
        Ok(serde_json::json!({
            "status": "success",
            "message": "MCP result storage request received - implementation in progress",
            "task_id": input.task_id,
            "execution_time_ms": input.execution_time_ms,
            "status_provided": input.status
        }))
    }

    /// Update an existing execution
    async fn update_execution(
        &self,
        ctx: &Context<'_>,
        id: GraphQLApiId,
        input: UpdateExecutionInput,
    ) -> Result<Execution> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Get the existing execution
        let execution_repo = context.repositories.execution_repository();
        let mut existing_execution = execution_repo.find_by_id(id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch execution: {}", e)))?
            .ok_or_else(|| ApiError::not_found("Execution", &id.0.to_string()))?;
        
        // Apply updates
        if let Some(status) = input.status {
            existing_execution.status = status;
        }
        if let Some(output) = input.output {
            existing_execution.output = Some(output);
        }
        if let Some(error_message) = input.error_message {
            existing_execution.error_message = Some(error_message);
        }
        if let Some(error_details) = input.error_details {
            existing_execution.error_details = Some(error_details);
        }
        if let Some(progress) = input.progress {
            existing_execution.progress = Some(progress);
        }
        
        // Update completion timestamp if status changed to completed
        if matches!(existing_execution.status, ratchet_api_types::ExecutionStatus::Completed) && existing_execution.completed_at.is_none() {
            existing_execution.completed_at = Some(chrono::Utc::now());
        }
        
        // Update the execution using the repository
        let updated_execution = execution_repo.update(existing_execution).await
            .map_err(|e| ApiError::internal_error(format!("Failed to update execution: {}", e)))?;
        
        Ok(updated_execution)
    }

    /// Delete an execution
    async fn delete_execution(
        &self,
        ctx: &Context<'_>,
        id: GraphQLApiId,
    ) -> Result<bool> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check if execution exists before deletion
        let execution_repo = context.repositories.execution_repository();
        let existing_execution = execution_repo.find_by_id(id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch execution: {}", e)))?;
        
        if existing_execution.is_none() {
            return Err(ApiError::not_found("Execution", &id.0.to_string()).into());
        }
        
        // Delete the execution using the repository
        execution_repo.delete(id.0.as_i32().unwrap_or(0)).await
            .map_err(|e| ApiError::internal_error(format!("Failed to delete execution: {}", e)))?;
        
        Ok(true)
    }

    /// Update an existing job
    async fn update_job(
        &self,
        ctx: &Context<'_>,
        id: GraphQLApiId,
        input: UpdateJobInput,
    ) -> Result<Job> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Get the existing job
        let job_repo = context.repositories.job_repository();
        let mut existing_job = job_repo.find_by_id(id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch job: {}", e)))?
            .ok_or_else(|| ApiError::not_found("Job", &id.0.to_string()))?;
        
        // Apply updates
        if let Some(priority) = input.priority {
            existing_job.priority = priority;
        }
        if let Some(status) = input.status {
            existing_job.status = status;
        }
        if let Some(scheduled_for) = input.scheduled_for {
            existing_job.scheduled_for = Some(scheduled_for);
        }
        if let Some(max_retries) = input.max_retries {
            existing_job.max_retries = max_retries;
        }
        if let Some(error_message) = input.error_message {
            existing_job.error_message = Some(error_message);
        }
        
        // Update the job using the repository
        let updated_job = job_repo.update(existing_job).await
            .map_err(|e| ApiError::internal_error(format!("Failed to update job: {}", e)))?;
        
        Ok(updated_job.into())
    }

    /// Delete a job
    async fn delete_job(
        &self,
        ctx: &Context<'_>,
        id: GraphQLApiId,
    ) -> Result<bool> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check if job exists before deletion
        let job_repo = context.repositories.job_repository();
        let existing_job = job_repo.find_by_id(id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch job: {}", e)))?;
        
        if existing_job.is_none() {
            return Err(ApiError::not_found("Job", &id.0.to_string()).into());
        }
        
        // Delete the job using the repository
        job_repo.delete(id.0.as_i32().unwrap_or(0)).await
            .map_err(|e| ApiError::internal_error(format!("Failed to delete job: {}", e)))?;
        
        Ok(true)
    }

    /// Delete a schedule
    async fn delete_schedule(
        &self,
        ctx: &Context<'_>,
        id: GraphQLApiId,
    ) -> Result<bool> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check if schedule exists before deletion
        let schedule_repo = context.repositories.schedule_repository();
        let existing_schedule = schedule_repo.find_by_id(id.0.as_i32().unwrap_or(0))
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to fetch schedule: {}", e)))?;
        
        if existing_schedule.is_none() {
            return Err(ApiError::not_found("Schedule", &id.0.to_string()).into());
        }
        
        // Delete the schedule using the repository
        schedule_repo.delete(id.0.as_i32().unwrap_or(0)).await
            .map_err(|e| ApiError::internal_error(format!("Failed to delete schedule: {}", e)))?;
        
        Ok(true)
    }

    /// Execute a task (create a job for execution)
    async fn execute_task(
        &self,
        ctx: &Context<'_>,
        input: ExecuteTaskInput,
    ) -> Result<Job> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Convert output destinations from input to UnifiedJob format
        let output_destinations = input.output_destinations.map(|destinations| {
            destinations.into_iter().map(|dest| {
                ratchet_api_types::UnifiedOutputDestination {
                    destination_type: match dest.destination_type {
                        OutputDestinationType::Webhook => "webhook".to_string(),
                        OutputDestinationType::File => "file".to_string(),
                        OutputDestinationType::Database => "database".to_string(),
                    },
                    template: None,
                    filesystem: None,
                    webhook: dest.webhook.map(|w| ratchet_api_types::UnifiedWebhookConfig {
                        url: w.url,
                        method: ratchet_api_types::HttpMethod::Post, // Default, would need proper conversion
                        timeout_seconds: 30,
                        content_type: Some(w.content_type),
                        retry_policy: w.retry_policy.map(|rp| ratchet_api_types::UnifiedRetryPolicy {
                            max_attempts: rp.max_attempts,
                            initial_delay_seconds: rp.initial_delay_ms / 1000,
                            max_delay_seconds: rp.max_delay_ms / 1000,
                            backoff_multiplier: rp.backoff_multiplier,
                        }),
                        authentication: None,
                    }),
                }
            }).collect()
        });

        // Create a job from the input
        let unified_job = ratchet_api_types::UnifiedJob {
            id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
            task_id: input.task_id.0,
            priority: input.priority.unwrap_or(ratchet_api_types::JobPriority::Normal),
            status: ratchet_api_types::JobStatus::Queued,
            retry_count: 0,
            max_retries: input.max_retries.unwrap_or(3),
            queued_at: chrono::Utc::now(),
            scheduled_for: None,
            error_message: None,
            output_destinations,
        };

        // Create the job using the repository
        let job_repo = context.repositories.job_repository();
        let created_job = job_repo.create(unified_job).await?;
        
        Ok(created_job.into())
    }
}