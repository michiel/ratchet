//! Command-line tools and utilities for Ratchet task management
//!
//! This crate provides essential command-line functionality for the Ratchet task automation
//! system, including task template generation, project scaffolding, and development utilities.

pub mod generate;
pub mod js_execution;
pub mod recording;

// Re-export commonly used types for convenience
pub use generate::{
    TaskGenerationConfig, 
    GeneratedTaskInfo, 
    generate_task
};

pub use js_execution::{
    execute_task_with_lib_compatibility,
    execute_task,
    ExecutionMode,
    TaskInput,
};

pub use recording::{
    set_recording_dir,
    get_recording_dir,
    is_recording,
    finalize_recording,
};