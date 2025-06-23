# MCP Tools Integration Plan

**Date**: 2025-06-23  
**Author**: Claude Code  
**Status**: Phase 1 Complete - Phase 2 In Progress  
**Priority**: High  

## Executive Summary

This plan addresses the critical gap between the comprehensive MCP server implementation (23 tools) and the basic transport handlers (2 stub tools) currently exposed to Claude. The goal is to integrate the full MCP tool registry with the streaming HTTP transport to provide Claude with complete Ratchet functionality.

## Current State Analysis

### What Exists
- ✅ **Full MCP Server**: 23 comprehensive tools in `ratchet-mcp/src/server/tools.rs`
- ✅ **Transport Layer**: StreamableHTTP and SSE handlers in `ratchet-server/src/mcp_handler.rs`
- ✅ **Infrastructure**: Authentication, session management, database integration
- ❌ **Integration**: Transport handlers only expose 2 basic stub tools

### What's Missing
- **Tool Registry Connection**: Transport handlers don't use the full MCP server
- **Request Routing**: No bridge between HTTP requests and tool execution
- **Session Context**: Tools need access to session and execution context
- **Error Handling**: Transport-specific error handling for tool failures

## Implementation Plan

### Phase 1: Core Integration (1-2 days)

#### 1.1 Connect MCP Server to Transport Handlers
**Files**: `ratchet-server/src/mcp_handler.rs`, `ratchet-server/src/startup.rs`

```rust
// Update McpEndpointState to include tool registry
pub struct McpEndpointState {
    pub config: McpApiConfig,
    pub mcp_server: Arc<McpServer>,
    pub tool_registry: Arc<RatchetToolRegistry>, // Add this
    // ... existing fields
}
```

**Tasks**:
- [ ] Add `RatchetToolRegistry` to `McpEndpointState`
- [ ] Update `handle_sse_request` to use registry for `tools/list`
- [ ] Update `handle_streamable_http_request` to use registry
- [ ] Replace hardcoded tool responses with registry lookups

#### 1.2 Implement Tool Execution Bridge
**Files**: `ratchet-server/src/mcp_handler.rs`

```rust
async fn execute_tool_from_registry(
    registry: &RatchetToolRegistry,
    tool_name: &str,
    arguments: serde_json::Value,
    context: ToolExecutionContext,
) -> Result<serde_json::Value, McpError>
```

**Tasks**:
- [ ] Create tool execution bridge function
- [ ] Handle tool authentication and authorization
- [ ] Map transport requests to tool registry calls
- [ ] Convert tool responses to JSON-RPC format

#### 1.3 Update tools/call Method Handlers
**Files**: `ratchet-mcp/src/transport/streamable_http.rs`, `ratchet-server/src/mcp_handler.rs`

**Tasks**:
- [ ] Remove hardcoded tool responses
- [ ] Route `tools/call` requests through tool registry
- [ ] Handle dynamic tool discovery
- [ ] Support all 23 implemented tools

**Estimated Time**: 2 days  
**Dependencies**: None  
**Risk**: Low - Straightforward integration work

### Phase 2: JavaScript Execution Integration (3-5 days)

#### 2.1 Enhance Test Execution Tools
**Files**: `ratchet-mcp/src/server/tools.rs`

**Current Status**: `ratchet_run_task_tests` returns basic stub

**Implementation**:
- [ ] Integrate with existing Ratchet task execution engine
- [ ] Support JavaScript test execution via Boa engine
- [ ] Implement test result collection and reporting
- [ ] Add test failure analysis and debugging info

#### 2.2 Complete Task Debugging Tools
**Files**: `ratchet-mcp/src/server/tools.rs`

**Current Status**: `ratchet_debug_task_execution` returns structured stub

**Implementation**:
- [ ] Add breakpoint support to JavaScript execution
- [ ] Implement variable inspection and step-through debugging
- [ ] Create debug session management
- [ ] Support remote debugging via MCP

#### 2.3 Real Task Execution via MCP
**Files**: `ratchet-mcp/src/server/tools.rs`

**Current Status**: `ratchet_execute_task` functional but may need enhancement

**Implementation**:
- [ ] Verify full integration with Ratchet execution engine
- [ ] Support progress streaming through MCP transport
- [ ] Handle task input/output serialization
- [ ] Implement execution cancellation support

**Estimated Time**: 4 days  
**Dependencies**: Phase 1 completion  
**Risk**: Medium - Requires deep integration with execution engine

### Phase 3: Advanced Development Tools (2-3 days)

#### 3.1 Template System Implementation
**Files**: `ratchet-mcp/src/server/tools.rs`, new template module

**Current Status**: `ratchet_generate_from_template`, `ratchet_list_templates` are stubs

**Implementation**:
- [ ] Create template definition system
- [ ] Build template library with common patterns
- [ ] Implement template parameter substitution
- [ ] Support custom template creation and management

**Templates to Include**:
- HTTP API client task
- Data processing task
- Webhook handler task
- Scheduled job task
- Testing utility task

#### 3.2 Enhanced Import/Export Tools
**Files**: `ratchet-mcp/src/server/tools.rs`

**Current Status**: Basic stub implementations

**Implementation**:
- [ ] Support ZIP file import/export
- [ ] Implement directory-based import/export
- [ ] Add task dependency resolution
- [ ] Support bulk operations with progress reporting

#### 3.3 Version Management System
**Files**: `ratchet-mcp/src/server/tools.rs`, database migrations

**Current Status**: `ratchet_create_task_version` returns stub

**Implementation**:
- [ ] Design task version database schema
- [ ] Implement version creation and management
- [ ] Support task migration between versions
- [ ] Add version comparison and rollback features

**Estimated Time**: 3 days  
**Dependencies**: Phase 1 completion  
**Risk**: Low-Medium - Well-defined requirements

### Phase 4: Production Readiness (1-2 days)

#### 4.1 Comprehensive Error Handling
**Files**: All MCP handler files

**Implementation**:
- [ ] Standardize error responses across all tools
- [ ] Add detailed error context and suggestions
- [ ] Implement error recovery mechanisms
- [ ] Support error reporting and analytics

#### 4.2 Performance Optimization
**Files**: Transport and tool execution files

**Implementation**:
- [ ] Add tool execution caching where appropriate
- [ ] Implement request batching for bulk operations
- [ ] Optimize session management for high concurrency
- [ ] Add performance monitoring and metrics

#### 4.3 Documentation and Examples
**Files**: `docs/mcp/`, example configurations

**Implementation**:
- [ ] Document all 23 MCP tools with examples
- [ ] Create Claude usage guides for each tool category
- [ ] Add troubleshooting guides
- [ ] Build comprehensive API reference

**Estimated Time**: 2 days  
**Dependencies**: Phases 1-3 completion  
**Risk**: Low - Documentation and polish work

## Success Criteria

### Phase 1 Success Metrics ✅ COMPLETED
- [x] Claude can discover all 23 tools via `tools/list`
- [x] All fully implemented tools (15/23) work through Claude
- [x] No regression in existing functionality
- [x] Transport handlers pass all existing tests

**Phase 1 Results (Completed 2025-06-23)**:
- Successfully integrated RatchetToolRegistry with transport handlers
- All 23 tools now discoverable via tools/list (vs previous 2 stub tools)
- Tool execution bridge implemented with proper JSON-RPC 2.0 responses
- Both SSE and StreamableHTTP transports updated
- 15 tools are immediately functional, 8 require task executor integration
- Server startup shows: "🤖 MCP Server-Sent Events API: Tools Available: ratchet.execute_task, ratchet.get_execution_status..." (full list of 23 tools)

### Phase 2 Success Metrics
- [ ] JavaScript test execution works via `ratchet_run_task_tests`
- [ ] Task debugging supports breakpoints and variable inspection
- [ ] Real task execution completes successfully through MCP
- [ ] Progress streaming works for long-running tasks

### Phase 3 Success Metrics
- [ ] Template system generates functional tasks
- [ ] Import/export handles complex task hierarchies
- [ ] Version management supports task evolution
- [ ] All 23 tools are fully functional

### Phase 4 Success Metrics
- [ ] Production-ready error handling and recovery
- [ ] Performance meets scalability requirements
- [ ] Complete documentation with examples
- [ ] Ready for Claude Code production use

## Timeline and Resource Allocation

### Total Estimated Time: 8-12 days

| Phase | Duration | Effort Level | Complexity |
|-------|----------|--------------|------------|
| Phase 1 | 2 days | High | Low |
| Phase 2 | 4 days | High | Medium |
| Phase 3 | 3 days | Medium | Medium |
| Phase 4 | 2 days | Medium | Low |

### Critical Path
1. **Phase 1** must complete before any other phase
2. **Phase 2** can partially overlap with Phase 3 for templates
3. **Phase 4** depends on completion of phases 1-3

## Risk Assessment and Mitigation

### High Risk Items
- **JavaScript execution integration**: Complex engine integration
  - *Mitigation*: Start with existing Ratchet execution patterns
  - *Fallback*: Implement basic execution first, enhance later

### Medium Risk Items
- **Performance under load**: Many tools, complex operations
  - *Mitigation*: Implement caching and optimization from start
  - *Monitoring*: Add metrics early in development

### Low Risk Items
- **Template system**: Well-defined requirements
- **Documentation**: Straightforward implementation

## Dependencies and Prerequisites

### Internal Dependencies
- Existing Ratchet task execution engine
- Database schema and migrations
- Authentication and authorization system
- Transport layer and session management

### External Dependencies
- Boa JavaScript engine (already integrated)
- SeaORM database layer (already integrated)
- Tokio async runtime (already integrated)

## Testing Strategy

### Unit Tests
- [ ] Test each tool individually with mock dependencies
- [ ] Test transport integration with tool registry
- [ ] Test error handling and edge cases

### Integration Tests
- [ ] Test full Claude → MCP → Ratchet execution flow
- [ ] Test session management under load
- [ ] Test all transport types (SSE, StreamableHTTP, stdio)

### End-to-End Tests
- [ ] Test complete task development workflow via Claude
- [ ] Test debugging and troubleshooting scenarios
- [ ] Test bulk operations and performance limits

## Implementation Notes

### Code Organization
```
ratchet-server/src/
├── mcp_handler.rs          # Transport integration (Phase 1)
├── mcp_bridge.rs           # New: Tool execution bridge (Phase 1)
└── startup.rs              # Updated endpoint configuration

ratchet-mcp/src/server/
├── tools.rs                # Enhanced tool implementations (Phases 2-3)
├── templates/              # New: Template system (Phase 3)
└── mod.rs                  # Updated exports

docs/
├── mcp/                    # New: MCP documentation (Phase 4)
└── plans/                  # This plan
```

### Configuration Changes
```yaml
# Example enhanced MCP configuration
mcp:
  transport: both
  tools:
    enable_javascript_execution: true
    enable_debugging: true
    template_directory: "templates/"
    max_execution_time: 300s
```

## Future Enhancements (Beyond This Plan)

### Advanced Features
- Machine learning-based error analysis
- Task performance optimization suggestions
- Collaborative task development features
- Integration with external development tools

### Ecosystem Integration
- VS Code extension for Ratchet task development
- GitHub Actions integration for CI/CD
- Monitoring and alerting for production deployments

## Conclusion

This plan transforms the Ratchet MCP implementation from a basic transport layer with stub tools into a comprehensive development platform accessible through Claude. The phased approach ensures steady progress while maintaining system stability.

The integration of all 23 tools will provide Claude with unprecedented access to Ratchet's task development, execution, and management capabilities, making it a powerful platform for automated task development and operations.