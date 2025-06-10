#!/bin/bash

# Ratchet Server Setup Script
# This script sets up directories and starts the Ratchet server with full configuration

set -e

echo "🚀 Setting up Ratchet Server..."

# Create necessary directories
echo "📁 Creating directories..."
mkdir -p /tmp/ratchet/logs
mkdir -p /tmp/ratchet/outputs
mkdir -p /tmp/ratchet/tasks
mkdir -p /tmp/ratchet/data

# Set permissions
chmod 755 /tmp/ratchet
chmod 755 /tmp/ratchet/logs
chmod 755 /tmp/ratchet/outputs
chmod 755 /tmp/ratchet/tasks
chmod 755 /tmp/ratchet/data

echo "📝 Creating sample task in local tasks directory..."

# Create a sample task in the local tasks directory
cat > /tmp/ratchet/tasks/hello-world/metadata.json << 'EOF'
{
  "uuid": "550e8400-e29b-41d4-a716-446655440001",
  "version": "1.0.0",
  "label": "Hello World",
  "description": "A simple hello world task",
  "author": "Ratchet Server",
  "tags": ["demo", "simple"],
  "timeout": 30000,
  "memory_limit": "64MB"
}
EOF

mkdir -p /tmp/ratchet/tasks/hello-world

cat > /tmp/ratchet/tasks/hello-world/input.schema.json << 'EOF'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "name": {
      "type": "string",
      "description": "Name to greet",
      "default": "World"
    }
  },
  "required": []
}
EOF

cat > /tmp/ratchet/tasks/hello-world/output.schema.json << 'EOF'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "message": {
      "type": "string",
      "description": "The greeting message"
    },
    "timestamp": {
      "type": "string",
      "description": "When the greeting was generated"
    }
  },
  "required": ["message", "timestamp"]
}
EOF

cat > /tmp/ratchet/tasks/hello-world/main.js << 'EOF'
(function(input, context) {
    const name = input.name || "World";
    
    return {
        message: `Hello, ${name}!`,
        timestamp: new Date().toISOString()
    };
})
EOF

mkdir -p /tmp/ratchet/tasks/hello-world/tests

cat > /tmp/ratchet/tasks/hello-world/tests/test-001.json << 'EOF'
{
  "name": "Basic hello test",
  "description": "Test basic hello functionality",
  "input": {
    "name": "Alice"
  },
  "expected_output": {
    "message": "Hello, Alice!"
  }
}
EOF

echo "✅ Sample task created at /tmp/ratchet/tasks/hello-world"

# Check if ratchet command is available
if ! command -v ratchet &> /dev/null; then
    echo "❌ Error: 'ratchet' command not found in PATH"
    echo "Please ensure Ratchet is installed and in your PATH"
    echo "You can build it from source or install from releases"
    exit 1
fi

echo "🔧 Starting Ratchet server with full configuration..."
echo ""
echo "Server will be available at:"
echo "  - HTTP API (REST/GraphQL): http://localhost:8080"
echo "  - MCP SSE Server:          http://localhost:8090"
echo "  - Logs:                    /tmp/ratchet/logs/ratchet.log"
echo "  - Task outputs:            /tmp/ratchet/outputs/"
echo ""
echo "Press Ctrl+C to stop the server"
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/ratchet-full-server-config.yaml"

# Check if config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "❌ Error: Configuration file not found at $CONFIG_FILE"
    exit 1
fi

# Start the Ratchet server
exec ratchet serve --config="$CONFIG_FILE"