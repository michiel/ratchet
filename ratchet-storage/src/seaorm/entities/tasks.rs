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
    /// Create a new task from ratchet-core Task
    pub fn from_ratchet_task(task: &ratchet_core::task::Task) -> Self {
        // Extract path from task source
        let path = match &task.source {
            ratchet_core::task::TaskSource::File { path } => path.clone(),
            ratchet_core::task::TaskSource::Url { url, .. } => url.clone(),
            ratchet_core::task::TaskSource::JavaScript { .. } => "javascript:inline".to_string(),
            ratchet_core::task::TaskSource::Plugin { plugin_id, task_name } => {
                format!("plugin://{}:{}", plugin_id, task_name)
            }
        };

        Self {
            id: 0, // Will be set by database
            uuid: task.metadata.id.0, // TaskId contains the UUID
            name: task.metadata.name.clone(),
            description: task.metadata.description.clone(),
            version: task.metadata.version.clone(),
            path,
            metadata: serde_json::json!({
                "id": task.metadata.id.0,
                "name": task.metadata.name,
                "version": task.metadata.version,
                "description": task.metadata.description,
                "author": task.metadata.author,
                "tags": task.metadata.tags,
                "deprecated": task.metadata.deprecated,
                "deprecation_message": task.metadata.deprecation_message,
                "documentation": task.metadata.documentation,
            }),
            input_schema: task.input_schema.clone(),
            output_schema: task.output_schema.clone(),
            enabled: task.enabled,
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
        }
    }
}