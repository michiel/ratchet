//! Example plugins for Ratchet
//!
//! This crate contains example plugin implementations that demonstrate
//! how to extend Ratchet with custom functionality.

pub mod examples;

// Re-export common plugin types for convenience
pub use ratchet_plugin::{
    ExecutionHook, Hook, HookPriority, Plugin, PluginContext, PluginError, PluginManager,
    PluginMetadata, PluginRegistry, PluginResult, PluginType, PluginVersion, TaskHook,
};
