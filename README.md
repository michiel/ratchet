# Ratchet

Ratchet is a JavaScript task execution framework written in Rust. It allows you to define and execute JavaScript tasks with input/output validation using JSON Schema.

## Features

- Execute JavaScript code with input/output schema validation
- Isolated execution environment for JavaScript code
- Support for asynchronous operations using Tokio runtime
- HTTP fetch API for making web requests from JavaScript
- JSON schema validation for inputs and outputs

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
└── output.schema.json
```

2. Run the task with input data:

```bash
ratchet run my-task/main.js --input '{"num1": 5, "num2": 10}'
```

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