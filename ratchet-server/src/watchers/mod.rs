//! File system watching for repository monitoring

pub mod filesystem_watcher;

pub use filesystem_watcher::{FilesystemWatcher, FilesystemWatcherConfig, WatchEvent};