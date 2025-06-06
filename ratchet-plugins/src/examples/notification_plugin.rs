//! Example notification plugin that simulates sending alerts

use async_trait::async_trait;
use ratchet_plugin::types::PluginStatus;
use ratchet_plugin::*;
use std::any::Any;
use tracing::{info, warn};

/// Simple notification plugin that demonstrates alert functionality
pub struct NotificationPlugin {
    metadata: PluginMetadata,
    webhook_url: Option<String>,
}

impl NotificationPlugin {
    /// Create a new notification plugin
    pub fn new() -> Self {
        let metadata = PluginMetadata::new(
            "ratchet.plugins.notifications",
            "Task Execution Notifications",
            PluginVersion::new(1, 0, 0),
            "Sends notifications and alerts for important events",
            "Ratchet Team",
            PluginType::Custom("notification".to_string()),
        );

        Self {
            metadata,
            webhook_url: None,
        }
    }

    /// Create a new notification plugin with a webhook URL
    pub fn with_webhook(webhook_url: impl Into<String>) -> Self {
        let mut plugin = Self::new();
        plugin.webhook_url = Some(webhook_url.into());
        plugin
    }

    /// Send a notification (simulated)
    async fn send_notification(&self, title: &str, message: &str, level: &str) {
        if let Some(webhook_url) = &self.webhook_url {
            info!(
                "📤 Sending webhook to {}: {} - {}",
                webhook_url, title, message
            );
            // In a real implementation, you would use reqwest to send HTTP request
        } else {
            match level {
                "error" => warn!("🚨 [ALERT] {}: {}", title, message),
                "warning" => warn!("⚠️  [WARNING] {}: {}", title, message),
                _ => info!("ℹ️  [INFO] {}: {}", title, message),
            }
        }
    }
}

impl Default for NotificationPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for NotificationPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, context: &mut PluginContext) -> PluginResult<()> {
        info!(
            "📬 Initializing Notification Plugin v{}",
            self.metadata.version
        );

        if self.webhook_url.is_some() {
            info!("🌐 Webhook URL configured: notifications will be sent via HTTP");
        } else {
            info!("📋 No webhook URL: notifications will be logged only");
        }

        // Set status to active (normally done by parent)
        context.set_status(PluginStatus::Active);

        info!("✅ Notification Plugin initialized successfully");
        Ok(())
    }

    async fn execute(&mut self, _context: &mut PluginContext) -> PluginResult<serde_json::Value> {
        info!("📬 Notification Plugin execute called");

        // Simulate sending some notifications
        self.send_notification(
            "Plugin Execution",
            "Notification plugin executed successfully",
            "info",
        )
        .await;

        // In a real plugin, this might:
        // - Check for pending alerts
        // - Process notification queue
        // - Send scheduled notifications

        let result = serde_json::json!({
            "status": "active",
            "webhook_configured": self.webhook_url.is_some(),
            "webhook_url": self.webhook_url,
            "notifications_sent": 1,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        info!("📬 Notification Plugin execution completed");
        Ok(result)
    }

    async fn shutdown(&mut self, context: &mut PluginContext) -> PluginResult<()> {
        info!("📬 Shutting down Notification Plugin");

        // Send shutdown notification
        self.send_notification(
            "Plugin Shutdown",
            "Notification plugin is shutting down",
            "info",
        )
        .await;

        // Set status to unloaded (normally done by parent)
        context.set_status(PluginStatus::Unloaded);

        info!("✅ Notification Plugin shutdown complete");
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
// To use this plugin, register manually: plugin_manager.register(Box::new(NotificationPlugin::new()))

#[cfg(test)]
mod tests {
    use super::*;
    use ratchet_config::RatchetConfig;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_notification_plugin_creation() {
        let plugin = NotificationPlugin::new();
        assert_eq!(plugin.metadata().id, "ratchet.plugins.notifications");
        assert_eq!(plugin.metadata().name, "Task Execution Notifications");
        assert!(plugin.webhook_url.is_none());
    }

    #[tokio::test]
    async fn test_notification_plugin_with_webhook() {
        let webhook_url = "https://example.com/webhook";
        let plugin = NotificationPlugin::with_webhook(webhook_url);
        assert_eq!(plugin.webhook_url.as_ref().unwrap(), webhook_url);
    }

    #[tokio::test]
    async fn test_notification_plugin_lifecycle() {
        let mut plugin = NotificationPlugin::new();
        let mut context = PluginContext::new(
            Uuid::new_v4(),
            serde_json::json!({}),
            RatchetConfig::default(),
        );

        // Test initialization
        assert!(plugin.initialize(&mut context).await.is_ok());

        // Test execution
        let result = plugin.execute(&mut context).await.unwrap();
        assert!(result.is_object());
        assert_eq!(result["status"], "active");
        assert_eq!(result["webhook_configured"], false);

        // Test shutdown
        assert!(plugin.shutdown(&mut context).await.is_ok());
    }
}
