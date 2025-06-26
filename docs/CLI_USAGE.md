# Ratchet CLI Usage Guide

This document provides comprehensive usage examples for the Ratchet CLI tool.

## Table of Contents

- [Basic Commands](#basic-commands)
- [Task Operations](#task-operations)
- [Server Operations](#server-operations)
- [Configuration Management](#configuration-management)
- [Repository Management](#repository-management)
- [Code Generation](#code-generation)
- [Interactive Console](#interactive-console)
- [Global Options](#global-options)

## Basic Commands

### Get Help

```bash
# General help
ratchet --help

# Command-specific help
ratchet serve --help
ratchet generate --help
```

### Version Information

```bash
ratchet --version
```

## Task Operations

### Run a Single Task

Execute a task from the filesystem with JSON input:

```bash
# Basic task execution
ratchet run-once --from-fs ./my-task --input-json '{}'

# Task with input parameters
ratchet run-once --from-fs ./api-client-task --input-json '{"url": "https://api.example.com", "timeout": 5000}'

# Record execution for debugging
ratchet run-once --from-fs ./complex-task --input-json '{"param": "value"}' --record ./recordings/
```

### Validate Tasks

Check task definitions for correctness:

```bash
# Basic validation
ratchet validate --from-fs ./my-task

# Auto-fix missing metadata
ratchet validate --from-fs ./my-task --fix
```

### Test Tasks

Run task test cases:

```bash
# Run all test cases
ratchet test --from-fs ./my-task

# Test specific task with validation
ratchet validate --from-fs ./http-client-task && ratchet test --from-fs ./http-client-task
```

### Replay Recorded Executions

Debug tasks using recorded execution data:

```bash
# Replay a recorded execution
ratchet replay --from-fs ./my-task --recording ./recordings/2025-06-26_14-30-15/

# Combine with validation for debugging
ratchet validate --from-fs ./my-task && ratchet replay --from-fs ./my-task --recording ./debug-session/
```

## Server Operations

### Start the Ratchet Server

Launch the unified server with REST API, GraphQL, and web interface:

```bash
# Start with default configuration
ratchet serve

# Start with custom configuration
ratchet serve --config ./production.yaml

# Start with specific log level
ratchet serve --config ./config.yaml --log-level debug
```

### Start MCP Server

Run the Model Context Protocol server for AI agent integration:

```bash
# STDIO transport (default) - for Claude Desktop
ratchet mcp-serve

# HTTP SSE transport for web clients
ratchet mcp-serve --transport sse --host 0.0.0.0 --port 8090

# MCP server with custom configuration
ratchet mcp-serve --config ./mcp-config.yaml --transport stdio

# MCP server for development
ratchet mcp-serve --transport sse --host localhost --port 3001 --log-level debug
```

### Legacy MCP Command

```bash
# Alternative MCP server command (equivalent to mcp-serve with stdio)
ratchet mcp --config ./config.yaml
```

## Configuration Management

### Generate Configuration Files

Create sample configuration files for different environments:

```bash
# Generate minimal configuration
ratchet config generate --type minimal --format yaml > minimal.yaml

# Generate full production configuration
ratchet config generate --type production --format yaml > production.yaml

# Generate development configuration
ratchet config generate --type dev --format json > dev.json

# Generate enterprise configuration with all features
ratchet config generate --type enterprise --format yaml > enterprise.yaml

# Generate Claude Desktop MCP configuration
ratchet config generate --type claude --format json > claude-mcp.json
```

### Validate Configuration

Check configuration file syntax and settings:

```bash
# Validate configuration file
ratchet config validate --config ./production.yaml

# Validate with detailed output
ratchet config validate --config ./config.yaml --log-level debug
```

### Show Current Configuration

Display the active configuration:

```bash
# Show full configuration
ratchet config show

# Show only MCP configuration
ratchet config show --mcp-only

# Show configuration in JSON format
ratchet config show --format json

# Show configuration from specific file
ratchet config show --config ./production.yaml --format yaml
```

## Repository Management

### Initialize Repositories

Create new task repositories:

```bash
# Initialize basic repository
ratchet repo init ./my-tasks --name "My Task Repository" --description "Custom tasks for my project"

# Initialize with specific version requirements
ratchet repo init ./enterprise-tasks \
  --name "Enterprise Tasks" \
  --description "Company-wide task library" \
  --version "2.0.0" \
  --ratchet-version ">=0.4.11"

# Force initialization in non-empty directory
ratchet repo init ./existing-dir --name "Legacy Tasks" --force
```

### Repository Status

Check the status of configured repositories:

```bash
# Basic status
ratchet repo status

# Detailed status with task counts
ratchet repo status --detailed

# Status for specific repository
ratchet repo status --repository "my-tasks" --detailed

# JSON output for automation
ratchet repo status --format json
```

### Verify Repositories

Test repository accessibility and list available tasks:

```bash
# Verify all repositories
ratchet repo verify

# Verify specific repository with task listing
ratchet repo verify --repository "enterprise-tasks" --list-tasks --detailed

# Offline verification (skip connectivity tests)
ratchet repo verify --offline --format yaml

# Detailed verification with JSON output
ratchet repo verify --detailed --format json
```

### Refresh Repository Metadata

Update repository index and metadata:

```bash
# Refresh all repositories
ratchet repo refresh-metadata

# Refresh specific repository
ratchet repo refresh-metadata ./my-tasks

# Force complete regeneration
ratchet repo refresh-metadata ./my-tasks --force
```

## Code Generation

### Generate Task Templates

Create new task scaffolding:

```bash
# Basic task generation
ratchet generate task --path ./new-task --label "http-client" --description "HTTP client task"

# Task with specific version
ratchet generate task \
  --path ./advanced-task \
  --label "data-processor" \
  --description "Process and transform data" \
  --version "2.1.0"

# Generate multiple tasks
for task in user-auth data-sync report-gen; do
  ratchet generate task --path ./${task} --label "${task}" --description "Generated ${task} task"
done
```

### Generate MCP Configuration

Create Claude Desktop MCP server configuration:

```bash
# Basic MCP servers.json generation
ratchet generate mcpservers-json --name "ratchet" > mcpServers.json

# Custom MCP configuration with specific transport
ratchet generate mcpservers-json \
  --name "my-ratchet" \
  --command "ratchet" \
  --args "mcp-serve" \
  --transport "stdio" \
  --pretty > ~/.config/claude/mcpServers.json

# HTTP SSE transport configuration
ratchet generate mcpservers-json \
  --name "ratchet-web" \
  --transport "sse" \
  --host "localhost" \
  --port "8090" \
  --env "RUST_LOG=debug" \
  --pretty > mcp-web-config.json

# Production MCP configuration
ratchet generate mcpservers-json \
  --name "ratchet-prod" \
  --command "/usr/local/bin/ratchet" \
  --args "mcp-serve" \
  --config "/etc/ratchet/production.yaml" \
  --env "RUST_LOG=info" \
  --env "RATCHET_ENV=production" \
  --format "json" > production-mcp.json
```

## Interactive Console

Start the interactive console for administration:

```bash
# Start local console
ratchet console

# Connect to remote Ratchet server
ratchet console --connect-url "http://localhost:8080"

# Console with authentication
ratchet console --connect-url "https://ratchet.company.com" --auth-token "your-token"

# Console with specific transport
ratchet console --transport "http" --connect-url "http://localhost:8080"
```

### Console Commands

Once in the console, you can use commands like:

```
# Repository management
repo list
repo status
repo add git@github.com:company/ratchet-tasks.git
repo refresh

# Task management  
task list
task show my-task
task execute my-task --input '{"key": "value"}'

# Execution monitoring
execution list
execution show <execution-id>
execution logs <execution-id>

# Server management
server status
server metrics
health check

# Database operations
db status
db migrate
db stats
```

## Global Options

These options are available for all commands:

### Configuration File

```bash
# Use specific configuration file
ratchet --config ./custom.yaml serve
ratchet --config /etc/ratchet/prod.yaml mcp-serve
```

### Logging Level

```bash
# Set log level for debugging
ratchet --log-level debug serve
ratchet --log-level trace mcp-serve --transport sse
ratchet --log-level warn run-once --from-fs ./my-task --input-json '{}'

# Available levels: trace, debug, info, warn, error
```

## Common Workflows

### Development Workflow

```bash
# 1. Generate a new task
ratchet generate task --path ./weather-api --label "weather" --description "Weather API client"

# 2. Validate the task
ratchet validate --from-fs ./weather-api --fix

# 3. Test the task
ratchet test --from-fs ./weather-api

# 4. Run the task with test data
ratchet run-once --from-fs ./weather-api --input-json '{"city": "London", "units": "metric"}'

# 5. Record execution for debugging
ratchet run-once --from-fs ./weather-api --input-json '{"city": "Paris"}' --record ./debug/
```

### Production Deployment

```bash
# 1. Generate production configuration
ratchet config generate --type production --format yaml > /etc/ratchet/production.yaml

# 2. Validate configuration
ratchet config validate --config /etc/ratchet/production.yaml

# 3. Initialize task repository
ratchet repo init /var/lib/ratchet/tasks --name "Production Tasks"

# 4. Start the server
ratchet serve --config /etc/ratchet/production.yaml --log-level info
```

### MCP Integration Setup

```bash
# 1. Generate MCP configuration for Claude Desktop
ratchet generate mcpservers-json \
  --name "ratchet" \
  --command "ratchet" \
  --args "mcp-serve" \
  --transport "stdio" \
  --pretty > ~/.config/claude/mcpServers.json

# 2. Start MCP server
ratchet mcp-serve --log-level info

# 3. Verify connection (in another terminal)
ratchet console --transport "mcp"
```

### Repository Management

```bash
# 1. Check repository status
ratchet repo status --detailed

# 2. Verify repository accessibility
ratchet repo verify --list-tasks

# 3. Refresh metadata if needed
ratchet repo refresh-metadata --force

# 4. View configuration
ratchet config show --format yaml
```

## Tips and Best Practices

1. **Always validate tasks** before deployment using `ratchet validate --fix`
2. **Use recordings** for debugging complex task executions
3. **Check repository status** regularly to ensure task availability
4. **Use appropriate log levels** (debug for development, info for production)
5. **Generate configurations** rather than writing them manually
6. **Test MCP integration** using the console before connecting Claude Desktop
7. **Keep task repositories** organized with meaningful names and descriptions

## Troubleshooting

### Common Issues

```bash
# Task validation fails
ratchet validate --from-fs ./problematic-task --fix --log-level debug

# Server won't start
ratchet config validate --config ./config.yaml
ratchet serve --config ./config.yaml --log-level debug

# MCP connection issues
ratchet mcp-serve --transport stdio --log-level trace

# Repository access problems
ratchet repo verify --repository "problematic-repo" --detailed --log-level debug
```

### Debug Commands

```bash
# Show detailed configuration
ratchet config show --format json

# Validate with verbose output
ratchet validate --from-fs ./task --fix --log-level trace

# Test connectivity
ratchet repo verify --detailed --format json

# Record execution for analysis
ratchet run-once --from-fs ./task --input-json '{}' --record ./debug/ --log-level debug
```