use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Task version history for tracking changes
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "task_versions")]
pub struct Model {
    /// Primary key
    #[sea_orm(primary_key)]
    pub id: i32,

    /// Foreign key to tasks table
    pub task_id: i32,

    /// Foreign key to task_repositories table
    pub repository_id: i32,

    /// Version string
    pub version: String,

    /// JavaScript source code at this version
    #[sea_orm(column_type = "Text")]
    pub source_code: String,

    /// Input schema at this version
    pub input_schema: Json,

    /// Output schema at this version
    pub output_schema: Json,

    /// Task metadata at this version
    pub metadata: Json,

    /// SHA256 checksum of source code
    pub checksum: String,

    /// Description of the change
    pub change_description: Option<String>,

    /// User/system that made the change
    pub changed_by: String,

    /// Source of the change: "api", "sync", "file", etc.
    pub change_source: String,

    /// Git commit hash if applicable
    pub repository_commit: Option<String>,

    /// When this version was created
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tasks::Entity",
        from = "Column::TaskId",
        to = "super::tasks::Column::Id"
    )]
    Task,

    #[sea_orm(
        belongs_to = "super::task_repositories::Entity",
        from = "Column::RepositoryId",
        to = "super::task_repositories::Column::Id"
    )]
    Repository,
}

impl Related<super::tasks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Task.def()
    }
}

impl Related<super::task_repositories::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Repository.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Calculate SHA256 checksum of source code
    pub fn calculate_checksum(source_code: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(source_code.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Create a new version from current task state
    pub fn from_task(
        task_id: i32,
        repository_id: i32,
        source_code: &str,
        input_schema: &serde_json::Value,
        output_schema: &serde_json::Value,
        metadata: &serde_json::Value,
        version: &str,
        changed_by: &str,
        change_source: &str,
        change_description: Option<String>,
        repository_commit: Option<String>,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            task_id,
            repository_id,
            version: version.to_string(),
            source_code: source_code.to_string(),
            input_schema: input_schema.clone(),
            output_schema: output_schema.clone(),
            metadata: metadata.clone(),
            checksum: Self::calculate_checksum(source_code),
            change_description,
            changed_by: changed_by.to_string(),
            change_source: change_source.to_string(),
            repository_commit,
            created_at: chrono::Utc::now(),
        }
    }
}