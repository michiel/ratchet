# Changes

## Version 0.4.4 (2025-06-15)

### Features
- **Unified MCP Command Structure**: Added new `mcp` command for general-purpose MCP server operations with SSE transport as default
- **Claude Code Compatibility**: Enhanced MCP protocol support with "2025-03-26" version for full Claude Code integration
- **Command Harmonization**: Standardized `mcp` and `mcp-serve` commands to share identical configuration and behavior, differing only in transport defaults

### Bug Fixes
- **MCP Protocol Version Support**: Fixed protocol handshake issues with Claude Code by adding latest protocol version "2025-03-26"
- **Future Type Compatibility**: Resolved Rust compilation errors in MCP server transport handling using boxed futures
- **Claude Code Tool Name Validation**: Fixed MCP tool name validation errors by replacing dots with underscores in all tool names to comply with Claude Code's `^[a-zA-Z0-9_-]{1,64}$` pattern

### Developer Experience
- **Comprehensive Documentation Updates**: Updated README.md, MCP documentation, and integration guides to reflect new command structure
- **Claude Code Integration**: Added seamless integration with `claude mcp add` command for zero-configuration setup
- **Flexible Transport Options**: Both MCP commands now support both stdio and SSE transports via `--transport` flag

### Infrastructure
- **Command Refactoring**: Consolidated MCP server logic into shared `mcp_command_with_config` function for better maintainability
- **Transport Detection**: Added intelligent stdio mode detection for proper logging behavior
- **Configuration Consistency**: Ensured both MCP commands use identical defaults and configuration handling

This release focuses on Claude Code integration and command structure improvements, providing a unified and flexible MCP server experience for both local desktop and web application use cases.