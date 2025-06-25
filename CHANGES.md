# Changes

## v0.4.10 (2025-06-25)

### Build & Development Experience
- **Docker Build Performance**: Comprehensive build optimization with caching
  - Implement dependency layer caching in Dockerfile for faster rebuilds
  - Add parallel multi-platform builds (amd64/arm64) in GitHub Actions
  - Create dependency pre-compilation workflow for cache warming
  - Add conditional build triggers to skip unnecessary builds
  - Enhance .dockerignore to reduce build context size
  - Expected performance: 80% faster builds for code changes

- **Test Infrastructure Improvements**: Resolved compilation errors and enhanced testing
  - Fixed axum integration test compilation errors across multiple test files
  - Resolved async/await issues in server build_app() calls
  - Fixed MCP test trait bound issues with UnifiedTaskService integration
  - Reduced build warnings through systematic cleanup (65 → 60 warnings)
  - Applied both automatic (cargo fix) and manual fixes for unused variables

### Configuration Management
- **Streamlined Configuration Examples**: Comprehensive configuration cleanup
  - Created two definitive example configurations: minimal.yaml and full.yaml
  - Added detailed comments explaining all configuration options and domains
  - Removed 551 legacy sample/example files (74,101 deletions) 
  - Maintained production-ready settings with security and performance considerations

### Documentation Enhancements
- **Documentation Restructuring**: Streamlined and focused documentation
  - Removed 37 redundant markdown files while preserving strategic content
  - Kept essential documentation: docs/plans/, docs/reviews/, docs/MCP_ENDPOINTS_REFERENCE.md
  - Created comprehensive MCP-based task development guide with working examples
  - Added detailed logging, tracing, and performance monitoring sections
  - Included complete task development workflow from connection to execution

- **Task Development Guide**: Complete MCP interface documentation
  - Step-by-step task development workflow using MCP protocol
  - Working example: HTTP GET task that extracts origin information from httpbin.org
  - Comprehensive error handling and debugging examples
  - Real-world JSON-RPC 2.0 communication patterns
  - Execution monitoring and log analysis techniques

### Code Quality & Maintenance
- **Build System Reliability**: Enhanced compilation and dependency management
  - Fixed trait bound issues in MCP adapter integration
  - Resolved unused import and variable warnings across the codebase
  - Improved axum service setup with proper .into_make_service() calls
  - Enhanced database connection handling in integration tests

- **GitHub Actions**: Improved CI/CD pipeline configuration
  - Reset actions-rust-release origin for better workflow management
  - Enhanced release automation and dependency caching
  - Improved multi-platform build support

This release focuses on developer experience improvements, build performance optimization, and comprehensive documentation while maintaining the robust MCP integration established in previous releases.

## v0.4.9 (2024-06-24)

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

