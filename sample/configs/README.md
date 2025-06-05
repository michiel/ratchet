# Sample Configuration Files

This directory contains example configuration files for various Ratchet deployment scenarios.

## Configuration Files

### General Ratchet Configurations
- `example-config.yaml` - Basic Ratchet configuration example

### MCP (Model Context Protocol) Configurations
- `example-mcp-minimal.yaml` - Minimal MCP configuration for getting started
- `example-mcp-dev.yaml` - Development environment MCP configuration
- `example-mcp-production.yaml` - Production-ready MCP configuration with security
- `example-mcp-enterprise.yaml` - Enterprise MCP configuration with advanced features
- `example-mcp-claude-integration.yaml` - Claude Desktop integration configuration

### Claude Desktop Integration
- `claude-config.json` - Example Claude Desktop MCP server configuration
- `claude_desktop_config.json` - Annotated Claude Desktop configuration with instructions

### SSE (Server-Sent Events) Configuration
- `example-sse-config.yaml` - SSE transport configuration for HTTP-based connections

### Test Configurations
- `test-config.yaml` - Configuration for running tests
- `ratchet-mcp-config.yaml` - MCP test configuration

## Usage

To use any of these configurations:

1. Copy the desired configuration file to your preferred location
2. Modify the settings according to your requirements
3. Reference the configuration when starting Ratchet:

```bash
# For general Ratchet server
ratchet server --config path/to/your-config.yaml

# For MCP server
ratchet-mcp -c path/to/your-mcp-config.yaml serve
```

## Configuration Hierarchy

- **Minimal**: Start here for basic setups
- **Development**: Includes debugging and development features
- **Production**: Adds security, rate limiting, and performance optimizations
- **Enterprise**: Full-featured configuration with all bells and whistles

For detailed configuration documentation, see:
- [MCP Configuration Guide](/docs/MCP_CONFIGURATION_GUIDE.md)
- [Server Configuration Guide](/docs/SERVER_CONFIGURATION_GUIDE.md)
- [Claude Desktop Setup](/docs/CLAUDE_DESKTOP_SETUP.md)