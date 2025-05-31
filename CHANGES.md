# Changes

## 0.2.0 (Unreleased)

Major server implementation with GraphQL API and task registry.

### Features

- Complete GraphQL API server with async-graphql
- **Refine.dev Compatible REST API** with comprehensive resource management
  - Full CRUD operations for Tasks, Jobs, Schedules, Executions, and Workers
  - Health monitoring endpoint for load balancers and monitoring systems
  - Pagination support with `_start`, `_end` parameters
  - Sorting and filtering with query parameter validation
  - CORS support for web application integration
  - OpenAPI 3.0.3 specification with complete documentation
  - Comprehensive integration test coverage (5 test suites)
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

### Improvements

- **Configuration System Enhancements**
  - Made all configuration fields optional with sensible defaults
  - Added support for partial configuration files
  - Improved empty configuration handling

### Infrastructure

- **CI/CD Improvements**
  - Added manual build workflow for on-demand builds via GitHub Actions
  - Support for configurable build profiles (debug/release)
  - Multi-platform build matrix (Linux x86_64/aarch64, Windows, macOS)
  - Automated artifact upload for all platforms

- **Static Build Support**
  - Replaced OpenSSL with rustls for pure Rust TLS implementation
  - SQLite bundled by default (no external dependencies)
  - Documented static build process for all platforms
  - Prepared for musl-based static Linux builds

### Bug Fixes

- **Fixed REST API query parameter deserialization** - Resolved 400 errors for pagination parameters like `?_start=0&_end=10`
  - Replaced problematic nested serde flatten directives with direct field mapping
  - Added proper query parameter validation to prevent malformed requests
  - Maintained backward compatibility through helper accessor methods
- **Resolved compilation warnings** - Prefixed unused fields and variables with underscores
- **Enhanced error handling** - Improved REST API error responses with detailed validation messages

## 0.1.0 (2025-05-23)

Initial release of Ratchet, a JavaScript task execution framework.

### Features

- Execute JavaScript tasks with input/output schema validation
- Support for asynchronous operations with Tokio runtime
- HTTP fetch API for making web requests from JavaScript
- Lazy loading of JavaScript files with LRU caching
- CLI with JSON input support