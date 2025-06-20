use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add output_destinations column to jobs table
        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .add_column(ColumnDef::new(Jobs::OutputDestinations).text().null())
                    .to_owned(),
            )
            .await?;

        // Add output_destinations column to schedules table
        manager
            .alter_table(
                Table::alter()
                    .table(Schedules::Table)
                    .add_column(ColumnDef::new(Schedules::OutputDestinations).text().null())
                    .to_owned(),
            )
            .await?;

        // Create delivery_results table for tracking output delivery
        manager
            .create_table(
                Table::create()
                    .table(DeliveryResults::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DeliveryResults::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DeliveryResults::JobId).integer().not_null())
                    .col(ColumnDef::new(DeliveryResults::ExecutionId).integer().not_null())
                    .col(
                        ColumnDef::new(DeliveryResults::DestinationType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryResults::DestinationId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(DeliveryResults::Success).boolean().not_null())
                    .col(ColumnDef::new(DeliveryResults::DeliveryTimeMs).integer().not_null())
                    .col(ColumnDef::new(DeliveryResults::SizeBytes).integer().not_null())
                    .col(ColumnDef::new(DeliveryResults::ResponseInfo).text().null())
                    .col(ColumnDef::new(DeliveryResults::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(DeliveryResults::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_delivery_results_job_id")
                            .from(DeliveryResults::Table, DeliveryResults::JobId)
                            .to(Jobs::Table, Jobs::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_delivery_results_execution_id")
                            .from(DeliveryResults::Table, DeliveryResults::ExecutionId)
                            .to(Executions::Table, Executions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for delivery_results table
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_delivery_results_job_id")
                    .table(DeliveryResults::Table)
                    .col(DeliveryResults::JobId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_delivery_results_execution_id")
                    .table(DeliveryResults::Table)
                    .col(DeliveryResults::ExecutionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_delivery_results_created_at")
                    .table(DeliveryResults::Table)
                    .col(DeliveryResults::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_delivery_results_destination_type")
                    .table(DeliveryResults::Table)
                    .col(DeliveryResults::DestinationType)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop delivery_results table
        manager
            .drop_table(Table::drop().table(DeliveryResults::Table).to_owned())
            .await?;

        // Remove output_destinations column from schedules table
        manager
            .alter_table(
                Table::alter()
                    .table(Schedules::Table)
                    .drop_column(Schedules::OutputDestinations)
                    .to_owned(),
            )
            .await?;

        // Remove output_destinations column from jobs table
        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .drop_column(Jobs::OutputDestinations)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Jobs {
    Table,
    Id,
    OutputDestinations,
}

#[derive(DeriveIden)]
enum Schedules {
    Table,
    OutputDestinations,
}

#[derive(DeriveIden)]
enum Executions {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum DeliveryResults {
    Table,
    Id,
    JobId,
    ExecutionId,
    DestinationType,
    DestinationId,
    Success,
    DeliveryTimeMs,
    SizeBytes,
    ResponseInfo,
    ErrorMessage,
    CreatedAt,
}
