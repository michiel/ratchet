//! Concrete implementations of output destinations

pub mod filesystem;
pub mod webhook;
pub mod stdio;

pub use filesystem::FilesystemDestination;
pub use webhook::WebhookDestination;
pub use stdio::{StdioDestination, StdioConfig, StdStream};