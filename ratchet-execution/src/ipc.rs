//! Inter-process communication types and transport for worker processes

// Re-export everything from ratchet-ipc for convenience
pub use ratchet_ipc::*;

// All types are re-exported from ratchet_ipc already

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_execution_context_creation() {
        let exec_uuid = Uuid::new_v4();
        let job_uuid = Some(Uuid::new_v4());
        let task_uuid = Uuid::new_v4();
        let version = "1.0.0".to_string();

        let context = ExecutionContext::new(exec_uuid, job_uuid, task_uuid, version.clone());

        assert_eq!(context.execution_id, exec_uuid.to_string());
        assert_eq!(context.job_id, job_uuid.map(|u| u.to_string()));
        assert_eq!(context.task_id, task_uuid.to_string());
        assert_eq!(context.task_version, version);
    }

    #[test]
    fn test_execution_context_serialization() {
        let context = ExecutionContext::new(
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            Uuid::new_v4(),
            "1.0.0".to_string(),
        );

        let json = serde_json::to_string(&context).unwrap();
        let deserialized: ExecutionContext = serde_json::from_str(&json).unwrap();

        assert_eq!(context.execution_id, deserialized.execution_id);
        assert_eq!(context.job_id, deserialized.job_id);
        assert_eq!(context.task_id, deserialized.task_id);
        assert_eq!(context.task_version, deserialized.task_version);
    }
}