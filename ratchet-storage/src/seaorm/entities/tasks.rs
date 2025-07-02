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

    /// Path to task files (directory or ZIP) - legacy field, now optional
    pub path: Option<String>,

    /// Task metadata as JSON
    pub metadata: Json,

    /// Input schema as JSON
    pub input_schema: Json,

    /// Output schema as JSON  
    pub output_schema: Json,

    /// Whether the task is enabled for execution
    pub enabled: bool,

    // New fields for full task storage
    /// JavaScript source code
    #[sea_orm(column_type = "Text")]
    pub source_code: String,

    /// Source type: "javascript", "typescript", etc.
    pub source_type: String,

    /// Storage type: "database", "file", "registry"
    pub storage_type: String,

    /// Original file path if applicable
    pub file_path: Option<String>,

    /// SHA256 checksum of source code
    pub checksum: String,

    /// Required reference to source repository
    pub repository_id: i32,

    /// Path within repository (for sync back)
    pub repository_path: String,

    /// Last sync timestamp
    pub last_synced_at: Option<ChronoDateTimeUtc>,

    /// Sync status: "synced", "modified", "conflict", "pending_push"
    pub sync_status: String,

    /// Whether task can be edited via API
    pub is_editable: bool,

    /// Source of creation: "pull", "api", "import"
    pub created_from: String,

    /// Whether changes need to be pushed to repository
    pub needs_push: bool,

    /// When the task was created
    pub created_at: ChronoDateTimeUtc,

    /// When the task was last updated
    pub updated_at: ChronoDateTimeUtc,

    /// Source file modification time
    pub source_modified_at: Option<ChronoDateTimeUtc>,

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

    #[sea_orm(
        belongs_to = "super::task_repositories::Entity",
        from = "Column::RepositoryId",
        to = "super::task_repositories::Column::Id"
    )]
    Repository,

    #[sea_orm(has_many = "super::task_versions::Entity")]
    TaskVersions,
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

impl Related<super::task_repositories::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Repository.def()
    }
}

impl Related<super::task_versions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TaskVersions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Create a new task from ratchet-core Task with repository assignment
    pub fn from_ratchet_task(task: &ratchet_core::task::Task, repository_id: i32, repository_path: &str) -> Self {
        use sha2::{Digest, Sha256};

        // Extract path from task source
        let (path, source_code, file_path) = match &task.source {
            ratchet_core::task::TaskSource::File { path } => {
                (Some(path.clone()), "// Source code will be loaded from file".to_string(), Some(path.clone()))
            },
            ratchet_core::task::TaskSource::Url { url, .. } => {
                (Some(url.clone()), "// Source code will be loaded from URL".to_string(), None)
            },
            ratchet_core::task::TaskSource::JavaScript { code, .. } => {
                (None, code.clone(), None)
            },
            ratchet_core::task::TaskSource::Plugin { plugin_id, task_name } => {
                let plugin_path = format!("plugin://{}:{}", plugin_id, task_name);
                (Some(plugin_path), "// Plugin-based task".to_string(), None)
            }
        };

        // Calculate checksum
        let mut hasher = Sha256::new();
        hasher.update(source_code.as_bytes());
        let checksum = format!("{:x}", hasher.finalize());

        Self {
            id: 0,                    // Will be set by database
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
            // New fields
            source_code,
            source_type: "javascript".to_string(),
            storage_type: "database".to_string(),
            file_path,
            checksum,
            repository_id,
            repository_path: repository_path.to_string(),
            last_synced_at: None,
            sync_status: "synced".to_string(),
            is_editable: true,
            created_from: "import".to_string(),
            needs_push: false,
            created_at: task.created_at,
            updated_at: task.updated_at,
            source_modified_at: None,
            validated_at: task.validated_at,
        }
    }

    /// Create a new task from API request
    pub fn from_api_request(
        name: String,
        description: Option<String>,
        version: String,
        source_code: String,
        source_type: Option<String>,
        input_schema: serde_json::Value,
        output_schema: serde_json::Value,
        metadata: Option<serde_json::Value>,
        repository_id: i32,
        repository_path: String,
    ) -> Self {
        use sha2::{Digest, Sha256};
        use uuid::Uuid;

        // Calculate checksum
        let mut hasher = Sha256::new();
        hasher.update(source_code.as_bytes());
        let checksum = format!("{:x}", hasher.finalize());

        let now = chrono::Utc::now();
        let uuid = Uuid::new_v4();

        Self {
            id: 0, // Will be set by database
            uuid,
            name: name.clone(),
            description,
            version: version.clone(),
            path: None,
            metadata: metadata.unwrap_or_else(|| serde_json::json!({
                "id": uuid,
                "name": name,
                "version": version,
            })),
            input_schema,
            output_schema,
            enabled: true,
            source_code,
            source_type: source_type.unwrap_or_else(|| "javascript".to_string()),
            storage_type: "database".to_string(),
            file_path: None,
            checksum,
            repository_id,
            repository_path,
            last_synced_at: None,
            sync_status: "modified".to_string(),
            is_editable: true,
            created_from: "api".to_string(),
            needs_push: false, // Will be set based on repository configuration
            created_at: now,
            updated_at: now,
            source_modified_at: Some(now),
            validated_at: None,
        }
    }

    /// Calculate SHA256 checksum of source code
    pub fn calculate_checksum(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.source_code.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Update source code and recalculate checksum
    pub fn update_source_code(&mut self, source_code: String) {
        self.source_code = source_code;
        self.checksum = self.calculate_checksum();
        self.updated_at = chrono::Utc::now();
        self.source_modified_at = Some(self.updated_at);
        self.sync_status = "modified".to_string();
        self.needs_push = true;
    }

    /// Mark task as synced with repository
    pub fn mark_synced(&mut self, repository_commit: Option<String>) {
        self.sync_status = "synced".to_string();
        self.needs_push = false;
        self.last_synced_at = Some(chrono::Utc::now());
        // Store commit hash in metadata if provided
        if let Some(commit) = repository_commit {
            if let Some(metadata_obj) = self.metadata.as_object_mut() {
                metadata_obj.insert("last_commit".to_string(), serde_json::Value::String(commit));
            }
        }
    }

    /// Mark task as having sync conflict
    pub fn mark_conflict(&mut self, error_message: &str) {
        self.sync_status = "conflict".to_string();
        // Store error in metadata
        if let Some(metadata_obj) = self.metadata.as_object_mut() {
            metadata_obj.insert("sync_error".to_string(), serde_json::Value::String(error_message.to_string()));
        }
    }
}
