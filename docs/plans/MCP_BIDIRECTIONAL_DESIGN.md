# Bidirectional MCP Integration: LLMs as Ratchet Clients

## Overview

The original MCP integration design enables Ratchet tasks to call out to LLMs. This extension explores the reverse direction: exposing Ratchet's capabilities as MCP tools that LLMs can invoke, including task execution, tracing, and logging access.

This would transform Ratchet into an MCP server that LLMs can use to:
- Execute tasks with full observability
- Query execution history and logs
- Monitor running tasks
- Access structured error information
- Perform complex automation workflows

## Architecture Extension

### Ratchet as MCP Server

```
┌─────────────────────┐
│   LLM/AI Agent      │
│  (Claude, GPT-4)    │
│  ┌───────────────┐  │
│  │ MCP Client    │  │
│  └───────┬───────┘  │
└──────────┼──────────┘
           │
    ┌──────┴──────┐
    │   Transport │
    │ (stdio/SSE) │
    └──────┬──────┘
           │
┌──────────▼──────────┐
│  Ratchet MCP Server │
│  ┌───────────────┐  │
│  │  Tool Registry│  │
│  └───────┬───────┘  │
└──────────┼──────────┘
           │
    ┌──────┴──────────┐
    │ Ratchet Core    │
    │ - Task Execution│
    │ - Logging       │
    │ - Tracing       │
    └─────────────────┘
```

### MCP Server Implementation

```rust
// ratchet-lib/src/mcp/server/mod.rs

pub struct RatchetMcpServer {
    task_service: Arc<dyn TaskService>,
    execution_service: Arc<dyn ExecutionService>,
    logging_service: Arc<dyn LoggingService>,
    auth_manager: Arc<AuthManager>,
}

impl RatchetMcpServer {
    pub async fn start(&self, config: McpServerConfig) -> Result<()> {
        match config.transport {
            McpTransport::Stdio => self.start_stdio_server().await,
            McpTransport::Sse => self.start_sse_server(config.bind_address).await,
        }
    }
    
    fn register_tools(&self) -> Vec<McpTool> {
        vec![
            // Task execution tools
            McpTool {
                name: "ratchet.execute_task",
                description: "Execute a Ratchet task with given input",
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {"type": "string"},
                        "input": {"type": "object"},
                        "trace": {"type": "boolean", "default": true}
                    },
                    "required": ["task_id", "input"]
                }),
            },
            
            // Execution monitoring tools
            McpTool {
                name: "ratchet.get_execution_status",
                description: "Get status and progress of a running execution",
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "execution_id": {"type": "string"}
                    },
                    "required": ["execution_id"]
                }),
            },
            
            // Logging access tools
            McpTool {
                name: "ratchet.get_execution_logs",
                description: "Retrieve logs for a specific execution",
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "execution_id": {"type": "string"},
                        "level": {"type": "string", "enum": ["trace", "debug", "info", "warn", "error"]},
                        "limit": {"type": "integer", "default": 100}
                    },
                    "required": ["execution_id"]
                }),
            },
            
            // Tracing tools
            McpTool {
                name: "ratchet.get_execution_trace",
                description: "Get detailed execution trace with timing and context",
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "execution_id": {"type": "string"},
                        "include_http_calls": {"type": "boolean", "default": true}
                    },
                    "required": ["execution_id"]
                }),
            },
            
            // Task discovery tools
            McpTool {
                name: "ratchet.list_available_tasks",
                description: "List all available tasks with their schemas",
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "filter": {"type": "string"},
                        "include_schemas": {"type": "boolean", "default": false}
                    }
                }),
            },
            
            // Error analysis tools
            McpTool {
                name: "ratchet.analyze_execution_error",
                description: "Get detailed error analysis for failed execution",
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "execution_id": {"type": "string"},
                        "include_suggestions": {"type": "boolean", "default": true}
                    },
                    "required": ["execution_id"]
                }),
            },
        ]
    }
}
```

## Tool Implementations

### Task Execution with Full Observability

```rust
// ratchet-lib/src/mcp/server/tools/execution.rs

async fn execute_task(
    &self,
    args: Value,
    context: &McpContext,
) -> Result<Value> {
    let task_id = args["task_id"].as_str().ok_or("task_id required")?;
    let input = &args["input"];
    let enable_trace = args["trace"].as_bool().unwrap_or(true);
    
    // Create execution with MCP context
    let execution_context = ExecutionContext {
        source: ExecutionSource::Mcp {
            session_id: context.session_id.clone(),
            client_info: context.client_info.clone(),
        },
        trace_enabled: enable_trace,
        log_level: if enable_trace { LogLevel::Debug } else { LogLevel::Info },
    };
    
    // Execute task with enhanced observability
    let execution_id = self.execution_service
        .create_execution(CreateExecutionRequest {
            task_id: task_id.to_string(),
            input_data: input.clone(),
            context: Some(execution_context),
        })
        .await?;
    
    // Stream progress updates back to LLM
    let progress_stream = self.create_progress_stream(&execution_id);
    
    // Wait for completion while streaming progress
    let result = self.execution_service
        .wait_for_completion(&execution_id, Some(progress_stream))
        .await?;
    
    // Return comprehensive result
    Ok(json!({
        "execution_id": execution_id,
        "status": result.status,
        "output": result.output,
        "duration_ms": result.duration_ms,
        "trace_url": format!("/traces/{}", execution_id),
        "logs_url": format!("/logs/{}", execution_id),
    }))
}
```

### Structured Log Access

```rust
// ratchet-lib/src/mcp/server/tools/logging.rs

async fn get_execution_logs(
    &self,
    args: Value,
    _context: &McpContext,
) -> Result<Value> {
    let execution_id = args["execution_id"].as_str().ok_or("execution_id required")?;
    let level_filter = args["level"].as_str().map(LogLevel::from_str).transpose()?;
    let limit = args["limit"].as_u64().unwrap_or(100) as usize;
    
    // Retrieve logs with LLM-optimized formatting
    let logs = self.logging_service
        .get_execution_logs(execution_id, LogQuery {
            level: level_filter,
            limit,
            format: LogFormat::LlmOptimized, // Special format for AI consumption
        })
        .await?;
    
    // Structure logs for AI analysis
    let structured_logs = logs.into_iter().map(|log| {
        json!({
            "timestamp": log.timestamp,
            "level": log.level,
            "message": log.message,
            "context": log.context,
            "error_info": log.error_info.map(|e| json!({
                "type": e.error_type,
                "message": e.message,
                "stack_trace": e.stack_trace,
                "suggestions": e.ai_suggestions,
            })),
        })
    }).collect::<Vec<_>>();
    
    Ok(json!({
        "execution_id": execution_id,
        "logs": structured_logs,
        "summary": generate_log_summary(&structured_logs),
    }))
}
```

### Execution Tracing

```rust
// ratchet-lib/src/mcp/server/tools/tracing.rs

async fn get_execution_trace(
    &self,
    args: Value,
    _context: &McpContext,
) -> Result<Value> {
    let execution_id = args["execution_id"].as_str().ok_or("execution_id required")?;
    let include_http = args["include_http_calls"].as_bool().unwrap_or(true);
    
    // Get comprehensive trace
    let trace = self.tracing_service
        .get_execution_trace(execution_id)
        .await?;
    
    // Format trace for AI consumption
    let formatted_trace = format_trace_for_llm(trace, include_http);
    
    Ok(json!({
        "execution_id": execution_id,
        "trace": formatted_trace,
        "visualization_url": format!("/traces/{}/visualize", execution_id),
        "insights": generate_trace_insights(&formatted_trace),
    }))
}

fn format_trace_for_llm(trace: ExecutionTrace, include_http: bool) -> Value {
    json!({
        "spans": trace.spans.into_iter()
            .filter(|span| include_http || span.operation_type != "http")
            .map(|span| json!({
                "name": span.name,
                "start_time": span.start_time,
                "duration_ms": span.duration_ms,
                "status": span.status,
                "attributes": span.attributes,
                "children": format_spans_tree(span.children),
            }))
            .collect::<Vec<_>>(),
        "critical_path": trace.critical_path,
        "bottlenecks": identify_bottlenecks(&trace),
    })
}
```

## Security & Access Control

### Authentication for MCP Connections

```rust
// ratchet-lib/src/mcp/server/auth.rs

pub struct McpAuthManager {
    allowed_clients: HashMap<String, ClientPermissions>,
}

#[derive(Debug, Clone)]
pub struct ClientPermissions {
    pub can_execute_tasks: bool,
    pub can_read_logs: bool,
    pub can_read_traces: bool,
    pub allowed_task_patterns: Vec<String>,
    pub rate_limits: RateLimits,
}

impl McpAuthManager {
    async fn authenticate_client(&self, auth: &McpAuth) -> Result<ClientContext> {
        match auth {
            McpAuth::ApiKey(key) => self.validate_api_key(key).await,
            McpAuth::OAuth2(token) => self.validate_oauth_token(token).await,
        }
    }
    
    fn authorize_tool_access(
        &self,
        client: &ClientContext,
        tool: &str,
        args: &Value,
    ) -> Result<()> {
        // Check permissions based on tool and arguments
        match tool {
            "ratchet.execute_task" => {
                if !client.permissions.can_execute_tasks {
                    return Err(McpError::Forbidden("Cannot execute tasks"));
                }
                // Check task pattern restrictions
                let task_id = args["task_id"].as_str().unwrap_or("");
                if !self.matches_allowed_patterns(task_id, &client.permissions.allowed_task_patterns) {
                    return Err(McpError::Forbidden("Task not allowed"));
                }
            }
            "ratchet.get_execution_logs" => {
                if !client.permissions.can_read_logs {
                    return Err(McpError::Forbidden("Cannot read logs"));
                }
            }
            // ... other tools
        }
        Ok(())
    }
}
```

### Rate Limiting

```rust
// ratchet-lib/src/mcp/server/rate_limit.rs

pub struct McpRateLimiter {
    limits: HashMap<String, RateLimit>,
}

impl McpRateLimiter {
    async fn check_rate_limit(
        &self,
        client: &ClientContext,
        tool: &str,
    ) -> Result<()> {
        let key = format!("{}:{}", client.id, tool);
        
        // Check tool-specific limits
        if let Some(limit) = self.limits.get(&key) {
            if !limit.check().await {
                return Err(McpError::RateLimited {
                    retry_after: limit.next_allowed_time(),
                });
            }
        }
        
        Ok(())
    }
}
```

## Configuration

### Server Configuration

```yaml
# Extended configuration for MCP server mode
mcp:
  # Client configuration (original design)
  servers:
    - name: "claude"
      # ...
      
  # Server configuration (new)
  server:
    enabled: true
    transport: "sse"  # or "stdio"
    bind_address: "0.0.0.0:8090"
    
    auth:
      type: "api_key"  # or "oauth2"
      api_keys:
        - key: "${MCP_API_KEY_1}"
          name: "ai-assistant"
          permissions:
            can_execute_tasks: true
            can_read_logs: true
            can_read_traces: true
            allowed_task_patterns:
              - "safe-*"
              - "read-only-*"
            rate_limits:
              execute_task: "10/minute"
              get_logs: "100/minute"
    
    # Security settings
    security:
      max_execution_time: 300  # 5 minutes
      max_log_entries: 1000
      allow_dangerous_tasks: false
      audit_log_enabled: true
    
    # Performance settings  
    performance:
      max_concurrent_executions_per_client: 5
      execution_queue_size: 100
      log_cache_ttl: 300  # 5 minutes
```

## Use Cases

### 1. AI-Powered Debugging Assistant

An LLM can help debug failing tasks:

```javascript
// LLM's perspective (pseudocode)
const execution = await mcp.invoke('ratchet.execute_task', {
    task_id: 'data-processor',
    input: { file: 'large-dataset.csv' }
});

if (execution.status === 'failed') {
    // Get detailed error analysis
    const error = await mcp.invoke('ratchet.analyze_execution_error', {
        execution_id: execution.execution_id
    });
    
    // Get relevant logs
    const logs = await mcp.invoke('ratchet.get_execution_logs', {
        execution_id: execution.execution_id,
        level: 'error'
    });
    
    // Get execution trace to understand flow
    const trace = await mcp.invoke('ratchet.get_execution_trace', {
        execution_id: execution.execution_id
    });
    
    // LLM analyzes all information and provides solution
    return analyzeAndSuggestFix(error, logs, trace);
}
```

### 2. Automated Workflow Orchestration

LLM orchestrates complex multi-step workflows:

```javascript
// LLM orchestrating a data pipeline
async function runDataPipeline(sourceFile) {
    // Step 1: Validate input
    const validation = await mcp.invoke('ratchet.execute_task', {
        task_id: 'validate-csv',
        input: { file: sourceFile }
    });
    
    if (!validation.output.valid) {
        return { error: 'Invalid input file', details: validation.output.errors };
    }
    
    // Step 2: Transform data
    const transform = await mcp.invoke('ratchet.execute_task', {
        task_id: 'transform-data',
        input: { 
            file: sourceFile,
            schema: validation.output.detected_schema
        }
    });
    
    // Monitor progress
    while (transform.status === 'running') {
        const status = await mcp.invoke('ratchet.get_execution_status', {
            execution_id: transform.execution_id
        });
        
        if (status.progress) {
            console.log(`Progress: ${status.progress.percent}%`);
        }
        
        await sleep(1000);
    }
    
    // Step 3: Generate report
    const report = await mcp.invoke('ratchet.execute_task', {
        task_id: 'generate-report',
        input: { 
            data: transform.output,
            format: 'pdf'
        }
    });
    
    return report.output;
}
```

### 3. Intelligent Monitoring

LLM monitors system health and takes corrective actions:

```javascript
// LLM monitoring and responding to issues
async function monitorSystem() {
    // Get recent executions
    const tasks = await mcp.invoke('ratchet.list_executions', {
        since: new Date(Date.now() - 3600000), // Last hour
        status: 'failed'
    });
    
    for (const task of tasks) {
        // Analyze each failure
        const analysis = await mcp.invoke('ratchet.analyze_execution_error', {
            execution_id: task.execution_id
        });
        
        // If it's a known recoverable error, retry with fixes
        if (isRecoverable(analysis)) {
            const fixedInput = applyFixes(task.input, analysis.suggestions);
            
            await mcp.invoke('ratchet.execute_task', {
                task_id: task.task_id,
                input: fixedInput,
                metadata: {
                    retry_reason: analysis.error_type,
                    original_execution: task.execution_id
                }
            });
        }
    }
}
```

## Benefits

### For LLM Integration

1. **Rich Context**: LLMs get detailed execution context, logs, and traces
2. **Structured Interaction**: Well-defined tool schemas ensure reliable invocation
3. **Observability**: Full visibility into task execution for better decision-making
4. **Error Analysis**: Detailed error information helps LLMs provide better solutions

### For Ratchet Platform

1. **Automated Operations**: LLMs can handle routine operational tasks
2. **Intelligent Debugging**: AI-assisted troubleshooting and error resolution
3. **Workflow Optimization**: LLMs can identify and optimize inefficient workflows
4. **Self-Healing**: Automatic retry and recovery from known issues

### For Users

1. **Natural Language Interface**: Interact with Ratchet through AI assistants
2. **Proactive Monitoring**: AI detects and resolves issues before users notice
3. **Enhanced Debugging**: Get AI-powered insights into execution failures
4. **Workflow Automation**: Complex workflows managed by AI

## Security Considerations

### Risks

1. **Unauthorized Execution**: LLMs executing dangerous or resource-intensive tasks
2. **Data Exposure**: Sensitive data in logs/traces accessed by LLMs
3. **Resource Exhaustion**: Unbounded task execution or log queries
4. **Prompt Injection**: Malicious inputs through LLM-generated task inputs

### Mitigations

1. **Strong Authentication**: API keys or OAuth2 for MCP connections
2. **Fine-grained Permissions**: Control which tasks LLMs can execute
3. **Rate Limiting**: Prevent resource exhaustion
4. **Input Sanitization**: Validate all LLM-provided inputs
5. **Audit Logging**: Track all LLM interactions
6. **Sandboxing**: Execute LLM-triggered tasks in restricted environments
7. **Data Filtering**: Redact sensitive information from logs/traces

## Implementation Phases

**UPDATED PRIORITIES**: MCP Server implementation is now the highest priority, with JavaScript integration deprioritized.

### Phase 1: Architecture Foundation (3-4 weeks) - **HIGHEST PRIORITY**
- Complete modularization with ratchet-mcp crate
- Enhanced worker architecture supporting persistent connections
- Bidirectional IPC layer for MCP message routing
- JSON-RPC 2.0 and MCP protocol implementation

### Phase 2: MCP Server Core (4-5 weeks) - **HIGHEST PRIORITY**
- Implement MCP server framework with tool registry
- Add tools for task execution, monitoring, and debugging
- Connection management and pooling
- Basic authentication and rate limiting

### Phase 3: Security & Performance (3-4 weeks) - **HIGH PRIORITY**
- Comprehensive security controls and permissions
- Advanced rate limiting and quotas
- Performance optimization for high-frequency operations
- Streaming and real-time support

### Phase 4: Production Hardening (2-3 weeks) - **HIGH PRIORITY**
- Comprehensive audit logging
- Monitoring and alerting
- Error analysis and debugging tools
- Documentation and examples

### Phase 5: JavaScript Integration (2-3 weeks) - **DEPRIORITIZED**
- MCP JavaScript API for task environment
- Client-side MCP operations
- Connection management from JavaScript

**Note**: JavaScript integration has been deprioritized to focus on the core MCP server infrastructure that enables LLMs to control Ratchet. This provides immediate value for AI-powered automation scenarios.

## Conclusion

Exposing Ratchet as an MCP server creates a powerful bidirectional integration where:
- Ratchet tasks can call out to LLMs for AI capabilities
- LLMs can call into Ratchet to execute tasks with full observability

This transforms Ratchet into an AI-native automation platform where LLMs can:
- Execute and monitor tasks
- Debug failures using logs and traces
- Orchestrate complex workflows
- Provide intelligent automation

The key is maintaining security through proper authentication, authorization, and rate limiting while providing rich enough context for LLMs to be effective operators of the Ratchet platform.