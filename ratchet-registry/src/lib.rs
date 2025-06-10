pub mod cache;
pub mod config;
pub mod error;
pub mod loaders;
pub mod registry;
pub mod service;
pub mod sync;
pub mod types;
pub mod watcher;

// Re-export main types and traits
pub use config::{RegistryConfig, TaskSource, WatcherConfig};
pub use error::{RegistryError, Result};
pub use loaders::{filesystem::FilesystemLoader, http::HttpLoader, TaskLoader};
pub use registry::{DefaultTaskRegistry, TaskRegistry};
pub use service::{DefaultRegistryService, RegistryService};
pub use sync::{ConflictResolver, DatabaseSync};
pub use types::{
    DiscoveredTask, RegistryEvent, SyncResult, TaskDefinition, TaskMetadata, TaskReference,
    ValidationResult,
};
pub use watcher::RegistryWatcher;

// Re-export for backward compatibility if needed
pub mod prelude {
    pub use crate::{
        config::*, error::*, types::*, DefaultRegistryService,
        DefaultTaskRegistry, RegistryService, TaskRegistry,
    };
}
