# MCP Monitoring Tools Implementation

This document describes the implementation of monitoring tools for `get_execution_status` and `get_execution_logs` in the Ratchet MCP server.

## Summary

I have successfully implemented two key monitoring tools that integrate with Ratchet's execution repository and logging system to provide real-time monitoring capabilities:

1. **`ratchet.get_execution_status`** - Retrieves comprehensive execution status information
2. **`ratchet.get_execution_logs`** - Searches and retrieves execution-specific logs

## Implementation Details

### Files Modified

1. **`ratchet-mcp/src/server/adapter.rs`**
   - Added `McpExecutionStatus` import and usage
   - Implemented `get_execution_status()` method in `McpTaskExecutor` trait
   - Enhanced log search functionality with better error handling and multiple search strategies

2. **`ratchet-mcp/src/server/tools.rs`**
   - Added `McpExecutionStatus` struct for structured status responses
   - Updated `McpTaskExecutor` trait with `get_execution_status()` method
   - Modified `get_execution_status_tool()` to use real repository data instead of placeholder
   - Enhanced error handling and response formatting

3. **`ratchet-mcp/src/tests/mod.rs`**
   - Added comprehensive tests for monitoring tools
   - Tests cover both valid/invalid UUIDs and error conditions
   - Verified tool registration and availability

### Key Features Implemented

#### 1. `ratchet.get_execution_status`

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "execution_id": {
      "type": "string",
      "description": "ID of the execution to check"
    }
  },
  "required": ["execution_id"]
}
```

**Functionality:**
- Validates execution ID as a proper UUID
- Queries the execution repository for real execution data
- Returns comprehensive status including:
  - Execution state (pending, running, completed, failed, cancelled)
  - Timing information (queued, started, completed timestamps)
  - Duration calculations
  - Input/output data
  - Error details if applicable
  - Progress estimation based on execution state

**Response Format:**
```json
{
  "execution_id": "uuid",
  "status": "completed|running|pending|failed|cancelled",
  "task_id": 123,
  "input": {...},
  "output": {...},
  "error_message": "string|null",
  "error_details": {...},
  "queued_at": "ISO 8601 timestamp",
  "started_at": "ISO 8601 timestamp|null",
  "completed_at": "ISO 8601 timestamp|null",
  "duration_ms": 1234,
  "progress": {
    "current_step": "running",
    "elapsed_ms": 5000,
    "percentage": null
  }
}
```

#### 2. `ratchet.get_execution_logs`

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "execution_id": {
      "type": "string",
      "description": "ID of the execution"
    },
    "level": {
      "type": "string",
      "enum": ["trace", "debug", "info", "warn", "error"],
      "description": "Minimum log level to retrieve"
    },
    "limit": {
      "type": "integer",
      "default": 100,
      "description": "Maximum number of log entries"
    },
    "format": {
      "type": "string",
      "enum": ["json", "text"],
      "default": "json",
      "description": "Output format"
    }
  },
  "required": ["execution_id"]
}
```

**Functionality:**
- Multi-strategy log retrieval:
  1. **Recording Path**: Checks for HAR format execution recordings (future implementation)
  2. **Log File Search**: Searches configured log files for execution-specific entries
  3. **Database Fallback**: Returns basic execution info when detailed logs unavailable

**Enhanced Log Search Features:**
- Searches multiple fields: `trace_id`, `span_id`, `execution_id`, `exec_id`, message content
- Handles both structured JSON logs and plain text logs
- Provides detailed search statistics
- Configurable log level filtering
- Line number tracking for debugging

**Response Format:**
```json
{
  "execution_id": "uuid",
  "log_file": "/path/to/log/file",
  "logs": [
    {
      "timestamp": "ISO 8601",
      "level": "info",
      "message": "Log message",
      "logger": "ratchet.execution",
      "fields": {...},
      "error": {...},
      "trace_id": "uuid",
      "span_id": "uuid",
      "line_number": 123
    }
  ],
  "total_found": 15,
  "limit_applied": 100,
  "min_level": "info",
  "search_stats": {
    "total_lines_processed": 1000,
    "parse_errors": 5,
    "has_more": false
  },
  "search_criteria": {
    "execution_id": "uuid",
    "search_fields": ["trace_id", "span_id", "execution_id", "exec_id", "message_content"]
  }
}
```

### Error Handling

Both tools implement robust error handling:

- **Invalid UUID Format**: Clear error messages for malformed execution IDs
- **Execution Not Found**: Appropriate responses when execution doesn't exist in database
- **Database Errors**: Proper error propagation with context
- **Log File Access**: Graceful fallbacks when log files are unavailable
- **Configuration Issues**: Clear messages when repositories aren't configured

### Integration with Ratchet Architecture

The implementation seamlessly integrates with Ratchet's existing architecture:

- **Repository Pattern**: Uses `ExecutionRepository` for database operations
- **Logging System**: Integrates with `LogEvent` and `LogLevel` structures
- **Process Executor**: Works with the existing `ProcessTaskExecutor` infrastructure
- **MCP Protocol**: Follows MCP tool standards with proper schema definitions

## Testing

Comprehensive test coverage includes:

- **Tool Registration**: Verifies tools are properly registered and discoverable
- **Invalid Input Handling**: Tests with malformed UUIDs and missing executions
- **Error Response Format**: Validates error responses follow expected format
- **Integration Testing**: End-to-end MCP server testing with monitoring tools

All tests pass successfully and demonstrate the tools work correctly within the MCP framework.

## Usage Example

Tools can be called through any MCP-compatible client:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "ratchet.get_execution_status",
    "arguments": {
      "execution_id": "123e4567-e89b-12d3-a456-426614174000"
    }
  }
}
```

## Future Enhancements

The implementation provides a solid foundation for future enhancements:

1. **HAR File Parsing**: Complete implementation of recording path log retrieval
2. **Real-time Progress**: Integration with task-specific progress reporting
3. **Log Streaming**: WebSocket-based real-time log streaming
4. **Performance Metrics**: Detailed execution performance analytics
5. **Alert Integration**: Automatic error detection and notification

## Conclusion

The monitoring tools successfully bridge the gap between Ratchet's powerful execution engine and external monitoring systems, providing real-time visibility into task execution status and comprehensive log access through the standardized MCP protocol.