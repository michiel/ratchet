//! Concrete implementations of output destinations

pub mod filesystem;
pub mod webhook;

pub use filesystem::FilesystemDestination;
pub use webhook::WebhookDestination;
