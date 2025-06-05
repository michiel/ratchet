//! Example plugins for Ratchet
//!
//! This crate contains example plugin implementations that demonstrate
//! how to extend Ratchet with custom functionality.

pub mod examples;

// Re-export common plugin types for convenience
pub use ratchet_plugin::{
    Plugin, PluginContext, PluginMetadata, PluginResult, PluginError,
    Hook, TaskHook, ExecutionHook, HookPriority,
    PluginManager, PluginRegistry, PluginType, PluginVersion,
};