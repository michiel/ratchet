#!/bin/bash

# Ratchet Server Test Script
# Tests all server endpoints and functionality

set -e

echo "🧪 Testing Ratchet Server..."
echo ""

# Configuration
HTTP_PORT=8080
MCP_PORT=8090
BASE_URL="http://localhost"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test function
test_endpoint() {
    local name="$1"
    local url="$2"
    local expected_status="$3"
    
    echo -n "Testing $name... "
    
    if response=$(curl -s -w "%{http_code}" -o /tmp/test_response "$url" 2>/dev/null); then
        status_code="${response: -3}"
        if [ "$status_code" = "$expected_status" ]; then
            echo -e "${GREEN}✓ PASS${NC} (HTTP $status_code)"
            return 0
        else
            echo -e "${RED}✗ FAIL${NC} (Expected $expected_status, got $status_code)"
            return 1
        fi
    else
        echo -e "${RED}✗ FAIL${NC} (Connection failed)"
        return 1
    fi
}

# Test JSON endpoint with jq validation
test_json_endpoint() {
    local name="$1"
    local url="$2"
    local jq_filter="$3"
    
    echo -n "Testing $name... "
    
    if response=$(curl -s "$url" 2>/dev/null); then
        if echo "$response" | jq -e "$jq_filter" >/dev/null 2>&1; then
            echo -e "${GREEN}✓ PASS${NC} (Valid JSON response)"
            return 0
        else
            echo -e "${RED}✗ FAIL${NC} (Invalid JSON or missing expected data)"
            echo "Response: $response"
            return 1
        fi
    else
        echo -e "${RED}✗ FAIL${NC} (Connection failed)"
        return 1
    fi
}

# Test POST endpoint
test_post_endpoint() {
    local name="$1"
    local url="$2"
    local data="$3"
    local expected_status="$4"
    
    echo -n "Testing $name... "
    
    if response=$(curl -s -w "%{http_code}" -o /tmp/test_response \
                      -X POST \
                      -H "Content-Type: application/json" \
                      -d "$data" \
                      "$url" 2>/dev/null); then
        status_code="${response: -3}"
        if [ "$status_code" = "$expected_status" ]; then
            echo -e "${GREEN}✓ PASS${NC} (HTTP $status_code)"
            return 0
        else
            echo -e "${RED}✗ FAIL${NC} (Expected $expected_status, got $status_code)"
            cat /tmp/test_response
            return 1
        fi
    else
        echo -e "${RED}✗ FAIL${NC} (Connection failed)"
        return 1
    fi
}

echo "🌐 Testing HTTP Server (Port $HTTP_PORT)"
echo "----------------------------------------"

# Test basic health check
test_endpoint "Health Check" "$BASE_URL:$HTTP_PORT/health" "200"

# Test REST API endpoints
test_json_endpoint "REST API - Tasks List" "$BASE_URL:$HTTP_PORT/api/v1/tasks" "type == \"array\""

# Test GraphQL endpoint
test_post_endpoint "GraphQL Query" "$BASE_URL:$HTTP_PORT/graphql" \
    '{"query": "{ tasks { id name } }"}' "200"

echo ""
echo "🔌 Testing MCP Server (Port $MCP_PORT)"
echo "---------------------------------------"

# Test MCP health check
test_endpoint "MCP Health Check" "$BASE_URL:$MCP_PORT/health" "200"

# Test if we can reach the SSE endpoint (should return 400 without session ID)
test_endpoint "MCP SSE Endpoint" "$BASE_URL:$MCP_PORT/sse/" "404"

echo ""
echo "📁 Testing File System"
echo "----------------------"

# Check if directories exist
check_directory() {
    local name="$1"
    local path="$2"
    
    echo -n "Checking $name... "
    if [ -d "$path" ]; then
        echo -e "${GREEN}✓ EXISTS${NC}"
        return 0
    else
        echo -e "${RED}✗ MISSING${NC}"
        return 1
    fi
}

check_directory "Logs Directory" "/tmp/ratchet/logs"
check_directory "Tasks Directory" "/tmp/ratchet/tasks"
check_directory "Outputs Directory" "/tmp/ratchet/outputs"

# Check if sample task exists
echo -n "Checking Sample Task... "
if [ -f "/tmp/ratchet/tasks/hello-world/main.js" ]; then
    echo -e "${GREEN}✓ EXISTS${NC}"
else
    echo -e "${RED}✗ MISSING${NC}"
fi

echo ""
echo "📝 Testing Log File"
echo "-------------------"

LOG_FILE="/tmp/ratchet/logs/ratchet.log"
echo -n "Checking Log File... "
if [ -f "$LOG_FILE" ]; then
    echo -e "${GREEN}✓ EXISTS${NC}"
    
    # Check if log file has recent entries (last 5 minutes)
    echo -n "Checking Recent Log Entries... "
    if find "$LOG_FILE" -newermt "5 minutes ago" | grep -q .; then
        echo -e "${GREEN}✓ ACTIVE${NC}"
        
        # Show last few log entries
        echo ""
        echo "📋 Recent Log Entries:"
        echo "----------------------"
        tail -5 "$LOG_FILE" | while IFS= read -r line; do
            # Try to format as JSON if possible
            if echo "$line" | jq . >/dev/null 2>&1; then
                echo "$line" | jq -r '[.timestamp // .time // "unknown", .level // "INFO", .message // .msg // .] | @tsv'
            else
                echo "$line"
            fi
        done
    else
        echo -e "${YELLOW}⚠ NO RECENT ENTRIES${NC}"
    fi
else
    echo -e "${RED}✗ MISSING${NC}"
fi

echo ""
echo "🎯 Testing Task Execution"
echo "-------------------------"

# Test task execution via REST API
if command -v jq >/dev/null 2>&1; then
    echo -n "Testing Hello World Task... "
    
    TASK_EXECUTION_DATA='{
        "task_path": "hello-world",
        "input": {
            "name": "Test"
        }
    }'
    
    if response=$(curl -s -X POST \
                      -H "Content-Type: application/json" \
                      -d "$TASK_EXECUTION_DATA" \
                      "$BASE_URL:$HTTP_PORT/api/v1/executions" 2>/dev/null); then
        if echo "$response" | jq -e '.success // false' >/dev/null 2>&1; then
            echo -e "${GREEN}✓ PASS${NC}"
            echo "Response: $(echo "$response" | jq -c .)"
        else
            echo -e "${RED}✗ FAIL${NC}"
            echo "Response: $response"
        fi
    else
        echo -e "${RED}✗ FAIL${NC} (Connection failed)"
    fi
else
    echo -e "${YELLOW}⚠ SKIPPED${NC} (jq not installed)"
fi

echo ""
echo "🎉 Test Summary"
echo "==============="
echo ""
echo "If all tests passed, your Ratchet server is fully operational!"
echo ""
echo "Next steps:"
echo "1. Configure Claude Desktop with the provided configuration"
echo "2. Test MCP integration by asking Claude about available tasks"
echo "3. Add your own tasks to /tmp/ratchet/tasks/"
echo "4. Monitor logs at /tmp/ratchet/logs/ratchet.log"
echo ""
echo "Server URLs:"
echo "- REST API: $BASE_URL:$HTTP_PORT/api/v1/"
echo "- GraphQL:  $BASE_URL:$HTTP_PORT/graphql"
echo "- MCP SSE:  $BASE_URL:$MCP_PORT/"

# Clean up
rm -f /tmp/test_response