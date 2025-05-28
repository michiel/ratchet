pub mod loaders;
pub mod registry;
pub mod service;

pub use registry::{TaskRegistry, TaskSource};
pub use service::{RegistryService, DefaultRegistryService};