use crate::database::{
    entities::{schedules, Schedule, ScheduleActiveModel, Schedules},
    DatabaseConnection, DatabaseError,
};
use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, Order, PaginatorTrait};

/// Repository for schedule-related database operations
#[derive(Clone)]
pub struct ScheduleRepository {
    db: DatabaseConnection,
}

impl ScheduleRepository {
    /// Create a new schedule repository
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new schedule
    pub async fn create(&self, schedule: Schedule) -> Result<Schedule, DatabaseError> {
        let active_model = ScheduleActiveModel {
            uuid: Set(schedule.uuid),
            task_id: Set(schedule.task_id),
            name: Set(schedule.name),
            cron_expression: Set(schedule.cron_expression),
            input_data: Set(schedule.input_data),
            enabled: Set(schedule.enabled),
            next_run_at: Set(schedule.next_run_at),
            last_run_at: Set(schedule.last_run_at),
            execution_count: Set(schedule.execution_count),
            max_executions: Set(schedule.max_executions),
            metadata: Set(schedule.metadata),
            created_at: Set(schedule.created_at),
            updated_at: Set(schedule.updated_at),
            ..Default::default()
        };

        let result = active_model.insert(self.db.get_connection()).await?;
        Ok(result)
    }

    /// Find schedule by ID
    pub async fn find_by_id(&self, id: i32) -> Result<Option<Schedule>, DatabaseError> {
        let schedule = Schedules::find_by_id(id)
            .one(self.db.get_connection())
            .await?;
        Ok(schedule)
    }

    /// Find schedules by task ID
    pub async fn find_by_task_id(&self, task_id: i32) -> Result<Vec<Schedule>, DatabaseError> {
        let schedules = Schedules::find()
            .filter(schedules::Column::TaskId.eq(task_id))
            .all(self.db.get_connection())
            .await?;
        Ok(schedules)
    }

    /// Find enabled schedules
    pub async fn find_enabled(&self) -> Result<Vec<Schedule>, DatabaseError> {
        let schedules = Schedules::find()
            .filter(schedules::Column::Enabled.eq(true))
            .all(self.db.get_connection())
            .await?;
        Ok(schedules)
    }

    /// Find schedules ready to run
    pub async fn find_ready_to_run(&self) -> Result<Vec<Schedule>, DatabaseError> {
        let now = chrono::Utc::now();
        let schedules = Schedules::find()
            .filter(schedules::Column::Enabled.eq(true))
            .filter(schedules::Column::NextRunAt.lte(now))
            .order_by(schedules::Column::NextRunAt, Order::Asc)
            .all(self.db.get_connection())
            .await?;
        Ok(schedules)
    }

    /// Update schedule
    pub async fn update(&self, schedule: Schedule) -> Result<Schedule, DatabaseError> {
        let mut active_model: ScheduleActiveModel = schedule.into();
        active_model.updated_at = Set(chrono::Utc::now());
        
        let updated_schedule = active_model.update(self.db.get_connection()).await?;
        Ok(updated_schedule)
    }

    /// Update schedule next run time
    pub async fn update_next_run(&self, id: i32, next_run_at: Option<chrono::DateTime<chrono::Utc>>) -> Result<(), DatabaseError> {
        let active_model = ScheduleActiveModel {
            id: Set(id),
            next_run_at: Set(next_run_at),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        };

        active_model.update(self.db.get_connection()).await?;
        Ok(())
    }

    /// Record schedule execution
    pub async fn record_execution(&self, id: i32) -> Result<(), DatabaseError> {
        // First get the current schedule to increment execution count
        let schedule = self.find_by_id(id).await?;
        if let Some(mut schedule) = schedule {
            schedule.record_execution();
            
            let active_model = ScheduleActiveModel {
                id: Set(id),
                last_run_at: Set(schedule.last_run_at),
                execution_count: Set(schedule.execution_count),
                updated_at: Set(schedule.updated_at),
                ..Default::default()
            };

            active_model.update(self.db.get_connection()).await?;
        }
        Ok(())
    }

    /// Enable or disable schedule
    pub async fn set_enabled(&self, id: i32, enabled: bool) -> Result<(), DatabaseError> {
        let active_model = ScheduleActiveModel {
            id: Set(id),
            enabled: Set(enabled),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        };

        active_model.update(self.db.get_connection()).await?;
        Ok(())
    }

    /// Delete schedule
    pub async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        Schedules::delete_by_id(id)
            .exec(self.db.get_connection())
            .await?;
        Ok(())
    }

    /// Count schedules
    pub async fn count(&self) -> Result<u64, DatabaseError> {
        let count = Schedules::find().count(self.db.get_connection()).await?;
        Ok(count)
    }

    /// Count enabled schedules
    pub async fn count_enabled(&self) -> Result<u64, DatabaseError> {
        let count = Schedules::find()
            .filter(schedules::Column::Enabled.eq(true))
            .count(self.db.get_connection())
            .await?;
        Ok(count)
    }
}

#[async_trait(?Send)]
impl super::Repository for ScheduleRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.count().await?;
        Ok(())
    }
}