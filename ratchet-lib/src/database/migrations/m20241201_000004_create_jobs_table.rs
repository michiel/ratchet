use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Jobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Jobs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Jobs::Uuid)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Jobs::TaskId).integer().not_null())
                    .col(ColumnDef::new(Jobs::ExecutionId).integer())
                    .col(ColumnDef::new(Jobs::ScheduleId).integer())
                    .col(
                        ColumnDef::new(Jobs::Priority)
                            .string()
                            .not_null()
                            .default("normal"),
                    )
                    .col(
                        ColumnDef::new(Jobs::Status)
                            .string()
                            .not_null()
                            .default("queued"),
                    )
                    .col(ColumnDef::new(Jobs::InputData).json().not_null())
                    .col(
                        ColumnDef::new(Jobs::RetryCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Jobs::MaxRetries)
                            .integer()
                            .not_null()
                            .default(3),
                    )
                    .col(
                        ColumnDef::new(Jobs::RetryDelaySeconds)
                            .integer()
                            .not_null()
                            .default(60),
                    )
                    .col(ColumnDef::new(Jobs::ErrorMessage).text())
                    .col(ColumnDef::new(Jobs::ErrorDetails).json())
                    .col(
                        ColumnDef::new(Jobs::QueuedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Jobs::ProcessAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Jobs::StartedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Jobs::CompletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Jobs::Metadata).json())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_jobs_task_id")
                            .from(Jobs::Table, Jobs::TaskId)
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_jobs_execution_id")
                            .from(Jobs::Table, Jobs::ExecutionId)
                            .to(Executions::Table, Executions::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_jobs_schedule_id")
                            .from(Jobs::Table, Jobs::ScheduleId)
                            .to(Schedules::Table, Schedules::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Jobs::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Jobs {
    Table,
    Id,
    Uuid,
    TaskId,
    ExecutionId,
    ScheduleId,
    Priority,
    Status,
    InputData,
    RetryCount,
    MaxRetries,
    RetryDelaySeconds,
    ErrorMessage,
    ErrorDetails,
    QueuedAt,
    ProcessAt,
    StartedAt,
    CompletedAt,
    Metadata,
}

#[derive(Iden)]
enum Tasks {
    Table,
    Id,
}

#[derive(Iden)]
enum Executions {
    Table,
    Id,
}

#[derive(Iden)]
enum Schedules {
    Table,
    Id,
}