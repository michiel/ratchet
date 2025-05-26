# Ratchet

Ratchet is a JavaScript task execution framework written in Rust. It allows you to define and execute JavaScript tasks with input/output validation using JSON Schema.

## Features

- Execute JavaScript code with input/output schema validation
- Isolated execution environment for JavaScript code
- Support for asynchronous operations using Tokio runtime
- HTTP fetch API for making web requests from JavaScript
- JSON schema validation for inputs and outputs
- **Recording functionality**: Capture HTTP requests in HAR format and execution logs
- Comprehensive tracing and debugging support
- Task validation and testing framework

## Project Structure

- `ratchet-cli`: Command-line interface for executing JavaScript tasks
- `ratchet-lib`: Core library containing the JavaScript execution engine
- `sample`: Example JavaScript tasks

## Requirements

- Rust 1.54.0 or higher
- Cargo

## Installation

Clone the repository and build the project:

```bash
git clone https://github.com/your-username/ratchet.git
cd ratchet
cargo build --release
```

The executable will be available at `target/release/ratchet`.

## Usage

### Running a JavaScript Task

1. Create a JavaScript task with input and output schemas:

```
my-task/
├── input.schema.json
├── main.js
├── output.schema.json
└── metadata.json
```

2. Run the task with input data:

```bash
ratchet run-once --from-fs my-task/ --input-json='{"num1": 5, "num2": 10}'
```

### CLI Commands

- **`run-once`**: Execute a single task
- **`validate`**: Validate task structure and syntax
- **`test`**: Run task tests

#### Command Options

- `--from-fs <PATH>`: Path to task directory or ZIP file
- `--input-json <JSON>`: JSON input data for the task
- `--log-level <LEVEL>`: Set logging level (trace, debug, info, warn, error)
- `--record <DIR>`: Record execution with HTTP calls and logs (see Recording section)

### Example JavaScript Task

Here's a simple addition task:

**main.js**:
```javascript
function(input) {
  const { num1, num2 } = input;
  
  if (typeof num1 !== 'number' || typeof num2 !== 'number') {
    throw new Error('num1 and num2 must be numbers');
  }
  
  return {
    sum: num1 + num2
  };
}
```

**input.schema.json**:
```json
{
  "type": "object",
  "properties": {
    "num1": { "type": "number" },
    "num2": { "type": "number" }
  },
  "required": ["num1", "num2"]
}
```

**output.schema.json**:
```json
{
  "type": "object",
  "properties": {
    "sum": { "type": "number" }
  },
  "required": ["sum"]
}
```

### Making HTTP Requests

Ratchet provides a fetch API similar to the browser's fetch API:

```javascript
function(input) {
  const response = fetch('https://api.example.com/data', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    }
  }, { key: 'value' });
  
  return response.body;
}
```

## Recording Functionality

Ratchet provides powerful recording capabilities to capture and analyze task execution, including all HTTP requests and detailed execution logs.

### Overview

The `--record` flag creates a timestamped session directory containing:
- **HTTP Archive (HAR)** files with all fetch API calls
- **Complete tracing logs** with execution details
- **Structured data** for debugging and analysis

### Usage

```bash
# Record task execution with HTTP calls and logs
ratchet run-once --from-fs my-task/ \
  --input-json='{"city":"Berlin"}' \
  --record /path/to/recordings
```

This creates a directory structure like:
```
/path/to/recordings/ratchet_session_20250526_143022/
├── requests.har     # HTTP Archive with all fetch calls
└── tracing.log      # Complete execution tracing
```

### Generated Files

#### `requests.har`
Standard HTTP Archive format containing:
- **Request details**: Method, URL, headers, body
- **Response data**: Status codes, headers, response body
- **Timing information**: Duration, connection timing
- **Metadata**: Timestamps, browser info

Example HAR entry:
```json
{
  "log": {
    "version": "1.2",
    "creator": { "name": "Ratchet", "version": "0.1.0" },
    "entries": [
      {
        "startedDateTime": "2025-05-26T14:30:22.123Z",
        "time": 245,
        "request": {
          "method": "POST",
          "url": "https://api.example.com/oauth/token",
          "headers": [
            { "name": "Content-Type", "value": "application/x-www-form-urlencoded" }
          ],
          "postData": {
            "mimeType": "application/x-www-form-urlencoded",
            "text": "grant_type=client_credentials&client_id=..."
          }
        },
        "response": {
          "status": 200,
          "statusText": "OK",
          "headers": [
            { "name": "Content-Type", "value": "application/json" }
          ],
          "content": {
            "mimeType": "application/json",
            "text": "{\"access_token\":\"...\",\"token_type\":\"Bearer\"}"
          }
        },
        "timings": { "wait": 245, "receive": 0 }
      }
    ]
  }
}
```

#### `tracing.log`
Complete execution log with:
- **Timestamped events** with microsecond precision
- **Log levels**: TRACE, DEBUG, INFO, WARN, ERROR
- **Module context**: Which component generated each log
- **Execution flow**: Task loading, validation, execution steps

Example log output:
```
2025-05-26T14:30:22.123456Z  INFO ratchet: Ratchet CLI starting
2025-05-26T14:30:22.123789Z  INFO ratchet: Loading task from: my-task/
2025-05-26T14:30:22.124123Z DEBUG ratchet_lib::task: Loading JavaScript content
2025-05-26T14:30:22.124456Z  INFO ratchet_lib::js_executor: Executing task: My Task
2025-05-26T14:30:22.124789Z DEBUG ratchet_lib::http: Making HTTP request to: https://api.example.com
2025-05-26T14:30:22.370123Z  INFO ratchet_lib::recording: Recorded HTTP request POST https://api.example.com -> 200
```

### Use Cases

#### 1. **API Integration Debugging**
```bash
# Debug OAuth flow with detailed HTTP capture
ratchet run-once --from-fs oauth-task/ \
  --input-json='{"client_id":"...", "client_secret":"..."}' \
  --record ./oauth-debug \
  --log-level debug
```

#### 2. **Performance Analysis**
```bash
# Analyze API response times and execution flow
ratchet run-once --from-fs api-task/ \
  --input-json='{"endpoint":"production"}' \
  --record ./performance-analysis \
  --log-level trace
```

#### 3. **CI/CD Integration**
```bash
# Record test execution for CI/CD artifacts
ratchet run-once --from-fs integration-test/ \
  --input-json='{"environment":"staging"}' \
  --record ./ci-artifacts/test-run-${BUILD_ID}
```

#### 4. **API Documentation**
```bash
# Generate API interaction examples
ratchet run-once --from-fs workflow-example/ \
  --input-json='{"scenario":"demo"}' \
  --record ./api-examples
```

### Environment Variable Support

Control logging via environment variables:
```bash
# Set global log level
RUST_LOG=debug ratchet run-once --from-fs my-task/ --record ./logs

# Module-specific logging
RUST_LOG=ratchet_lib::http=trace ratchet run-once --from-fs my-task/ --record ./logs
```

### HAR File Analysis

HAR files can be:
- **Imported into browser dev tools** for visual analysis
- **Processed with HAR analysis tools** like HAR Analyzer
- **Parsed programmatically** for automated testing
- **Used for API documentation** generation

### Recording Best Practices

1. **Use descriptive recording directories**:
   ```bash
   --record ./recordings/oauth-flow-$(date +%Y%m%d)
   ```

2. **Combine with appropriate log levels**:
   ```bash
   --log-level debug --record ./debug-session
   ```

3. **Archive recordings for later analysis**:
   ```bash
   tar -czf session-archive.tar.gz ./recordings/ratchet_session_*
   ```

4. **Clean up old recordings periodically**:
   ```bash
   find ./recordings -name "ratchet_session_*" -mtime +30 -exec rm -rf {} \;
   ```

## Task Structure

A complete task directory includes:

```
my-task/
├── metadata.json        # Task metadata and identification
├── main.js             # JavaScript implementation
├── input.schema.json   # Input validation schema
├── output.schema.json  # Output validation schema
└── tests/             # Test cases (optional)
    ├── test-001.json
    ├── test-002.json
    └── test-003-with-mock.json
```

### metadata.json
```json
{
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "version": "1.0.0",
  "label": "My Task",
  "description": "Description of what this task does"
}
```

### Test Files
Test files in the `tests/` directory contain:
```json
{
  "input": {
    "city": "Berlin",
    "units": "metric"
  },
  "expected_output": {
    "temperature": 22.5,
    "description": "clear sky"
  },
  "mock": {
    "http": {
      "url": "api.openweathermap.org",
      "method": "GET",
      "response": {
        "status": 200,
        "body": { "main": { "temp": 22.5 } }
      }
    }
  }
}
```

## Testing

### Running Task Tests
```bash
# Run all tests for a task
ratchet test --from-fs my-task/

# Validate task structure
ratchet validate --from-fs my-task/
```

### Test Features
- **Automatic test discovery** from `tests/` directory
- **Mock HTTP responses** for external API testing
- **Schema validation** of inputs and outputs
- **Detailed test reporting** with pass/fail status

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Adding a New Feature

1. Implement the feature in the `ratchet-lib` crate
2. Add tests for the new feature
3. Expose the feature through the CLI if necessary

## License

[MIT License](LICENSE)