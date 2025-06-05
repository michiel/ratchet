# Changes

## 0.3.0 (Unreleased)

Major MCP (Model Context Protocol) server implementation enabling AI assistant integration.

### Features

- **Complete MCP Server Implementation** with production-ready functionality
  - Full JSON-RPC 2.0 protocol support with MCP extensions
  - 6 fully implemented tools replacing all placeholder implementations:
    - `ratchet.execute_task` - Execute tasks with streaming progress support
    - `ratchet.list_available_tasks` - Discover available tasks with metadata
    - `ratchet.get_execution_status` - Real-time execution status monitoring
    - `ratchet.get_execution_logs` - Comprehensive log retrieval with search
    - `ratchet.analyze_execution_error` - Intelligent error analysis with suggestions
    - `ratchet.get_execution_trace` - Detailed execution traces for debugging
  - **Dual Transport Support**:
    - stdio transport for CLI and local integrations
    - SSE (Server-Sent Events) transport for HTTP-based clients with CORS
  - **Streaming Progress Notifications** for long-running tasks
    - Real-time progress updates with configurable filtering
    - Step-based progress tracking with custom data
    - Frequency and delta-based filtering options
  - **High-Performance Batch Processing**
    - Parallel, sequential, and dependency-based execution modes
    - Request deduplication and result caching
    - Comprehensive error handling and timeout management
  - **Enterprise Configuration System**
    - Multiple authentication methods (API key, JWT, OAuth2)
    - Granular security controls and rate limiting
    - Connection pooling and performance optimization
    - Audit logging with external destinations
  - **Claude Desktop Integration** with ready-to-use configurations

- **Enhanced Logging System** with hybrid tracing support
  - Configuration-based logging initialization
  - Multiple sinks (console, file) with rotation and buffering
  - Structured logging with enrichment and sampling
  - Fallback to simple tracing for compatibility
  - Special handling for MCP stdio mode to preserve protocol integrity

### Improvements

- **Build System Fixes**
  - Resolved tokio runtime panic during shutdown
  - Fixed MCP test failures related to protocol version and stdio handling
  - Updated dependencies for better compatibility
  - Cleaned up unused imports and warnings

- **Task Execution Enhancements**
  - Fixed Task API usage (migrated from Task::execute to execute_task function)
  - Added HttpManager integration for task execution context
  - Improved error handling and mutable borrow patterns
  - Enhanced worker process communication

- **Documentation Updates**
  - Comprehensive MCP configuration guide with examples
  - SSE implementation documentation
  - Streaming example task with progress reporting
  - Updated TODO.md to reflect project milestones

### Infrastructure

- **MCP CLI Integration**
  - Added `mcp-serve` command for starting MCP server
  - Added `config validate` command for configuration validation
  - Added `config generate` command for creating sample configurations
  - Support for multiple transport types via CLI arguments

- **Testing Infrastructure**
  - Added comprehensive MCP integration tests
  - SSE transport tests with end-to-end validation
  - Streaming progress notification tests
  - Batch processing tests with dependency resolution

## 0.2.0 (2025-06-01)

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