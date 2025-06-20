//! Example logging plugin that demonstrates basic plugin functionality

use async_trait::async_trait;
use ratchet_plugin::types::PluginStatus;
use ratchet_plugin::*;
use std::any::Any;
use tracing::info;

/// Simple logging plugin that logs plugin lifecycle events
pub struct LoggingPlugin {
    metadata: PluginMetadata,
}

impl LoggingPlugin {
    /// Create a new logging plugin
    pub fn new() -> Self {
        let metadata = PluginMetadata::new(
            "ratchet.plugins.logging",
            "Task Execution Logger",
            PluginVersion::new(1, 0, 0),
            "Logs plugin and task execution events for monitoring",
            "Ratchet Team",
            PluginType::Logging,
        );

        Self { metadata }
    }
}

impl Default for LoggingPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for LoggingPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, context: &mut PluginContext) -> PluginResult<()> {
        info!("🔧 Initializing Logging Plugin v{}", self.metadata.version);
        info!("📋 Plugin will log system events and task executions");

        // Set status to active (normally done by parent)
        context.set_status(PluginStatus::Active);

        info!("✅ Logging Plugin initialized successfully");
        Ok(())
    }

    async fn execute(&mut self, _context: &mut PluginContext) -> PluginResult<serde_json::Value> {
        info!("🚀 Logging Plugin execute called");

        // In a real plugin, this might:
        // - Set up log forwarding
        // - Configure logging destinations
        // - Register event listeners

        // For this example, just return some status information
        let result = serde_json::json!({
            "status": "active",
            "logs_processed": 42,
            "log_level": "info",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        info!("📊 Logging Plugin execution completed with result: {}", result);
        Ok(result)
    }

    async fn shutdown(&mut self, context: &mut PluginContext) -> PluginResult<()> {
        info!("🛑 Shutting down Logging Plugin");

        // Set status to unloaded (normally done by parent)
        context.set_status(PluginStatus::Unloaded);

        info!("✅ Logging Plugin shutdown complete");
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
// To use this plugin, register manually: plugin_manager.register(Box::new(LoggingPlugin::new()))

#[cfg(test)]
mod tests {
    use super::*;
    use ratchet_config::RatchetConfig;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_logging_plugin_creation() {
        let plugin = LoggingPlugin::new();
        assert_eq!(plugin.metadata().id, "ratchet.plugins.logging");
        assert_eq!(plugin.metadata().name, "Task Execution Logger");
        assert_eq!(plugin.metadata().plugin_type, PluginType::Logging);
    }

    #[tokio::test]
    async fn test_logging_plugin_lifecycle() {
        let mut plugin = LoggingPlugin::new();
        let mut context = PluginContext::new(Uuid::new_v4(), serde_json::json!({}), RatchetConfig::default());

        // Test initialization
        assert!(plugin.initialize(&mut context).await.is_ok());

        // Test execution
        let result = plugin.execute(&mut context).await.unwrap();
        assert!(result.is_object());
        assert_eq!(result["status"], "active");

        // Test shutdown
        assert!(plugin.shutdown(&mut context).await.is_ok());
    }
}
