#!/bin/bash
# Test MCP server stdio communication

# Initialize connection request
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test-client","version":"1.0.0"}}}' | ./target/release/ratchet-mcp serve --config sample/configs/test-config.yaml --transport stdio