use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Task entity representing a JavaScript task definition
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    /// Primary key
    #[sea_orm(primary_key)]
    pub id: i32,

    /// Unique identifier for the task
    #[sea_orm(unique)]
    pub uuid: Uuid,

    /// Task name/label
    pub name: String,

    /// Task description
    pub description: Option<String>,

    /// Task version
    pub version: String,

    /// Path to task files (directory or ZIP)
    pub path: String,

    /// Task metadata as JSON
    pub metadata: Json,

    /// Input schema as JSON
    pub input_schema: Json,

    /// Output schema as JSON  
    pub output_schema: Json,

    /// Whether the task is enabled for execution
    pub enabled: bool,

    /// When the task was created
    pub created_at: ChronoDateTimeUtc,

    /// When the task was last updated
    pub updated_at: ChronoDateTimeUtc,

    /// When the task was last validated
    pub validated_at: Option<ChronoDateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::executions::Entity")]
    Executions,

    #[sea_orm(has_many = "super::schedules::Entity")]
    Schedules,

    #[sea_orm(has_many = "super::jobs::Entity")]
    Jobs,
}

impl Related<super::executions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Executions.def()
    }
}

impl Related<super::schedules::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Schedules.def()
    }
}

impl Related<super::jobs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Jobs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Create a new task from ratchet-lib Task
    pub fn from_ratchet_task(task: &crate::task::Task) -> Self {
        Self {
            id: 0, // Will be set by database
            uuid: task.uuid(),
            name: task.metadata.label.clone(),
            description: Some(task.metadata.description.clone()),
            version: task.metadata.version.clone(),
            path: task.path.to_string_lossy().to_string(),
            metadata: serde_json::json!({
                "uuid": task.metadata.uuid,
                "version": task.metadata.version,
                "label": task.metadata.label,
                "description": task.metadata.description,
            }),
            input_schema: task.input_schema.clone(),
            output_schema: task.output_schema.clone(),
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            validated_at: None,
        }
    }
}
