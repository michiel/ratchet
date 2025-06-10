# Ratchet Documentation Index

Welcome to the Ratchet documentation! This guide provides comprehensive coverage of all Ratchet features, from basic setup to advanced deployment scenarios.

## 📖 Core Documentation

### [Architecture Overview](../ARCHITECTURE.md) 
**Location: Root** - Comprehensive technical documentation covering:
- System architecture overview with unified binary design
- Process separation and worker architecture
- Component responsibilities and modular structure
- Technology stack and implementation details
- Migration from monolithic to modular architecture

### [CLI Usage Guide](CLI_USAGE.md)
Complete command-line interface documentation for the unified `ratchet` binary:
- `ratchet serve` - Full HTTP/GraphQL API server
- `ratchet mcp-serve` - MCP server for AI integration
- `ratchet run-once` - Direct task execution
- Environment variables and configuration
- GraphQL examples and workflows

### [Configuration Guide](CONFIGURATION_GUIDE.md)
**New Consolidated Guide** - Complete configuration reference covering:
- Basic and advanced configuration options
- Server modes and deployment scenarios
- Environment variables and secrets management
- Security configuration and authentication
- Performance tuning and optimization
- Docker and container deployment

## 🤖 AI & LLM Integration

### [MCP Integration Guide](MCP_INTEGRATION_GUIDE.md)
**New Comprehensive Guide** - Complete MCP integration documentation:
- Quick setup for Claude Desktop
- Available MCP tools and capabilities
- Authentication and security configuration
- Troubleshooting and best practices
- Advanced deployment scenarios
- Integration with other AI platforms

### [LLM Task Development Guide](LLM_TASK_DEVELOPMENT.md)
Guide for developing AI-compatible tasks:
- Task structure for LLM consumption
- Input/output schema design
- Error handling and debugging
- Integration patterns and examples

## 🌐 APIs & Integration

### [REST API Documentation](REST_API_README.md)
Complete REST API reference including:
- Refine.dev compatibility
- Endpoint documentation with examples
- Error handling patterns
- Pagination and filtering
- Authentication and security

### [OpenAPI Specification](openapi.yaml)
Machine-readable API specification with:
- Complete endpoint definitions
- Request/response schemas
- Authentication details
- Interactive viewer: [openapi-viewer.html](openapi-viewer.html)

### [Fetch API Guide](FETCH_API.md)
JavaScript fetch API documentation for task developers:
- HTTP request examples
- Request/response handling
- Error handling patterns
- Best practices and limitations

## 🔧 Development & Operations

### [Testing Guide](TESTING.md)
Comprehensive testing documentation:
- Unit and integration testing
- Task testing strategies
- CI/CD integration
- Test automation

### [Logging System](LOGGING_OVERVIEW.md)
Advanced logging infrastructure with AI-powered analysis:
- [System Overview](LOGGING_OVERVIEW.md) - Architecture and components
- [Usage Guide](LOGGING_USAGE.md) - Implementation and best practices
- Structured logging with contextual enrichment
- Error pattern recognition and automated analysis
- LLM-optimized export formats for debugging

### [Output Destinations Guide](OUTPUT_DESTINATIONS.md)
Flexible output delivery system for task results:
- Filesystem, webhook, database, and cloud storage destinations
- Template variables and dynamic paths
- Authentication and retry policies
- Cross-platform compatibility
- Configuration examples and best practices

### [Cross-Platform Considerations](CROSS-PLATFORM-CONSIDERATIONS.md)
Platform-specific deployment and compatibility guide:
- Windows, macOS, and Linux deployment
- File system differences and path handling
- Performance optimizations per platform
- Troubleshooting platform-specific issues

### [Build Optimization](BUILD_OPTIMIZATION_REPORT.md)
Build system optimization and static linking:
- Static builds for deployment
- Cross-compilation strategies
- Dependency management
- Performance optimizations

### [SSE Implementation](SSE_IMPLEMENTATION_SUMMARY.md)
Server-Sent Events implementation for real-time communication:
- WebSocket alternative design
- MCP transport layer
- Browser compatibility
- Performance characteristics

## 📋 Planning & Roadmap

### [Future Plans](plans/)
Detailed planning documents for upcoming features:

#### Workflow Engine
- [DAG Workflow Plan](plans/DAG_WORKFLOW_PLAN.md) - Visual workflow engine with branching logic
- [Execution Restructure Plan](plans/EXECUTION_RESTRUCTURE_PLAN.md) - Enhanced execution architecture

#### Marketplace & Distribution
- [Task Marketplace Plan](plans/TASK_MARKETPLACE_PLAN.md) - Task distribution ecosystem
- [Bundle packaging and monetization](plans/TASK_MARKETPLACE_PLAN.md#bundle-format)

#### AI Integration Expansion
- [MCP Bidirectional Design](plans/MCP_BIDIRECTIONAL_DESIGN.md) - Enhanced AI communication
- [MCP Task Example](plans/MCP_TASK_EXAMPLE.md) - Reference implementation

#### Infrastructure Improvements
- [Architecture Improvements](plans/ARCHITECTURE_IMPROVEMENTS.md) - System enhancements
- [Error Logging Improvements](plans/ERROR_LOGGING_IMPROVEMENT_PLAN.md) - Enhanced debugging

## 🔗 Additional Resources

### Quick References
- [rest-api-examples.sh](rest-api-examples.sh) - cURL examples for REST API
- [MCP Transport Protocols](mcp-transports.md) - Technical MCP implementation details

### Project Root Documentation
- [Project README](../README.md) - Project overview and quick start
- [Changelog](../CHANGES.md) - Release notes and version history  
- [Roadmap](../TODO.md) - Development priorities and upcoming features
- [Claude Configuration](../CLAUDE.md) - Project-specific AI assistant instructions

### Configuration Examples
Located in `../sample/configs/`:
- `example-config.yaml` - Basic server configuration
- `ratchet-mcp-config.yaml` - Complete MCP configuration
- `claude-desktop-*.json` - Claude Desktop integration examples
- `example-mcp-*.yaml` - Environment-specific MCP configs

### Archives
Completed migration and historical documentation moved to [docs/archives/](archives/):
- Architecture migration analysis
- Legacy implementation plans
- Codebase review analyses
- Migration completion reports

## 🚀 Getting Started

1. **New Users**: Start with [Project README](../README.md) for quick setup
2. **CLI Usage**: See [CLI Usage Guide](CLI_USAGE.md) for unified binary commands
3. **AI Integration**: Follow [MCP Integration Guide](MCP_INTEGRATION_GUIDE.md) for Claude Desktop setup
4. **Configuration**: Use [Configuration Guide](CONFIGURATION_GUIDE.md) for deployment scenarios
5. **Development**: Reference [Architecture Overview](../ARCHITECTURE.md) for system understanding

## 📚 Documentation Organization

### Root Directory (Required Files)
- `README.md` ✅ - Project overview
- `CHANGES.md` ✅ - Version history  
- `ARCHITECTURE.md` ✅ - System architecture
- `TODO.md` ✅ - Development roadmap

### docs/ Directory
- **Core guides** - Installation, configuration, usage
- **API documentation** - REST, GraphQL, OpenAPI specs
- **Integration guides** - AI, MCP, external systems
- **Development resources** - Testing, logging, cross-platform
- **Planning documents** - Future features and roadmap
- **Archives** - Historical and completed migration docs

### Key Changes Made
- ✅ **Consolidated MCP documentation** into single comprehensive guide
- ✅ **Created unified Configuration Guide** combining all config documentation
- ✅ **Moved ARCHITECTURE.md to root** per requirements
- ✅ **Updated all binary references** to unified `ratchet` command
- ✅ **Archived migration documents** to keep current docs clean
- ✅ **Fixed cross-references** and updated links throughout

This documentation structure provides clear navigation while maintaining compliance with the root file requirements and eliminating redundant content.