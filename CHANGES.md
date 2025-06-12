# Changes

## Version 0.4.0 (2025-01-06)

### Major Changes
- **Complete legacy deprecation**: Removed monolithic `ratchet-lib` crate in favor of modular architecture
- **Modular architecture**: Migrated to 20+ specialized crates (storage, execution, APIs, MCP, etc.)
- **Unified API interfaces**: Consistent REST and GraphQL endpoints with unified field naming and error handling

### Features
- **Enhanced CLI startup logging**: Comprehensive endpoint information organized by service type
- **MCP transport improvements**: Updated MCP integration with better protocol support
- **Cross-platform compatibility**: Improved support for Linux, macOS, and Windows

### Developer Experience
- **Better route visibility**: Server displays detailed HTTP methods and URLs at startup
- **Cleaner builds**: Removed unused imports and fixed compilation warnings
- **Testing improvements**: Resolved test compilation issues and dependency conflicts

### Bug Fixes
- Fixed critical test compilation issues across workspace
- Resolved build compilation errors in core modules
- Fixed testing module feature dependency issues in ratchet-storage

---

## Previous Development Progress

Ratchet has undergone significant architectural improvements focused on modernizing the codebase and enhancing developer experience. The project has successfully migrated from the legacy monolithic `ratchet-lib` crate to a modular architecture with specialized crates for different functionalities (storage, execution, APIs, MCP integration). This migration included implementing unified API interfaces across REST and GraphQL endpoints with consistent field naming and error handling, while cleaning up deprecated code paths and reducing build warnings.

The most recent enhancements center on improving the developer experience through enhanced CLI startup logging that provides comprehensive endpoint information organized by service type (Health, REST API, GraphQL, MCP). The server now displays detailed route information with HTTP methods and full URLs at startup, making it significantly easier for developers to understand available services and integrate with the APIs. Additionally, substantial code cleanup was performed to remove unused imports and fix compilation warnings, resulting in cleaner builds and better maintainability across the GraphQL, REST API, and MCP components.