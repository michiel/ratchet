//! Embedded tasks module
//!
//! This module contains built-in tasks that are embedded directly in the Ratchet binary.
//! These tasks are always available and cannot be modified or deleted by users.

use std::collections::HashMap;

/// Embedded task data structure
#[derive(Debug, Clone)]
pub struct EmbeddedTask {
    pub name: String,
    pub metadata: &'static str,
    pub input_schema: &'static str,
    pub output_schema: &'static str,
    pub main_js: &'static str,
}

/// Heartbeat task embedded content
pub mod heartbeat {
    use super::EmbeddedTask;
    
    /// Heartbeat task metadata
    pub const METADATA: &str = include_str!("heartbeat/metadata.json");
    
    /// Heartbeat task input schema
    pub const INPUT_SCHEMA: &str = include_str!("heartbeat/input.schema.json");
    
    /// Heartbeat task output schema
    pub const OUTPUT_SCHEMA: &str = include_str!("heartbeat/output.schema.json");
    
    /// Heartbeat task main JavaScript implementation
    pub const MAIN_JS: &str = include_str!("heartbeat/main.js");
    
    /// Create embedded heartbeat task
    pub fn create_task() -> EmbeddedTask {
        EmbeddedTask {
            name: "heartbeat".to_string(),
            metadata: METADATA,
            input_schema: INPUT_SCHEMA,
            output_schema: OUTPUT_SCHEMA,
            main_js: MAIN_JS,
        }
    }
}

/// Registry of all embedded tasks
pub struct EmbeddedTaskRegistry {
    tasks: HashMap<String, EmbeddedTask>,
}

impl EmbeddedTaskRegistry {
    /// Create a new embedded task registry with all built-in tasks
    pub fn new() -> Self {
        let mut tasks = HashMap::new();
        
        // Register heartbeat task
        let heartbeat_task = heartbeat::create_task();
        tasks.insert(heartbeat_task.name.clone(), heartbeat_task);
        
        Self { tasks }
    }
    
    /// Get all embedded task names
    pub fn task_names(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }
    
    /// Get an embedded task by name
    pub fn get_task(&self, name: &str) -> Option<&EmbeddedTask> {
        self.tasks.get(name)
    }
    
    /// Get all embedded tasks
    pub fn get_all_tasks(&self) -> Vec<&EmbeddedTask> {
        self.tasks.values().collect()
    }
    
    /// Check if a task is embedded
    pub fn contains_task(&self, name: &str) -> bool {
        self.tasks.contains_key(name)
    }
}

impl Default for EmbeddedTaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_embedded_registry_creation() {
        let registry = EmbeddedTaskRegistry::new();
        assert!(registry.contains_task("heartbeat"));
        assert_eq!(registry.task_names().len(), 1);
    }
    
    #[test]
    fn test_heartbeat_task_creation() {
        let task = heartbeat::create_task();
        assert_eq!(task.name, "heartbeat");
        assert!(!task.metadata.is_empty());
        assert!(!task.input_schema.is_empty());
        assert!(!task.output_schema.is_empty());
        assert!(!task.main_js.is_empty());
    }
    
    #[test]
    fn test_heartbeat_metadata_is_valid_json() {
        let metadata = heartbeat::METADATA;
        serde_json::from_str::<serde_json::Value>(metadata)
            .expect("Heartbeat metadata should be valid JSON");
    }
    
    #[test]
    fn test_heartbeat_schemas_are_valid_json() {
        let input_schema = heartbeat::INPUT_SCHEMA;
        let output_schema = heartbeat::OUTPUT_SCHEMA;
        
        serde_json::from_str::<serde_json::Value>(input_schema)
            .expect("Heartbeat input schema should be valid JSON");
        serde_json::from_str::<serde_json::Value>(output_schema)
            .expect("Heartbeat output schema should be valid JSON");
    }
}