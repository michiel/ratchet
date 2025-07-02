use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Task repository entity for managing task sources
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "task_repositories")]
pub struct Model {
    /// Primary key
    #[sea_orm(primary_key)]
    pub id: i32,

    /// Repository name
    #[sea_orm(unique)]
    pub name: String,

    /// Repository type: "filesystem", "git", "http", "registry"
    pub repository_type: String,

    /// Repository URI (path, URL, or identifier)
    pub uri: String,

    /// Git branch (for git repos)
    pub branch: Option<String>,

    /// Authentication configuration as JSON
    pub auth_config: Option<Json>,

    /// Whether sync is enabled
    pub sync_enabled: bool,

    /// Sync interval in minutes
    pub sync_interval_minutes: Option<i32>,

    /// Last sync timestamp
    pub last_sync_at: Option<ChronoDateTimeUtc>,

    /// Sync status: "success", "error", "pending"
    pub sync_status: String,

    /// Sync error message
    pub sync_error: Option<String>,

    /// Sync priority (higher = first)
    pub priority: i32,

    /// Whether this is the default repository for new tasks
    pub is_default: bool,

    /// Whether tasks can be pushed back to this repository
    pub is_writable: bool,

    /// File patterns to watch/sync as JSON array
    pub watch_patterns: Json,

    /// Patterns to ignore as JSON array
    pub ignore_patterns: Json,

    /// Auto-push changes to repository
    pub push_on_change: bool,

    /// Repository-specific metadata as JSON
    pub metadata: Json,

    /// When the repository was created
    pub created_at: ChronoDateTimeUtc,

    /// When the repository was last updated
    pub updated_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::tasks::Entity")]
    Tasks,

    #[sea_orm(has_many = "super::task_versions::Entity")]
    TaskVersions,
}

impl Related<super::tasks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tasks.def()
    }
}

impl Related<super::task_versions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TaskVersions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Create a default filesystem repository
    pub fn create_default_filesystem(base_path: &str) -> Self {
        Self {
            id: 0, // Will be set by database
            name: "default-filesystem".to_string(),
            repository_type: "filesystem".to_string(),
            uri: base_path.to_string(),
            branch: None,
            auth_config: None,
            sync_enabled: true,
            sync_interval_minutes: Some(5),
            last_sync_at: None,
            sync_status: "pending".to_string(),
            sync_error: None,
            priority: 1,
            is_default: true,
            is_writable: true,
            watch_patterns: serde_json::json!(["**/*.js", "**/task.yaml", "**/task.json"]),
            ignore_patterns: serde_json::json!(["**/node_modules/**", "**/.git/**", "**/target/**"]),
            push_on_change: false,
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Check if repository supports write operations
    pub fn can_write(&self) -> bool {
        self.is_writable && self.sync_enabled
    }

    /// Get watch patterns as Vec<String>
    pub fn get_watch_patterns(&self) -> Vec<String> {
        self.watch_patterns
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get ignore patterns as Vec<String>
    pub fn get_ignore_patterns(&self) -> Vec<String> {
        self.ignore_patterns
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }
}