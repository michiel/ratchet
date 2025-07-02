use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create task_repositories table first
        manager
            .create_table(
                Table::create()
                    .table(TaskRepositories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskRepositories::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskRepositories::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(TaskRepositories::RepositoryType).string().not_null())
                    .col(ColumnDef::new(TaskRepositories::Uri).string().not_null())
                    .col(ColumnDef::new(TaskRepositories::Branch).string())
                    .col(ColumnDef::new(TaskRepositories::AuthConfig).json())
                    .col(ColumnDef::new(TaskRepositories::SyncEnabled).boolean().not_null().default(true))
                    .col(ColumnDef::new(TaskRepositories::SyncIntervalMinutes).integer())
                    .col(ColumnDef::new(TaskRepositories::LastSyncAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(TaskRepositories::SyncStatus).string().not_null().default("pending"))
                    .col(ColumnDef::new(TaskRepositories::SyncError).text())
                    .col(ColumnDef::new(TaskRepositories::Priority).integer().not_null().default(1))
                    .col(ColumnDef::new(TaskRepositories::IsDefault).boolean().not_null().default(false))
                    .col(ColumnDef::new(TaskRepositories::IsWritable).boolean().not_null().default(true))
                    .col(ColumnDef::new(TaskRepositories::WatchPatterns).json().not_null())
                    .col(ColumnDef::new(TaskRepositories::IgnorePatterns).json().not_null())
                    .col(ColumnDef::new(TaskRepositories::PushOnChange).boolean().not_null().default(false))
                    .col(ColumnDef::new(TaskRepositories::Metadata).json().not_null())
                    .col(
                        ColumnDef::new(TaskRepositories::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TaskRepositories::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create task_versions table
        manager
            .create_table(
                Table::create()
                    .table(TaskVersions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskVersions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskVersions::TaskId).integer().not_null())
                    .col(ColumnDef::new(TaskVersions::RepositoryId).integer().not_null())
                    .col(ColumnDef::new(TaskVersions::Version).string().not_null())
                    .col(ColumnDef::new(TaskVersions::SourceCode).text().not_null())
                    .col(ColumnDef::new(TaskVersions::InputSchema).json().not_null())
                    .col(ColumnDef::new(TaskVersions::OutputSchema).json().not_null())
                    .col(ColumnDef::new(TaskVersions::Metadata).json().not_null())
                    .col(ColumnDef::new(TaskVersions::Checksum).string().not_null())
                    .col(ColumnDef::new(TaskVersions::ChangeDescription).text())
                    .col(ColumnDef::new(TaskVersions::ChangedBy).string().not_null())
                    .col(ColumnDef::new(TaskVersions::ChangeSource).string().not_null())
                    .col(ColumnDef::new(TaskVersions::RepositoryCommit).string())
                    .col(
                        ColumnDef::new(TaskVersions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_versions_task_id")
                            .from(TaskVersions::Table, TaskVersions::TaskId)
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_versions_repository_id")
                            .from(TaskVersions::Table, TaskVersions::RepositoryId)
                            .to(TaskRepositories::Table, TaskRepositories::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Insert default filesystem repository
        let now = chrono::Utc::now();
        manager
            .exec_stmt(
                sea_query::Query::insert()
                    .into_table(TaskRepositories::Table)
                    .columns([
                        TaskRepositories::Name,
                        TaskRepositories::RepositoryType,
                        TaskRepositories::Uri,
                        TaskRepositories::SyncEnabled,
                        TaskRepositories::SyncIntervalMinutes,
                        TaskRepositories::SyncStatus,
                        TaskRepositories::Priority,
                        TaskRepositories::IsDefault,
                        TaskRepositories::IsWritable,
                        TaskRepositories::WatchPatterns,
                        TaskRepositories::IgnorePatterns,
                        TaskRepositories::PushOnChange,
                        TaskRepositories::Metadata,
                        TaskRepositories::CreatedAt,
                        TaskRepositories::UpdatedAt,
                    ])
                    .values([
                        "default-filesystem".into(),
                        "filesystem".into(),
                        "./tasks".into(),
                        true.into(),
                        5.into(),
                        "pending".into(),
                        1.into(),
                        true.into(),
                        true.into(),
                        serde_json::json!(["**/*.js", "**/task.yaml", "**/task.json"]).into(),
                        serde_json::json!(["**/node_modules/**", "**/.git/**", "**/target/**"]).into(),
                        false.into(),
                        serde_json::json!({}).into(),
                        now.into(),
                        now.into(),
                    ]).map_err(|e| DbErr::Custom(format!("Failed to build insert query: {}", e)))?
                    .to_owned(),
            )
            .await?;

        // SQLite doesn't support multiple alter operations in one statement
        // So we need to split these into separate ALTER TABLE statements

        // Note: SQLite doesn't support modifying column types, but since we're making 
        // the path column nullable (less restrictive), we can skip this step.
        // The column will work as optional even without explicitly changing the constraint.

        // Add source code field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::SourceCode).text().not_null().default(""))
                    .to_owned(),
            )
            .await?;

        // Add source type field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::SourceType).string().not_null().default("javascript"))
                    .to_owned(),
            )
            .await?;

        // Add storage type field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::StorageType).string().not_null().default("database"))
                    .to_owned(),
            )
            .await?;

        // Add file path field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::FilePath).string())
                    .to_owned(),
            )
            .await?;

        // Add checksum field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::Checksum).string().not_null().default(""))
                    .to_owned(),
            )
            .await?;

        // Add repository ID field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::RepositoryId).integer().not_null().default(1))
                    .to_owned(),
            )
            .await?;

        // Add repository path field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::RepositoryPath).string().not_null().default(""))
                    .to_owned(),
            )
            .await?;

        // Add last synced at field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::LastSyncedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        // Add sync status field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::SyncStatus).string().not_null().default("synced"))
                    .to_owned(),
            )
            .await?;

        // Add is editable field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::IsEditable).boolean().not_null().default(true))
                    .to_owned(),
            )
            .await?;

        // Add created from field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::CreatedFrom).string().not_null().default("import"))
                    .to_owned(),
            )
            .await?;

        // Add needs push field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::NeedsPush).boolean().not_null().default(false))
                    .to_owned(),
            )
            .await?;

        // Add source modified at field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::SourceModifiedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        // Note: SQLite doesn't support adding foreign key constraints to existing tables.
        // In a production environment, you would need to recreate the table with the constraint.
        // For now, we'll rely on application-level referential integrity.

        // Create indexes for better performance
        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_repository_id")
                    .table(Tasks::Table)
                    .col(Tasks::RepositoryId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_sync_status")
                    .table(Tasks::Table)
                    .col(Tasks::SyncStatus)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_needs_push")
                    .table(Tasks::Table)
                    .col(Tasks::NeedsPush)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_task_repositories_is_default")
                    .table(TaskRepositories::Table)
                    .col(TaskRepositories::IsDefault)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_task_versions_task_id")
                    .table(TaskVersions::Table)
                    .col(TaskVersions::TaskId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Note: No foreign key constraint was added due to SQLite limitations

        // Remove added columns from tasks table (SQLite requires separate statements)
        
        // Drop source modified at field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::SourceModifiedAt)
                    .to_owned(),
            )
            .await?;

        // Drop needs push field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::NeedsPush)
                    .to_owned(),
            )
            .await?;

        // Drop created from field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::CreatedFrom)
                    .to_owned(),
            )
            .await?;

        // Drop is editable field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::IsEditable)
                    .to_owned(),
            )
            .await?;

        // Drop sync status field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::SyncStatus)
                    .to_owned(),
            )
            .await?;

        // Drop last synced at field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::LastSyncedAt)
                    .to_owned(),
            )
            .await?;

        // Drop repository path field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::RepositoryPath)
                    .to_owned(),
            )
            .await?;

        // Drop repository ID field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::RepositoryId)
                    .to_owned(),
            )
            .await?;

        // Drop checksum field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::Checksum)
                    .to_owned(),
            )
            .await?;

        // Drop file path field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::FilePath)
                    .to_owned(),
            )
            .await?;

        // Drop storage type field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::StorageType)
                    .to_owned(),
            )
            .await?;

        // Drop source type field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::SourceType)
                    .to_owned(),
            )
            .await?;

        // Drop source code field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::SourceCode)
                    .to_owned(),
            )
            .await?;

        // Note: SQLite doesn't support modifying column constraints.
        // When rolling back, the path column will remain as it was originally defined.

        // Drop new tables
        manager.drop_table(Table::drop().table(TaskVersions::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(TaskRepositories::Table).to_owned()).await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Tasks {
    Table,
    Id,
    SourceCode,
    SourceType,
    StorageType,
    FilePath,
    Checksum,
    RepositoryId,
    RepositoryPath,
    LastSyncedAt,
    SyncStatus,
    IsEditable,
    CreatedFrom,
    NeedsPush,
    SourceModifiedAt,
}

#[derive(Iden)]
enum TaskRepositories {
    Table,
    Id,
    Name,
    RepositoryType,
    Uri,
    Branch,
    AuthConfig,
    SyncEnabled,
    SyncIntervalMinutes,
    LastSyncAt,
    SyncStatus,
    SyncError,
    Priority,
    IsDefault,
    IsWritable,
    WatchPatterns,
    IgnorePatterns,
    PushOnChange,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum TaskVersions {
    Table,
    Id,
    TaskId,
    RepositoryId,
    Version,
    SourceCode,
    InputSchema,
    OutputSchema,
    Metadata,
    Checksum,
    ChangeDescription,
    ChangedBy,
    ChangeSource,
    RepositoryCommit,
    CreatedAt,
}