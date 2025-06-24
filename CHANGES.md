# Changes

## Unreleased (after v0.4.8)

### Major Features
- **Model Context Protocol (MCP) Integration**: Complete MCP server implementation with multi-transport support
  - Full MCP protocol v2024-11-05 support with resources, tools, and prompts
  - HTTP SSE transport with Claude Desktop compatibility and OAuth stub endpoints
  - STDIO transport for command-line MCP client integration
  - Comprehensive task execution through MCP with progress monitoring and log retrieval

### Architecture Improvements
- **Unified Task Service Architecture**: Storage-agnostic task abstraction layer
  - Created `TaskService` trait in ratchet-interfaces for unified task operations
  - Implemented `UnifiedTaskService` combining database and registry task access
  - Updated MCP adapter to use TaskService instead of direct repository access
  - Prepared foundation for REST and GraphQL to use same unified interface
  - Consistent UUID generation strategy for registry tasks using hash-based approach

- **JavaScript Execution Enhancement**: Complete JavaScript task execution pipeline
  - Fixed JavaScript execution in runtime worker with proper Boa engine integration
  - Implemented real JavaScript task execution replacing stubbed methods
  - Added thread-safe execution using `tokio::spawn_blocking` for non-Send operations
  - Created comprehensive task resolution with hardcoded samples for testing

### MCP Features & Enhancements
- **Advanced Template System**: Template management and versioning for MCP tools
  - Template import/export functionality with validation
  - Version management and compatibility checking
  - Dynamic template loading and caching

- **Comprehensive Tool Registry**: Full MCP tool integration with Ratchet's execution engine
  - Task execution tools with progress monitoring
  - Log retrieval tools with execution-specific filtering
  - Task listing and discovery tools
  - Execution status monitoring tools

- **Transport Layer Improvements**: Enhanced connectivity and compatibility
  - Streaming HTTP transport with tools/call method support
  - Proper protocol version handling and method resolution
  - Tool name compliance with MCP pattern requirements (^[a-zA-Z0-9_-]{1,64}$)
  - Claude Desktop connection issue resolution

### Infrastructure & System Improvements
- **Graceful Shutdown Enhancement**: Improved server lifecycle management
  - Implemented graceful shutdown coordination for background services
  - Better resource cleanup and connection handling
  - Enhanced service health monitoring

- **Docker & Build System**: Updated containerization and build infrastructure
  - Updated Docker build to Rust 1.86
  - Enhanced cross-platform compatibility
  - Improved build optimization for MCP features

### Documentation & Developer Experience
- **MCP Documentation**: Comprehensive MCP endpoint and usage documentation
  - Added MCP endpoints reference guide
  - Enhanced API documentation for MCP integration
  - Improved debugging and troubleshooting guides

- **Enhanced Logging & Monitoring**: Better observability for MCP operations
  - Startup port logging corrections
  - Improved error handling and debugging output
  - Enhanced execution monitoring and progress tracking

### Bug Fixes & Stability
- **Database & Repository Fixes**: Resolved critical data access issues
  - Fixed in-memory database connection pool issues causing table loss
  - Improved repository access patterns and error handling
  - Enhanced data consistency and transaction management

- **OpenAPI Documentation**: Re-enabled comprehensive API documentation
  - Fixed ToSchema compilation issues across multiple types
  - Enhanced schema generation for MCP-related endpoints
  - Improved API discoverability and documentation quality

- **GitHub Organization Migration**: Updated project organization and references
  - Migrated from michiel/ to ratchet-runner/ organization
  - Updated all repository references and documentation links
  - Enhanced project maintainability and collaboration structure

This release introduces comprehensive MCP support, establishing Ratchet as a fully-featured MCP server with advanced task execution capabilities, while laying the groundwork for unified task access across all interfaces.

