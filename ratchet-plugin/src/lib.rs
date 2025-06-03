//! Plugin system for Ratchet
//!
//! This crate provides a flexible plugin architecture that allows extending
//! Ratchet with custom functionality through dynamically loaded plugins.

pub mod core;
pub mod discovery;
pub mod error;
pub mod hooks;
pub mod loader;
pub mod manager;
pub mod registry;
pub mod types;

// Re-export main types
pub use core::{Plugin, PluginContext, PluginMetadata};
pub use error::{PluginError, PluginResult};
pub use hooks::{ExecutionHook, Hook, HookPriority, HookRegistry, TaskHook};
pub use loader::{PluginLoader, StaticPluginLoader, DynamicPluginLoader};
pub use manager::{PluginManager, PluginManagerBuilder};
pub use registry::{PluginRegistry, PluginInfo};
pub use types::{PluginType, PluginVersion, PluginDependency};

/// Plugin system version
pub const PLUGIN_SYSTEM_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Minimum supported plugin API version
pub const MIN_PLUGIN_API_VERSION: &str = "0.1.0";

/// Plugin discovery macros
pub mod macros {
    pub use inventory;
    
    /// Register a static plugin
    /// 
    /// # Example
    /// ```rust,ignore
    /// use ratchet_plugin::{Plugin, PluginMetadata, register_plugin};
    /// 
    /// struct MyPlugin;
    /// 
    /// impl Plugin for MyPlugin {
    ///     fn metadata(&self) -> &PluginMetadata {
    ///         // implementation
    ///     }
    /// }
    /// 
    /// register_plugin!(MyPlugin::new());
    /// ```
    #[macro_export]
    macro_rules! register_plugin {
        ($plugin:expr) => {
            $crate::macros::inventory::submit! {
                fn plugin() -> Box<dyn $crate::Plugin> {
                    Box::new($plugin)
                }
            }
        };
    }
}