//! Example metrics plugin that collects basic statistics

use async_trait::async_trait;
use ratchet_plugin::*;
use std::any::Any;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::info;

/// Simple metrics plugin that demonstrates metric collection
pub struct MetricsPlugin {
    metadata: PluginMetadata,
    execution_count: Arc<AtomicU64>,
}

impl MetricsPlugin {
    /// Create a new metrics plugin
    pub fn new() -> Self {
        let metadata = PluginMetadata::new(
            "ratchet.plugins.metrics",
            "Task Execution Metrics",
            PluginVersion::new(1, 0, 0),
            "Collects basic metrics and statistics about plugin executions",
            "Ratchet Team",
            PluginType::Monitoring,
        );

        Self {
            metadata,
            execution_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get current execution count
    pub fn get_execution_count(&self) -> u64 {
        self.execution_count.load(Ordering::Relaxed)
    }
}

impl Default for MetricsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for MetricsPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, context: &mut PluginContext) -> PluginResult<()> {
        info!("📊 Initializing Metrics Plugin v{}", self.metadata.version);
        info!("📈 Plugin will collect execution statistics");

        // Call parent initialization
        Plugin::initialize(self, context).await?;

        info!("✅ Metrics Plugin initialized successfully");
        Ok(())
    }

    async fn execute(&mut self, _context: &mut PluginContext) -> PluginResult<serde_json::Value> {
        info!("📊 Metrics Plugin execute called");

        // Increment execution counter
        let count = self.execution_count.fetch_add(1, Ordering::Relaxed) + 1;

        // In a real plugin, this might:
        // - Collect system metrics
        // - Track performance data
        // - Send metrics to monitoring systems

        let result = serde_json::json!({
            "status": "collecting",
            "execution_count": count,
            "plugin_id": self.metadata.id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "metrics": {
                "total_executions": count,
                "uptime_seconds": 3600, // Mock data
                "memory_usage_mb": 128   // Mock data
            }
        });

        info!("📈 Metrics collected: execution #{}, total: {}", count, count);
        Ok(result)
    }

    async fn shutdown(&mut self, context: &mut PluginContext) -> PluginResult<()> {
        let final_count = self.execution_count.load(Ordering::Relaxed);
        info!("📊 Shutting down Metrics Plugin (final count: {})", final_count);

        // Call parent shutdown
        Plugin::shutdown(self, context).await?;

        info!("✅ Metrics Plugin shutdown complete");
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Auto-registration temporarily disabled due to macro issues
// To use this plugin, register manually: plugin_manager.register(Box::new(MetricsPlugin::new()))

#[cfg(test)]
mod tests {
    use super::*;
    use ratchet_config::RatchetConfig;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_metrics_plugin_creation() {
        let plugin = MetricsPlugin::new();
        assert_eq!(plugin.metadata().id, "ratchet.plugins.metrics");
        assert_eq!(plugin.metadata().name, "Task Execution Metrics");
        assert_eq!(plugin.metadata().plugin_type, PluginType::Monitoring);
        assert_eq!(plugin.get_execution_count(), 0);
    }

    #[tokio::test]
    async fn test_metrics_plugin_execution() {
        let mut plugin = MetricsPlugin::new();
        let mut context = PluginContext::new(Uuid::new_v4(), serde_json::json!({}), RatchetConfig::default());

        // Initial count should be 0
        assert_eq!(plugin.get_execution_count(), 0);

        // Execute plugin
        let result = plugin.execute(&mut context).await.unwrap();
        assert!(result.is_object());
        assert_eq!(result["execution_count"], 1);
        assert_eq!(plugin.get_execution_count(), 1);

        // Execute again
        let result2 = plugin.execute(&mut context).await.unwrap();
        assert_eq!(result2["execution_count"], 2);
        assert_eq!(plugin.get_execution_count(), 2);
    }
}
