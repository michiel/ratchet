# Design Proposal: Ratchet Console Command

## Overview

This proposal outlines the design for a new `ratchet console` command that provides an interactive REPL (Read-Eval-Print Loop) terminal console for administering and interacting with a running Ratchet instance via MCP (Model Context Protocol).

## Problem Statement

Currently, Ratchet provides CLI commands for one-time operations and a server mode, but lacks an interactive administrative interface. Administrators need:

1. **Interactive Administration**: Real-time administration without restarting commands
2. **MCP Integration**: Leverage existing MCP infrastructure for remote operations
3. **Rich Command Set**: Access to all Ratchet functionality through console commands
4. **Live Monitoring**: Real-time status updates and system monitoring
5. **Scriptable Interface**: Console commands that can be scripted or automated

## Proposed Solution

### Command Structure

```bash
ratchet console [OPTIONS]
```

**Options:**
- `--config <PATH>`: Path to configuration file
- `--connect <URL>`: Connect to remote Ratchet MCP server (default: local)
- `--transport <TYPE>`: Transport type (stdio, sse, websocket) [default: sse]
- `--host <HOST>`: Host to connect to [default: 127.0.0.1]
- `--port <PORT>`: Port to connect to [default: 8090]
- `--auth-token <TOKEN>`: Authentication token for remote connections
- `--history-file <PATH>`: Custom history file location
- `--script <PATH>`: Execute script file on startup

### Console Interface

The console will provide a rich interactive interface:

```
Ratchet Console v0.6.0
Connected to: ratchet-server@localhost:8090
Type 'help' for available commands, 'exit' to quit

ratchet> 
```

**Features:**
- Command completion and history (using `rustyline`)
- Syntax highlighting for JSON inputs
- Multi-line command support
- Built-in help system with command documentation
- Status indicators (connection state, server health)

## Console Commands

### Core Management Commands

#### Repository Management
```bash
# List configured repositories
repo list [--format table|json|yaml]

# Add a new repository
repo add <name> <uri> [--type git|filesystem] [--branch <branch>] [--auth <auth-name>]

# Remove a repository
repo remove <name> [--force]

# Refresh repository metadata
repo refresh <name> [--force]

# Show repository status
repo status [<name>] [--detailed]

# Verify repository accessibility
repo verify [<name>] [--detailed] [--list-tasks]
```

#### Task Management
```bash
# List available tasks
task list [--repository <name>] [--enabled] [--format table|json|yaml]

# Show task details
task show <task-id> [--include-schema]

# Enable/disable tasks
task enable <task-id>
task disable <task-id>

# Validate a task
task validate <task-id>

# Test a task
task test <task-id> --input '<json>'

# Execute a task
task execute <task-id> --input '<json>' [--webhook <url>] [--file <path>]
```

#### Execution Management
```bash
# List executions
execution list [--task <task-id>] [--status <status>] [--limit <n>]

# Show execution details
execution show <execution-id>

# Cancel a running execution
execution cancel <execution-id>

# Retry a failed execution
execution retry <execution-id>

# Stream execution logs
execution logs <execution-id> [--follow]
```

#### Job Queue Management
```bash
# List jobs in queue
job list [--status <status>] [--priority <priority>] [--format table|json|yaml]

# Show job details
job show <job-id>

# Cancel a queued job
job cancel <job-id>

# Retry a failed job
job retry <job-id>

# Clear completed jobs
job clear [--older-than <duration>]

# Pause/resume job processing
job pause
job resume
```

#### Database Management
```bash
# Run database migrations
db migrate [--dry-run]

# Show migration status
db status

# Dump database to file
db dump <file-path> [--format sql|json]

# Import database from file
db import <file-path> [--format sql|json] [--force]

# Vacuum database (cleanup)
db vacuum

# Show database statistics
db stats
```

#### Server Management
```bash
# Show server health and status
server status [--detailed]

# Show server configuration
server config [--format yaml|json]

# Reload configuration
server reload [--config <path>]

# Show connected workers
server workers

# Show server metrics
server metrics [--format table|json|prometheus]

# Graceful shutdown
server shutdown [--timeout <seconds>]
```

#### Monitoring Commands
```bash
# Real-time system monitoring
monitor [--interval <seconds>] [--metrics tasks,jobs,workers,db]

# Show system statistics
stats [--format table|json|yaml]

# Show recent activity
activity [--limit <n>] [--follow]

# Health check
health [--detailed]
```

### Interactive Features

#### Multi-line Input
```bash
ratchet> task execute my-task --input '{
...>   "param1": "value1",
...>   "param2": {
...>     "nested": "value"
...>   }
...> }'
```

#### Command History and Search
```bash
# Search command history
ratchet> !task execute
ratchet> !!  # Repeat last command
ratchet> !-2 # Execute command 2 steps back
```

#### Variable Support
```bash
ratchet> set task_id = "my-task-123"
ratchet> task show $task_id
ratchet> set input = '{"param1": "value1"}'
ratchet> task execute $task_id --input $input
```

#### Scripting Support
```bash
# Execute a script file
ratchet> source script.ratchet

# Example script content:
echo "Starting maintenance tasks..."
repo refresh --force
db migrate
job clear --older-than 7d
echo "Maintenance completed"
```

## Technical Architecture

### Components

```
┌─────────────────────────────────────┐
│         Ratchet Console             │
│  ┌─────────────────────────────────┐ │
│  │        REPL Interface           │ │
│  │  - rustyline for readline      │ │
│  │  - Command parsing             │ │
│  │  - Syntax highlighting         │ │
│  │  - History management          │ │
│  └─────────────────────────────────┘ │
│  ┌─────────────────────────────────┐ │
│  │     Command Dispatcher          │ │
│  │  - Command routing             │ │
│  │  - Parameter validation        │ │
│  │  - Output formatting           │ │
│  └─────────────────────────────────┘ │
│  ┌─────────────────────────────────┐ │
│  │       MCP Client                │ │
│  │  - Connection management       │ │
│  │  - Transport abstraction       │ │
│  │  - Authentication              │ │
│  └─────────────────────────────────┘ │
└─────────────────────────────────────┘
                  │
                  │ MCP Protocol
                  │
┌─────────────────────────────────────┐
│         Ratchet Server              │
│  ┌─────────────────────────────────┐ │
│  │       MCP Server                │ │
│  │  - Administrative tools        │ │
│  │  - Extended capabilities       │ │
│  │  - Security & permissions      │ │
│  └─────────────────────────────────┘ │
│  ┌─────────────────────────────────┐ │
│  │    Ratchet Core Services        │ │
│  │  - Task management             │ │
│  │  - Job queue                   │ │
│  │  - Database operations         │ │
│  └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

### Implementation Structure

```
ratchet-cli/src/commands/
├── console/
│   ├── mod.rs              # Console command entry point
│   ├── repl.rs             # REPL implementation
│   ├── parser.rs           # Command parsing
│   ├── executor.rs         # Command execution
│   ├── formatter.rs        # Output formatting
│   └── commands/
│       ├── mod.rs
│       ├── repo.rs         # Repository commands
│       ├── task.rs         # Task commands
│       ├── execution.rs    # Execution commands
│       ├── job.rs          # Job commands
│       ├── database.rs     # Database commands
│       ├── server.rs       # Server commands
│       └── monitor.rs      # Monitoring commands

ratchet-mcp/src/server/
├── admin_tools.rs          # Administrative MCP tools
├── permissions.rs          # Enhanced permissions for admin
└── capabilities.rs         # Extended capabilities
```

### MCP Integration

#### New Administrative Tools

The MCP server will be extended with administrative tools:

```json
{
  "tools": {
    "repo_add": {
      "description": "Add a new repository to the configuration",
      "parameters": {
        "name": "string",
        "uri": "string", 
        "type": "git|filesystem",
        "branch": "string (optional)",
        "auth_name": "string (optional)"
      }
    },
    "repo_refresh": {
      "description": "Refresh repository metadata",
      "parameters": {
        "name": "string",
        "force": "boolean (optional)"
      }
    },
    "db_migrate": {
      "description": "Run database migrations",
      "parameters": {
        "dry_run": "boolean (optional)"
      }
    },
    "job_clear": {
      "description": "Clear completed jobs",
      "parameters": {
        "older_than": "string (optional, duration)"
      }
    }
  }
}
```

#### Enhanced Security

- **Permission-based access**: Different permission levels for read-only vs administrative operations
- **Authentication tokens**: Support for token-based authentication for remote connections
- **Audit logging**: All administrative operations logged with user context
- **Rate limiting**: Protection against command flooding

## Benefits

### For Developers
- **Interactive Development**: Test and debug tasks interactively
- **Real-time Feedback**: Immediate results and error messages
- **Scriptable Operations**: Automate common development tasks

### For System Administrators
- **Live Administration**: Manage running systems without restarts
- **Comprehensive Monitoring**: Real-time system status and metrics
- **Maintenance Operations**: Database management and cleanup tasks
- **Remote Management**: Manage distributed Ratchet instances

### For DevOps Teams
- **Automation Support**: Script complex administrative workflows
- **Integration Ready**: Easy integration with existing automation tools
- **Consistent Interface**: Unified interface for all Ratchet operations

## Implementation Status

### Phase 1: Core Infrastructure ✅ COMPLETED
- [x] Basic REPL implementation with rustyline
- [x] Command parsing and routing framework
- [x] Basic help system and command completion
- [x] Mock MCP client integration (foundation for real integration)

### Phase 2: Essential Commands ✅ COMPLETED
- [x] Repository management commands (add, remove, refresh, status)
- [x] Task management commands (list, show, enable/disable)
- [x] Basic execution commands (list, show, execute)
- [x] Server status and health commands

### Phase 3: Advanced Features ✅ COMPLETED
- [x] Database management commands (migrate, dump, stats)
- [x] Job queue management (list, cancel, retry, clear)
- [x] Monitoring and metrics commands
- [x] Variable support and scripting capabilities

### Phase 4: Polish & Security 🚧 IN PROGRESS
- [x] Documentation and examples
- [ ] Real MCP client integration (currently using mock responses)
- [ ] Enhanced security and permissions
- [ ] Authentication for remote connections
- [ ] Audit logging for administrative operations
- [ ] Performance optimizations and error handling

## Current Implementation

The console command has been successfully implemented with:

### ✅ Working Features
- **Interactive REPL**: Full rustyline integration with history and command editing
- **Command Parsing**: Robust parsing of commands, flags, and JSON inputs
- **Mock Execution**: Complete mock implementation of all administrative commands
- **Variable System**: Set, use, and manage variables in the console
- **Script Support**: Execute script files with `.ratchet` extension
- **Output Formatting**: Rich formatted output with tables, JSON, and colored text
- **Help System**: Comprehensive help for all commands
- **CLI Integration**: Fully integrated with the main `ratchet` CLI

### 🚧 Next Steps
- **MCP Integration**: Replace mock executor with real MCP client calls
- **Security Layer**: Add authentication and permission checks
- **Real-time Features**: Implement streaming responses for monitoring commands

## Dependencies

### New Dependencies
- `rustyline`: For readline functionality and command history
- `clap_complete`: For command completion support
- `syntect`: For syntax highlighting (optional)
- `tokio-stream`: For streaming responses

### Configuration Changes

New console configuration section in `ratchet.yaml`:

```yaml
console:
  mcp:
    # MCP server settings for console connections
    bind_address: "127.0.0.1"
    port: 8090
    transport: "sse"
    
  security:
    # Console-specific security settings
    require_auth: true
    admin_permissions: ["repo:write", "db:admin", "server:admin"]
    read_permissions: ["repo:read", "task:read", "execution:read"]
    
  interface:
    # Console interface settings
    history_size: 1000
    completion_enabled: true
    color_enabled: true
```

## Risk Assessment

### Technical Risks
- **MCP Protocol Limitations**: May need protocol extensions for complex operations
- **Connection Stability**: Network issues could interrupt long-running operations
- **Memory Usage**: Command history and state management could consume memory

### Mitigation Strategies
- **Graceful Degradation**: Fall back to basic functionality if advanced features fail
- **Connection Recovery**: Automatic reconnection with session state preservation
- **Resource Management**: Configurable limits and cleanup policies

## Success Metrics

- **Adoption Rate**: Percentage of Ratchet users utilizing the console
- **Command Usage**: Frequency of different command categories
- **Error Rates**: Success/failure rates for console operations
- **Performance**: Response times for common operations
- **User Feedback**: Satisfaction scores and feature requests

## Future Enhancements

- **Web-based Console**: Browser-based version of the console interface
- **Multi-server Management**: Connect to and manage multiple Ratchet instances
- **Plugin System**: Allow custom commands and extensions
- **Visual Dashboard**: Integration with monitoring dashboards
- **AI Assistant**: Natural language command interpretation and suggestions

---

This design proposal provides a comprehensive plan for implementing the `ratchet console` command, offering a powerful interactive interface for Ratchet administration while leveraging the existing MCP infrastructure.