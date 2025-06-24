#!/bin/bash

# Test script to verify MCP endpoint responses
set -e

echo "Testing MCP endpoint JSON-RPC responses..."

# Test 1: Test initialization request
echo -e "\n1. Testing initialization request:"
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}' \
  -w "\nStatus: %{http_code}\n"

# Test 2: Test invalid request
echo -e "\n2. Testing invalid request:"
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"invalid": "request"}' \
  -w "\nStatus: %{http_code}\n"

# Test 3: Test health endpoint
echo -e "\n3. Testing health endpoint:"
curl -X GET http://localhost:8080/mcp/health \
  -H "Content-Type: application/json" \
  -w "\nStatus: %{http_code}\n"

# Test 4: Test GET request (SSE)
echo -e "\n4. Testing GET request (SSE):"
timeout 3s curl -X GET http://localhost:8080/mcp \
  -H "Accept: text/event-stream" \
  -w "\nStatus: %{http_code}\n" || echo "SSE stream timeout (expected)"

echo -e "\nMCP endpoint tests completed!"