//! Example plugin implementations

pub mod logging_plugin;
pub mod metrics_plugin;
pub mod notification_plugin;

// Re-export example plugins
pub use logging_plugin::LoggingPlugin;
pub use metrics_plugin::MetricsPlugin;
pub use notification_plugin::NotificationPlugin;
