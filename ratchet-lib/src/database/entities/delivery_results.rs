use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Delivery result entity for tracking output delivery status
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "delivery_results")]
pub struct Model {
    /// Primary key
    #[sea_orm(primary_key)]
    pub id: i32,

    /// Foreign key to jobs table
    pub job_id: i32,

    /// Foreign key to executions table
    pub execution_id: i32,

    /// Type of destination (filesystem, webhook, etc.)
    pub destination_type: String,

    /// Unique identifier for the destination
    pub destination_id: String,

    /// Whether delivery was successful
    pub success: bool,

    /// Delivery time in milliseconds
    pub delivery_time_ms: i32,

    /// Size of delivered data in bytes
    pub size_bytes: i32,

    /// Response information from destination (HTTP response, file path, etc.)
    pub response_info: Option<String>,

    /// Error message if delivery failed
    pub error_message: Option<String>,

    /// When the delivery was attempted
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::jobs::Entity",
        from = "Column::JobId",
        to = "super::jobs::Column::Id"
    )]
    Job,

    #[sea_orm(
        belongs_to = "super::executions::Entity",
        from = "Column::ExecutionId",
        to = "super::executions::Column::Id"
    )]
    Execution,
}

impl Related<super::jobs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Job.def()
    }
}

impl Related<super::executions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Execution.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Create a new delivery result
    pub fn new(
        job_id: i32,
        execution_id: i32,
        destination_type: String,
        destination_id: String,
        success: bool,
        delivery_time_ms: i32,
        size_bytes: i32,
        response_info: Option<String>,
        error_message: Option<String>,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            job_id,
            execution_id,
            destination_type,
            destination_id,
            success,
            delivery_time_ms,
            size_bytes,
            response_info,
            error_message,
            created_at: chrono::Utc::now(),
        }
    }
}
