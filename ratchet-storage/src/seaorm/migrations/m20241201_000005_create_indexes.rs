use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Index on tasks.uuid for fast UUID lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_uuid")
                    .table(Tasks::Table)
                    .col(Tasks::Uuid)
                    .to_owned(),
            )
            .await?;

        // Index on tasks.name for fast name searches
        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_name")
                    .table(Tasks::Table)
                    .col(Tasks::Name)
                    .to_owned(),
            )
            .await?;

        // Index on tasks.enabled for filtering active tasks
        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_enabled")
                    .table(Tasks::Table)
                    .col(Tasks::Enabled)
                    .to_owned(),
            )
            .await?;

        // Index on executions.task_id for fast task execution lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_executions_task_id")
                    .table(Executions::Table)
                    .col(Executions::TaskId)
                    .to_owned(),
            )
            .await?;

        // Index on executions.status for filtering by execution status
        manager
            .create_index(
                Index::create()
                    .name("idx_executions_status")
                    .table(Executions::Table)
                    .col(Executions::Status)
                    .to_owned(),
            )
            .await?;

        // Index on executions.queued_at for chronological ordering
        manager
            .create_index(
                Index::create()
                    .name("idx_executions_queued_at")
                    .table(Executions::Table)
                    .col(Executions::QueuedAt)
                    .to_owned(),
            )
            .await?;

        // Index on schedules.task_id for fast schedule lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_schedules_task_id")
                    .table(Schedules::Table)
                    .col(Schedules::TaskId)
                    .to_owned(),
            )
            .await?;

        // Index on schedules.enabled for filtering active schedules
        manager
            .create_index(
                Index::create()
                    .name("idx_schedules_enabled")
                    .table(Schedules::Table)
                    .col(Schedules::Enabled)
                    .to_owned(),
            )
            .await?;

        // Index on schedules.next_run_at for scheduler queries
        manager
            .create_index(
                Index::create()
                    .name("idx_schedules_next_run_at")
                    .table(Schedules::Table)
                    .col(Schedules::NextRunAt)
                    .to_owned(),
            )
            .await?;

        // Composite index on jobs (status, priority, queued_at) for job queue processing
        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_queue_processing")
                    .table(Jobs::Table)
                    .col(Jobs::Status)
                    .col(Jobs::Priority)
                    .col(Jobs::QueuedAt)
                    .to_owned(),
            )
            .await?;

        // Index on jobs.task_id for fast job lookups by task
        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_task_id")
                    .table(Jobs::Table)
                    .col(Jobs::TaskId)
                    .to_owned(),
            )
            .await?;

        // Index on jobs.schedule_id for fast scheduled job lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_schedule_id")
                    .table(Jobs::Table)
                    .col(Jobs::ScheduleId)
                    .to_owned(),
            )
            .await?;

        // Index on jobs.process_at for delayed job processing
        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_process_at")
                    .table(Jobs::Table)
                    .col(Jobs::ProcessAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop all indexes in reverse order
        let indexes = vec![
            "idx_jobs_process_at",
            "idx_jobs_schedule_id",
            "idx_jobs_task_id",
            "idx_jobs_queue_processing",
            "idx_schedules_next_run_at",
            "idx_schedules_enabled",
            "idx_schedules_task_id",
            "idx_executions_queued_at",
            "idx_executions_status",
            "idx_executions_task_id",
            "idx_tasks_enabled",
            "idx_tasks_name",
            "idx_tasks_uuid",
        ];

        for index_name in indexes {
            manager
                .drop_index(Index::drop().name(index_name).to_owned())
                .await?;
        }

        Ok(())
    }
}

#[derive(Iden)]
enum Tasks {
    Table,
    Uuid,
    Name,
    Enabled,
}

#[derive(Iden)]
enum Executions {
    Table,
    TaskId,
    Status,
    QueuedAt,
}

#[derive(Iden)]
enum Schedules {
    Table,
    TaskId,
    Enabled,
    NextRunAt,
}

#[derive(Iden)]
enum Jobs {
    Table,
    TaskId,
    ScheduleId,
    Status,
    Priority,
    QueuedAt,
    ProcessAt,
}