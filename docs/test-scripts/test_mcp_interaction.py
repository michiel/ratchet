#!/usr/bin/env python3
"""Test MCP server interactions"""

import json
import subprocess
import sys
import time
from typing import Dict, Any

def send_request(proc: subprocess.Popen, request: Dict[str, Any]) -> Dict[str, Any]:
    """Send a JSON-RPC request and get response"""
    request_str = json.dumps(request) + '\n'
    proc.stdin.write(request_str.encode())
    proc.stdin.flush()
    
    # Read response
    response_line = proc.stdout.readline().decode().strip()
    if response_line:
        return json.loads(response_line)
    return {}

def test_mcp_server():
    """Test MCP server functionality"""
    # Start MCP server
    proc = subprocess.Popen(
        ['./target/release/ratchet-mcp', '-c', 'sample/configs/test-config.yaml', 'serve', '--transport', 'stdio'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=False
    )
    
    try:
        # Wait for server to start
        time.sleep(0.5)
        
        # Test 1: Initialize
        print("Test 1: Initialize connection")
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }
        response = send_request(proc, init_request)
        print(f"Initialize response: {json.dumps(response, indent=2)}")
        
        # Test 2: List tools
        print("\nTest 2: List available tools")
        tools_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        response = send_request(proc, tools_request)
        print(f"Tools list response: {json.dumps(response, indent=2)}")
        
        # Test 3: Execute a task
        print("\nTest 3: Execute addition task")
        execute_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "execute_task",
                "arguments": {
                    "task_name": "addition",
                    "input": {
                        "a": 5,
                        "b": 3
                    }
                }
            }
        }
        response = send_request(proc, execute_request)
        print(f"Execute task response: {json.dumps(response, indent=2)}")
        
        # Test 4: Get server info
        print("\nTest 4: Get server info")
        info_request = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "server_info",
                "arguments": {}
            }
        }
        response = send_request(proc, info_request)
        print(f"Server info response: {json.dumps(response, indent=2)}")
        
    finally:
        # Cleanup
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    test_mcp_server()