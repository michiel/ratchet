//! Concrete implementations of output destinations

pub mod filesystem;
pub mod stdio;
pub mod webhook;

pub use filesystem::FilesystemDestination;
pub use stdio::{StdStream, StdioConfig, StdioDestination};
pub use webhook::WebhookDestination;
