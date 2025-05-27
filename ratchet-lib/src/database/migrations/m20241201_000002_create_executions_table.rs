use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Executions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Executions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Executions::Uuid)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Executions::TaskId).integer().not_null())
                    .col(ColumnDef::new(Executions::Input).json().not_null())
                    .col(ColumnDef::new(Executions::Output).json())
                    .col(
                        ColumnDef::new(Executions::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(Executions::ErrorMessage).text())
                    .col(ColumnDef::new(Executions::ErrorDetails).json())
                    .col(
                        ColumnDef::new(Executions::QueuedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Executions::StartedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Executions::CompletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Executions::DurationMs).integer())
                    .col(ColumnDef::new(Executions::HttpRequests).json())
                    .col(ColumnDef::new(Executions::RecordingPath).string())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_executions_task_id")
                            .from(Executions::Table, Executions::TaskId)
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Executions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Executions {
    Table,
    Id,
    Uuid,
    TaskId,
    Input,
    Output,
    Status,
    ErrorMessage,
    ErrorDetails,
    QueuedAt,
    StartedAt,
    CompletedAt,
    DurationMs,
    HttpRequests,
    RecordingPath,
}

#[derive(Iden)]
enum Tasks {
    Table,
    Id,
}