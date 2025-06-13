# Changes

## Version 0.4.2 (2025-06-13)

### Features
- **Enhanced Interactive Console**: Significantly improved ratchet console with advanced interactive features
  - **Tab Completion**: Comprehensive tab completion for commands, actions, and filenames using rustyline
  - **Variable Expansion**: Advanced variable substitution supporting `$VAR`, `${VAR}`, `${ENV:VAR}`, `${VAR:-default}`, `${VAR:+value}` formats
  - **Intelligent Defaults**: Smart default actions for commands (e.g., `repo` → `repo list`, `health` → `health check`)
  - **Environment Variable Management**: New `env` command for viewing and managing environment variables
  - **Improved Error Messages**: Enhanced error handling with helpful command suggestions for better discoverability
  - **Single-Word Commands**: Better handling of single-word commands like `health`, `stats`, and `monitor`

### Bug Fixes
- **HTTPS Git Repository Support**: Restored HTTPS Git repository functionality that was broken during OpenSSL to rustls migration
- **Hybrid TLS Architecture**: Implemented optimal balance using rustls for HTTP client operations and OpenSSL for Git operations
- **Vendored OpenSSL by Default**: Made vendored OpenSSL the default for git2 builds to ensure cross-platform compatibility
- **GitHub Actions Enhancements**: Added perl5 installation across all target platforms (Ubuntu, Windows, macOS) for vendored OpenSSL builds
- **CLI Feature Integration**: Added git features to ratchet CLI package for proper GitHub Actions compatibility
- **Windows Build Fixes**: Resolved Windows perl PATH issues in GitHub Actions by replacing problematic refreshenv with explicit PATH updates

### Developer Experience
- **Console UX Improvements**: The interactive console now provides a significantly enhanced developer experience with context-aware completion, intelligent command defaults, and powerful variable substitution capabilities
- **Better Command Discoverability**: Enhanced error messages guide users to available commands and correct usage patterns

This release addresses critical Git repository functionality that was inadvertently broken during the security-focused OpenSSL to rustls migration, while also delivering major improvements to the interactive console experience. The console now offers advanced features comparable to modern CLI tools, making Ratchet administration and development workflows much more efficient and user-friendly.

---

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
