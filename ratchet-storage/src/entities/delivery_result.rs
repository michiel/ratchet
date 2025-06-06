//! Delivery result entity definition

use super::Entity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Delivery result entity for tracking output delivery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryResult {
    pub id: i32,
    pub uuid: Uuid,
    pub job_id: i32,
    pub execution_id: i32,
    pub destination_type: String,
    pub destination_id: String,
    pub success: bool,
    pub delivery_time_ms: i32,
    pub size_bytes: Option<i32>,
    pub response_info: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for DeliveryResult {
    fn id(&self) -> i32 {
        self.id
    }
    fn uuid(&self) -> Uuid {
        self.uuid
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

impl DeliveryResult {
    pub fn new(
        job_id: i32,
        execution_id: i32,
        destination_type: String,
        destination_id: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            uuid: Uuid::new_v4(),
            job_id,
            execution_id,
            destination_type,
            destination_id,
            success: false,
            delivery_time_ms: 0,
            size_bytes: None,
            response_info: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }
}
