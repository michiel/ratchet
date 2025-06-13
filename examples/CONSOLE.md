# Ratchet Console

The Ratchet Console is an interactive REPL (Read-Eval-Print Loop) interface for administering and managing Ratchet instances.

## Features

- **Interactive REPL**: Command-line interface with history and completion
- **Administrative Commands**: Comprehensive set of commands for system management
- **Variable Support**: Set and use variables in commands
- **Script Execution**: Run console commands from script files
- **Mock Implementation**: Currently uses mock data for demonstration

## Getting Started

### Start the Console

```bash
# Start console with default settings
ratchet console

# Connect to remote server
ratchet console --host remote-server.com --port 8090

# Use authentication token
ratchet console --auth-token your-token-here

# Execute a script on startup
ratchet console --script examples/console-demo.ratchet
```

### Basic Commands

#### Built-in Console Commands

```bash
help                    # Show available commands
exit, quit             # Exit the console
clear                  # Clear screen
history                # Show command history
set var = value        # Set a variable
unset var              # Remove a variable
vars                   # Show all variables
source file.ratchet    # Execute script file
connect                # Connect to server
disconnect             # Disconnect from server
```

#### Repository Management

```bash
repo list                              # List repositories
repo add myrepo /path/to/repo         # Add repository
repo remove myrepo                    # Remove repository
repo refresh myrepo                   # Refresh repository
repo status                           # Show repository status
repo verify                           # Verify repositories
```

#### Task Management

```bash
task list                             # List all tasks
task show task-001                    # Show task details
task enable task-001                  # Enable a task
task disable task-001                 # Disable a task
task execute task-001 '{"key":"val"}' # Execute a task
```

#### Execution Management

```bash
execution list                        # List executions
execution show exec-001               # Show execution details
execution cancel exec-001             # Cancel execution
execution retry exec-001              # Retry execution
execution logs exec-001               # View execution logs
```

#### Job Queue Management

```bash
job list                              # List jobs in queue
job show job-001                      # Show job details
job cancel job-001                    # Cancel a job
job retry job-001                     # Retry a failed job
job clear                             # Clear completed jobs
job pause                             # Pause job processing
job resume                            # Resume job processing
```

#### Server Management

```bash
server status                         # Show server status
server workers                        # Show worker status
server metrics                        # Show server metrics
server config                         # Show configuration
server reload                         # Reload configuration
```

#### Database Management

```bash
db status                             # Show migration status
db migrate                            # Run migrations
db migrate --dry-run                  # Preview migrations
db stats                              # Show database statistics
db dump /path/to/backup.sql          # Dump database
```

#### Monitoring

```bash
health                                # Check system health
stats                                 # Show system statistics
monitor                               # Start real-time monitoring
```

### Variables

You can set and use variables in your commands:

```bash
# Set variables
set task_id = "my-task-001"
set input = '{"num1": 42, "num2": 58}'

# Use variables (prefix with $)
task show $task_id
task execute $task_id $input

# Show all variables
vars

# Remove a variable
unset task_id
```

### Script Files

Create script files with `.ratchet` extension:

```bash
# example.ratchet
echo "Starting maintenance..."
repo refresh --force
db migrate
job clear --older-than 7d
echo "Maintenance completed"
```

Execute scripts:

```bash
# In console
source example.ratchet

# On startup
ratchet console --script example.ratchet
```

## Command Reference

### Console Built-ins

| Command | Description |
|---------|-------------|
| `help` | Show available commands |
| `exit`, `quit` | Exit the console |
| `clear` | Clear the screen |
| `history` | Show command history |
| `set <var> = <value>` | Set a variable |
| `unset <var>` | Remove a variable |
| `vars` | Show all variables |
| `source <file>` | Execute script file |
| `connect` | Connect to server |
| `disconnect` | Disconnect from server |

### Repository Commands

| Command | Description |
|---------|-------------|
| `repo list` | List all repositories |
| `repo add <name> <uri>` | Add a new repository |
| `repo remove <name>` | Remove a repository |
| `repo refresh [name]` | Refresh repository metadata |
| `repo status` | Show repository status |
| `repo verify` | Verify repository accessibility |

### Task Commands

| Command | Description |
|---------|-------------|
| `task list` | List all tasks |
| `task show <id>` | Show task details |
| `task enable <id>` | Enable a task |
| `task disable <id>` | Disable a task |
| `task execute <id> [input]` | Execute a task |

### Execution Commands

| Command | Description |
|---------|-------------|
| `execution list` | List executions |
| `execution show <id>` | Show execution details |
| `execution cancel <id>` | Cancel execution |
| `execution retry <id>` | Retry execution |
| `execution logs <id>` | View execution logs |

### Job Commands

| Command | Description |
|---------|-------------|
| `job list` | List jobs in queue |
| `job show <id>` | Show job details |
| `job cancel <id>` | Cancel a job |
| `job retry <id>` | Retry a failed job |
| `job clear` | Clear completed jobs |
| `job pause` | Pause job processing |
| `job resume` | Resume job processing |

### Server Commands

| Command | Description |
|---------|-------------|
| `server status` | Show server status |
| `server workers` | Show worker information |
| `server metrics` | Show server metrics |
| `server config` | Show configuration |
| `server reload` | Reload configuration |

### Database Commands

| Command | Description |
|---------|-------------|
| `db status` | Show migration status |
| `db migrate` | Run database migrations |
| `db stats` | Show database statistics |
| `db dump <file>` | Dump database to file |

### Monitoring Commands

| Command | Description |
|---------|-------------|
| `health` | Check system health |
| `stats` | Show system statistics |
| `monitor` | Start real-time monitoring |

## Current Status

This is the initial implementation of the Ratchet Console with:

✅ **Completed Features:**
- Interactive REPL with rustyline
- Command parsing and routing
- Mock command implementations
- Variable support
- Script execution
- Help system
- Output formatting

🚧 **In Progress:**
- MCP client integration for real server communication
- Enhanced security and authentication
- Extended administrative tools

📋 **Planned Features:**
- Real-time monitoring dashboard
- Command completion
- Syntax highlighting
- Plugin system for custom commands
- Web-based console interface

## Contributing

The console implementation is modular and extensible. To add new commands:

1. Add the command structure to `parser.rs`
2. Implement the command logic in `executor.rs` 
3. Add any specific command modules to `commands/`
4. Update the help system and documentation

For MCP integration, extend the `executor.rs` to use real MCP client calls instead of mock responses.