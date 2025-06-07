use super::LogEvent;
use serde_json::json;
use sysinfo::{ProcessExt, System, SystemExt};

/// Trait for log enrichment
pub trait Enricher: Send + Sync {
    /// Enrich a log event with additional context
    fn enrich(&self, event: &mut LogEvent);
}

/// Container for multiple enrichers
pub struct LogEnricher {
    enrichers: Vec<Box<dyn Enricher>>,
}

impl LogEnricher {
    pub fn new(enrichers: Vec<Box<dyn Enricher>>) -> Self {
        Self { enrichers }
    }

    pub fn enrich(&self, event: &mut LogEvent) {
        for enricher in &self.enrichers {
            enricher.enrich(event);
        }
    }
}

impl Default for LogEnricher {
    fn default() -> Self {
        Self::new(vec![
            Box::new(SystemEnricher::new()),
            Box::new(ProcessEnricher::new()),
        ])
    }
}

/// Enricher that adds system information
pub struct SystemEnricher {
    hostname: String,
}

impl Default for SystemEnricher {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemEnricher {
    pub fn new() -> Self {
        Self {
            hostname: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
        }
    }
}

impl Enricher for SystemEnricher {
    fn enrich(&self, event: &mut LogEvent) {
        event
            .fields
            .insert("hostname".to_string(), json!(self.hostname));
        event
            .fields
            .insert("os".to_string(), json!(std::env::consts::OS));
        event
            .fields
            .insert("arch".to_string(), json!(std::env::consts::ARCH));
    }
}

/// Enricher that adds process information
pub struct ProcessEnricher {
    process_id: u32,
    process_name: String,
}

impl Default for ProcessEnricher {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessEnricher {
    pub fn new() -> Self {
        let process_id = std::process::id();
        let process_name = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        Self {
            process_id,
            process_name,
        }
    }
}

impl Enricher for ProcessEnricher {
    fn enrich(&self, event: &mut LogEvent) {
        event
            .fields
            .insert("process_id".to_string(), json!(self.process_id));
        event
            .fields
            .insert("process_name".to_string(), json!(self.process_name));

        // Add memory usage if available
        let mut system = System::new_all();
        system.refresh_process(sysinfo::Pid::from(self.process_id as usize));
        if let Some(process) = system.process(sysinfo::Pid::from(self.process_id as usize)) {
            event.fields.insert(
                "memory_usage_mb".to_string(),
                json!(process.memory() / 1024 / 1024),
            );
            event
                .fields
                .insert("cpu_usage_percent".to_string(), json!(process.cpu_usage()));
        }
    }
}

/// Enricher that adds task context
pub struct TaskContextEnricher;

impl Default for TaskContextEnricher {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskContextEnricher {
    pub fn new() -> Self {
        Self
    }
}

impl Enricher for TaskContextEnricher {
    fn enrich(&self, event: &mut LogEvent) {
        // Note: Task context enrichment would require access to task registry
        // For now, we just ensure task-related fields are present
        if event.fields.contains_key("task_id") {
            // Add a marker that this event is task-related
            event
                .fields
                .insert("context_type".to_string(), json!("task_execution"));
        }
    }
}

/// Enricher that adds execution context
pub struct ExecutionContextEnricher;

impl Default for ExecutionContextEnricher {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionContextEnricher {
    pub fn new() -> Self {
        Self
    }
}

impl Enricher for ExecutionContextEnricher {
    fn enrich(&self, event: &mut LogEvent) {
        // Add execution-specific context if available
        if event.fields.contains_key("execution_id") {
            event.fields.insert(
                "execution_phase".to_string(),
                json!(event.fields.get("phase").unwrap_or(&json!("unknown"))),
            );
        }

        // Add timing information if this is an execution completion
        if let Some(started_at) = event.fields.get("started_at") {
            if let Some(completed_at) = event.fields.get("completed_at") {
                // Calculate duration if both timestamps are available
                if let (Some(start), Some(end)) = (
                    started_at
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()),
                    completed_at
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()),
                ) {
                    let duration_ms = (end.timestamp_millis() - start.timestamp_millis()).max(0);
                    event
                        .fields
                        .insert("duration_ms".to_string(), json!(duration_ms));
                }
            }
        }
    }
}
