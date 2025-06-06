use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Schedules::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Schedules::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Schedules::Uuid)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Schedules::TaskId).integer().not_null())
                    .col(ColumnDef::new(Schedules::Name).string().not_null())
                    .col(
                        ColumnDef::new(Schedules::CronExpression)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Schedules::InputData).json().not_null())
                    .col(
                        ColumnDef::new(Schedules::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(Schedules::NextRunAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Schedules::LastRunAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Schedules::ExecutionCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Schedules::MaxExecutions).integer())
                    .col(ColumnDef::new(Schedules::Metadata).json())
                    .col(
                        ColumnDef::new(Schedules::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Schedules::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_schedules_task_id")
                            .from(Schedules::Table, Schedules::TaskId)
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Schedules::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Schedules {
    Table,
    Id,
    Uuid,
    TaskId,
    Name,
    CronExpression,
    InputData,
    Enabled,
    NextRunAt,
    LastRunAt,
    ExecutionCount,
    MaxExecutions,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Tasks {
    Table,
    Id,
}
