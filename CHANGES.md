# Changes

## 0.2.0 (Unreleased)

Major server implementation with GraphQL API and task registry.

### Features

- Complete GraphQL API server with async-graphql
- Unified Task Registry system for centralized task management
  - Filesystem loader supporting directories, ZIP files, and collections
  - Version management with duplicate detection
  - Automatic synchronization between registry and database
  - Single GraphQL interface combining registry and database views
  - Reference-based storage eliminating data duplication
- **File System Watcher** for automatic task reloading
  - Cross-platform file monitoring (Linux inotify, macOS FSEvents, Windows ReadDirectoryChangesW)
  - Real-time task updates when files change
  - Configurable `watch: true|false` option per filesystem source
  - Event debouncing to handle rapid file changes efficiently
  - Graceful error handling without server crashes
  - Smart ignore patterns for temporary files
- Process separation architecture for thread-safe JavaScript execution
- Job queue system with priority and retry logic
- CLI `serve` command for easy server deployment
- Database persistence with Sea-ORM and SQLite
- Worker process management with IPC communication

### Improvements

- Refactored into modular architecture with clear separation of concerns
- Added comprehensive configuration system with YAML and environment support
- Implemented service layer abstraction for better testability

## 0.1.0 (2025-05-23)

Initial release of Ratchet, a JavaScript task execution framework.

### Features

- Execute JavaScript tasks with input/output schema validation
- Support for asynchronous operations with Tokio runtime
- HTTP fetch API for making web requests from JavaScript
- Lazy loading of JavaScript files with LRU caching
- CLI with JSON input support