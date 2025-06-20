//! Heartbeat system for health monitoring

use anyhow::Result;
use chrono::Utc;
use cron::Schedule;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

use ratchet_interfaces::database::{RepositoryFactory, ScheduleFilters};
use ratchet_api_types::PaginationInput;
use ratchet_output::{OutputDeliveryManager, OutputDestinationConfig, OutputFormat};

use crate::config::HeartbeatConfig;

/// System-wide heartbeat identifier
const HEARTBEAT_TASK_ID: &str = "00000000-0000-0000-0000-000000000001";
const HEARTBEAT_SCHEDULE_NAME: &str = "system_heartbeat";

/// Heartbeat service for managing system health monitoring
pub struct HeartbeatService {
    config: HeartbeatConfig,
    repositories: Arc<dyn RepositoryFactory>,
    output_manager: Arc<OutputDeliveryManager>,
}

impl HeartbeatService {
    /// Create a new heartbeat service
    pub fn new(
        config: HeartbeatConfig,
        repositories: Arc<dyn RepositoryFactory>,
        output_manager: Arc<OutputDeliveryManager>,
    ) -> Self {
        Self {
            config,
            repositories,
            output_manager,
        }
    }

    /// Initialize the heartbeat system on server startup
    pub async fn initialize(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Heartbeat system disabled in configuration");
            return Ok(());
        }

        info!("Initializing heartbeat system");

        // Validate and normalize cron schedule
        let normalized_cron = self.normalize_cron_schedule(&self.config.cron_schedule)?;
        if let Err(e) = Schedule::from_str(&normalized_cron) {
            error!("Invalid heartbeat cron schedule '{}' (normalized: '{}'): {}", 
                   self.config.cron_schedule, normalized_cron, e);
            warn!("Heartbeat system will be disabled due to invalid cron schedule");
            return Ok(()); // Don't fail server startup
        }

        // Setup output destinations (non-fatal if it fails)
        if let Err(e) = self.setup_output_destinations().await {
            warn!("Failed to setup heartbeat output destinations: {}. Continuing without heartbeat.", e);
            return Ok(());
        }

        // Create or update the heartbeat schedule (non-fatal if it fails)
        if let Err(e) = self.ensure_heartbeat_schedule().await {
            warn!("Failed to create heartbeat schedule: {}. Heartbeat system will be unavailable.", e);
            return Ok(());
        }

        info!(
            "Heartbeat system initialized with schedule '{}' and {} output destinations",
            normalized_cron,
            self.config.output_destinations.len()
        );

        Ok(())
    }

    /// Normalize cron schedule to the expected format
    fn normalize_cron_schedule(&self, cron_expr: &str) -> Result<String> {
        let parts: Vec<&str> = cron_expr.split_whitespace().collect();
        
        match parts.len() {
            5 => {
                // Standard 5-field cron (min hour day month dow) - add seconds
                Ok(format!("0 {}", cron_expr))
            }
            6 => {
                // Already has seconds field
                Ok(cron_expr.to_string())
            }
            _ => {
                // Try some common patterns
                match cron_expr {
                    "*/5 * * * *" => Ok("0 */5 * * * *".to_string()), // Every 5 minutes
                    "*/10 * * * *" => Ok("0 */10 * * * *".to_string()), // Every 10 minutes
                    "0 * * * *" => Ok("0 0 * * * *".to_string()), // Every hour
                    _ => Err(anyhow::anyhow!(
                        "Invalid cron expression format. Expected 5 or 6 fields, got {}: '{}'", 
                        parts.len(), cron_expr
                    ))
                }
            }
        }
    }

    /// Setup output destinations for heartbeat
    async fn setup_output_destinations(&self) -> Result<()> {
        for destination_name in &self.config.output_destinations {
            match destination_name.as_str() {
                "stdio" => {
                    let config = OutputDestinationConfig::Stdio {
                        stream: "stdout".to_string(),
                        format: OutputFormat::Json,
                        include_metadata: false,
                        line_buffered: true,
                        prefix: Some("[HEARTBEAT] ".to_string()),
                    };
                    
                    self.output_manager
                        .add_destination("heartbeat_stdio".to_string(), config)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to add stdio destination: {}", e))?;
                    
                    debug!("Added stdio output destination for heartbeat");
                }
                name => {
                    warn!("Unknown heartbeat output destination: {}", name);
                }
            }
        }
        Ok(())
    }

    /// Ensure the heartbeat schedule exists in the database
    async fn ensure_heartbeat_schedule(&self) -> Result<()> {
        let schedule_repo = self.repositories.schedule_repository();

        // Check if heartbeat schedule already exists
        let filters = ScheduleFilters {
            task_id: None,
            enabled: None,
            next_run_before: None,
            task_id_in: None,
            id_in: None,
            name_contains: None,
            name_exact: Some(HEARTBEAT_SCHEDULE_NAME.to_string()),
            name_starts_with: None,
            name_ends_with: None,
            cron_expression_contains: None,
            cron_expression_exact: None,
            next_run_after: None,
            last_run_after: None,
            last_run_before: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            has_next_run: None,
            has_last_run: None,
            is_due: None,
            overdue: None,
        };
        let pagination = PaginationInput { 
            page: Some(1), 
            limit: Some(1), 
            offset: None 
        };
        
        match schedule_repo.find_with_filters(filters, pagination).await {
            Ok(response) if !response.items.is_empty() => {
                let existing_schedule = &response.items[0];
                debug!("Heartbeat schedule already exists with ID: {}", existing_schedule.id);
                
                // Update schedule if cron expression has changed
                let normalized_cron = self.normalize_cron_schedule(&self.config.cron_schedule)?;
                if existing_schedule.cron_expression != normalized_cron {
                    info!(
                        "Updating heartbeat schedule cron from '{}' to '{}'",
                        existing_schedule.cron_expression,
                        normalized_cron
                    );
                    
                    let mut updated_schedule = existing_schedule.clone();
                    updated_schedule.cron_expression = normalized_cron.clone();
                    updated_schedule.updated_at = Utc::now();
                    
                    // Calculate next run time
                    if let Ok(schedule) = Schedule::from_str(&normalized_cron) {
                        if let Some(next_run) = schedule.upcoming(Utc).next() {
                            updated_schedule.next_run = Some(next_run);
                        }
                    }
                    
                    schedule_repo.update(updated_schedule).await
                        .map_err(|e| anyhow::anyhow!("Failed to update heartbeat schedule: {}", e))?;
                }
            }
            Ok(response) if response.items.is_empty() => {
                info!("Creating new heartbeat schedule");
                self.create_heartbeat_schedule().await?;
            }
            Ok(_) => {
                // This should not happen since we're filtering by exact name
                warn!("Unexpected: multiple heartbeat schedules found");
            }
            Err(e) => {
                error!("Failed to check for existing heartbeat schedule: {}", e);
                return Err(anyhow::anyhow!("Database error: {}", e));
            }
        }

        Ok(())
    }

    /// Create a new heartbeat schedule
    async fn create_heartbeat_schedule(&self) -> Result<()> {
        let schedule_repo = self.repositories.schedule_repository();
        let task_repo = self.repositories.task_repository();

        // First, ensure the heartbeat task exists - if not, we'll need to register it from embedded tasks
        let heartbeat_task = match task_repo.find_by_name("heartbeat").await {
            Ok(Some(task)) => task,
            Ok(None) => {
                return Err(anyhow::anyhow!(
                    "Heartbeat task not found in database. Please ensure embedded tasks are loaded."
                ));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to check for heartbeat task: {}", e));
            }
        };

        // Normalize and parse cron schedule to get next run time
        let normalized_cron = self.normalize_cron_schedule(&self.config.cron_schedule)?;
        let schedule = Schedule::from_str(&normalized_cron)
            .map_err(|e| anyhow::anyhow!("Invalid cron schedule: {}", e))?;
        
        let next_run_at = schedule.upcoming(Utc).next();

        // Create the heartbeat schedule using the API types
        let heartbeat_schedule = ratchet_api_types::UnifiedSchedule {
            id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
            task_id: heartbeat_task.id,
            name: HEARTBEAT_SCHEDULE_NAME.to_string(),
            description: Some("System heartbeat health monitoring".to_string()),
            cron_expression: normalized_cron,
            enabled: true,
            next_run: next_run_at,
            last_run: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            output_destinations: None,
        };

        let created_schedule = schedule_repo.create(heartbeat_schedule).await
            .map_err(|e| anyhow::anyhow!("Failed to create heartbeat schedule: {}", e))?;

        info!(
            "Created heartbeat schedule with ID {:?} - next run: {:?}",
            created_schedule.id,
            created_schedule.next_run
        );

        Ok(())
    }

    /// Get the current heartbeat configuration
    pub fn config(&self) -> &HeartbeatConfig {
        &self.config
    }

    /// Check if heartbeat is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the next scheduled heartbeat time
    pub async fn next_heartbeat_time(&self) -> Result<Option<chrono::DateTime<Utc>>> {
        let schedule_repo = self.repositories.schedule_repository();
        
        let filters = ScheduleFilters {
            task_id: None,
            enabled: None,
            next_run_before: None,
            task_id_in: None,
            id_in: None,
            name_contains: None,
            name_exact: Some(HEARTBEAT_SCHEDULE_NAME.to_string()),
            name_starts_with: None,
            name_ends_with: None,
            cron_expression_contains: None,
            cron_expression_exact: None,
            next_run_after: None,
            last_run_after: None,
            last_run_before: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            has_next_run: None,
            has_last_run: None,
            is_due: None,
            overdue: None,
        };
        let pagination = PaginationInput { page: Some(1), limit: Some(1), offset: None };
        
        match schedule_repo.find_with_filters(filters, pagination).await {
            Ok(response) if !response.items.is_empty() => Ok(response.items[0].next_run),
            Ok(_) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to get heartbeat schedule: {}", e)),
        }
    }

    /// Get heartbeat execution statistics
    pub async fn execution_stats(&self) -> Result<HeartbeatStats> {
        let schedule_repo = self.repositories.schedule_repository();
        
        let filters = ScheduleFilters {
            task_id: None,
            enabled: None,
            next_run_before: None,
            task_id_in: None,
            id_in: None,
            name_contains: None,
            name_exact: Some(HEARTBEAT_SCHEDULE_NAME.to_string()),
            name_starts_with: None,
            name_ends_with: None,
            cron_expression_contains: None,
            cron_expression_exact: None,
            next_run_after: None,
            last_run_after: None,
            last_run_before: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            has_next_run: None,
            has_last_run: None,
            is_due: None,
            overdue: None,
        };
        let pagination = PaginationInput { page: Some(1), limit: Some(1), offset: None };
        
        match schedule_repo.find_with_filters(filters, pagination).await {
            Ok(response) if !response.items.is_empty() => {
                let schedule = &response.items[0];
                Ok(HeartbeatStats {
                    enabled: schedule.enabled,
                    execution_count: 0, // Not available in current API
                    last_run_at: schedule.last_run,
                    next_run_at: schedule.next_run,
                    last_status: None, // Not available in current API
                    cron_schedule: schedule.cron_expression.clone(),
                })
            }
            Ok(_) => {
                Ok(HeartbeatStats {
                    enabled: false,
                    execution_count: 0,
                    last_run_at: None,
                    next_run_at: None,
                    last_status: None,
                    cron_schedule: "Not configured".to_string(),
                })
            }
            Err(e) => Err(anyhow::anyhow!("Failed to get heartbeat stats: {}", e)),
        }
    }
}

/// Heartbeat execution statistics
#[derive(Debug, Clone)]
pub struct HeartbeatStats {
    pub enabled: bool,
    pub execution_count: u64,
    pub last_run_at: Option<chrono::DateTime<Utc>>,
    pub next_run_at: Option<chrono::DateTime<Utc>>,
    pub last_status: Option<String>,
    pub cron_schedule: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    
    // Simple mock for testing that doesn't panic on unimplemented methods
    struct TestRepositoryFactory;
    
    #[async_trait]
    impl RepositoryFactory for TestRepositoryFactory {
        fn task_repository(&self) -> &dyn ratchet_interfaces::TaskRepository {
            unimplemented!("Not needed for these tests")
        }
        
        fn execution_repository(&self) -> &dyn ratchet_interfaces::ExecutionRepository {
            unimplemented!("Not needed for these tests")
        }
        
        fn job_repository(&self) -> &dyn ratchet_interfaces::JobRepository {
            unimplemented!("Not needed for these tests")
        }
        
        fn schedule_repository(&self) -> &dyn ratchet_interfaces::ScheduleRepository {
            unimplemented!("Not needed for these tests")
        }
        
        fn user_repository(&self) -> &dyn ratchet_interfaces::database::UserRepository {
            unimplemented!("Not needed for these tests")
        }
        
        fn session_repository(&self) -> &dyn ratchet_interfaces::database::SessionRepository {
            unimplemented!("Not needed for these tests")
        }
        
        fn api_key_repository(&self) -> &dyn ratchet_interfaces::database::ApiKeyRepository {
            unimplemented!("Not needed for these tests")
        }
        
        async fn health_check(&self) -> Result<(), ratchet_interfaces::DatabaseError> {
            Ok(())
        }
    }

    #[test]
    fn test_cron_schedule_normalization() {
        let config = HeartbeatConfig::default();
        let repositories = std::sync::Arc::new(TestRepositoryFactory);
        let output_manager = std::sync::Arc::new(ratchet_output::OutputDeliveryManager::new());
        
        let service = HeartbeatService::new(config, repositories, output_manager);

        // Test 5-field cron (standard format)
        assert_eq!(
            service.normalize_cron_schedule("*/5 * * * *").unwrap(),
            "0 */5 * * * *"
        );

        // Test 6-field cron (already normalized)
        assert_eq!(
            service.normalize_cron_schedule("0 */5 * * * *").unwrap(),
            "0 */5 * * * *"
        );

        // Test common patterns
        assert_eq!(
            service.normalize_cron_schedule("*/10 * * * *").unwrap(),
            "0 */10 * * * *"
        );

        assert_eq!(
            service.normalize_cron_schedule("0 * * * *").unwrap(),
            "0 0 * * * *"
        );

        // Test invalid patterns
        assert!(service.normalize_cron_schedule("* * *").is_err());
        assert!(service.normalize_cron_schedule("invalid cron").is_err());
    }

    #[test]
    fn test_cron_schedule_validation() {
        let config = HeartbeatConfig::default();
        let repositories = std::sync::Arc::new(TestRepositoryFactory);
        let output_manager = std::sync::Arc::new(ratchet_output::OutputDeliveryManager::new());
        
        let service = HeartbeatService::new(config, repositories, output_manager);

        // Test valid cron schedules
        let valid_schedules = vec![
            "0 */5 * * * *",
            "0 0 * * * *",
            "0 */10 * * * *",
            "0 0 0 * * *",
        ];

        for schedule in valid_schedules {
            let normalized = service.normalize_cron_schedule(schedule).unwrap();
            assert!(Schedule::from_str(&normalized).is_ok(), "Failed to parse: {}", normalized);
        }
    }
}