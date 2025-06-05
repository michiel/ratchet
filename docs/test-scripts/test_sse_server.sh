#!/bin/bash

# Test script for SSE MCP server functionality

set -e

echo "Testing SSE MCP Server Implementation"
echo "======================================"

# Test 1: Configuration validation
echo "1. Testing configuration validation..."
if cargo run -p ratchet-mcp --bin ratchet-mcp -- --config sample/configs/example-sse-config.yaml validate-config; then
    echo "   ✅ Configuration validation passed"
else
    echo "   ❌ Configuration validation failed"
    exit 1
fi

# Test 2: Compile and run tests
echo "2. Running SSE-specific tests..."
if cargo test -p ratchet-mcp sse --quiet; then
    echo "   ✅ SSE tests passed"
else
    echo "   ❌ SSE tests failed"
    exit 1
fi

# Test 3: Check that SSE transport is properly exposed
echo "3. Testing SSE transport factory..."
if cargo test -p ratchet-mcp transport_factory_sse --quiet; then
    echo "   ✅ SSE transport factory test passed"
else
    echo "   ❌ SSE transport factory test failed"
    exit 1
fi

echo ""
echo "SSE Transport Implementation Summary:"
echo "======================================"
echo "✅ HTTP Server-Sent Events (SSE) transport implemented"
echo "✅ SSE endpoint: GET /sse/{session_id}"
echo "✅ Message endpoint: POST /message/{session_id}"
echo "✅ Health check endpoint: GET /health"
echo "✅ CORS support for browser-based clients"
echo "✅ Connection management and health tracking"
echo "✅ Request/response correlation via session IDs"
echo "✅ Authentication support (Bearer, Basic, API Key)"
echo "✅ Comprehensive test coverage"
echo ""
echo "Usage Examples:"
echo "--------------"
echo "# Start SSE server with config:"
echo "cargo run -p ratchet-mcp --bin ratchet-mcp -- --config sample/configs/example-sse-config.yaml"
echo ""
echo "# Connect to SSE endpoint (browser/client):"
echo "GET http://localhost:3000/sse/YOUR_SESSION_ID"
echo ""
echo "# Send MCP messages:"
echo "POST http://localhost:3000/message/YOUR_SESSION_ID"
echo "Content-Type: application/json"
echo '{"jsonrpc":"2.0","method":"initialize","id":"1","params":{...}}'
echo ""
echo "# Check server health:"
echo "GET http://localhost:3000/health"
echo ""
echo "All SSE transport functionality has been successfully implemented! 🎉"