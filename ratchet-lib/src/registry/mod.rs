// Registry functionality moved to ratchet-registry crate
// Re-export for backward compatibility

pub use ratchet_registry::{
    RegistryConfig, TaskSource,
};

// Re-export with original names for backward compatibility
pub use ratchet_registry::DefaultTaskRegistry as TaskRegistry;
pub use ratchet_registry::DefaultRegistryService;
pub use ratchet_registry::RegistryService;
pub use ratchet_registry::WatcherConfig;
pub use ratchet_registry::RegistryWatcher;

// Legacy module structure for backward compatibility
pub mod loaders {
    pub use ratchet_registry::loaders::*;
}

pub mod registry {
    pub use ratchet_registry::registry::*;
    pub use ratchet_registry::{TaskSource, DefaultTaskRegistry as TaskRegistry};
}

pub mod service {
    pub use ratchet_registry::service::*;
}

pub mod watcher {
    pub use ratchet_registry::watcher::*;
    pub use ratchet_registry::WatcherConfig;
}
