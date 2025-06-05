#!/usr/bin/env python3
"""Complete MCP server test with sample task execution"""

import json
import subprocess
import sys
import time
import urllib.request
import urllib.error
from typing import Dict, Any, Optional

# First, ensure the Ratchet server is running
BASE_URL = "http://127.0.0.1:8080"

def check_server_health():
    """Check if Ratchet server is healthy"""
    try:
        with urllib.request.urlopen(f"{BASE_URL}/health") as response:
            return response.status == 200
    except:
        return False

def send_mcp_request(proc: subprocess.Popen, request: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    """Send a JSON-RPC request and get response"""
    request_str = json.dumps(request) + '\n'
    proc.stdin.write(request_str.encode())
    proc.stdin.flush()
    
    # Read response with timeout
    start_time = time.time()
    while time.time() - start_time < 5:  # 5 second timeout
        line = proc.stdout.readline()
        if line:
            try:
                return json.loads(line.decode().strip())
            except json.JSONDecodeError:
                continue
    return None

def test_mcp_server():
    """Test MCP server functionality"""
    
    # Check server health first
    if not check_server_health():
        print("ERROR: Ratchet server is not running. Please start it first.")
        return
    
    print("✓ Ratchet server is healthy")
    
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
        time.sleep(1)
        
        # Test 1: Initialize
        print("\n1. Testing MCP Initialize")
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "0.1.0",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                },
                "capabilities": {}
            }
        }
        response = send_mcp_request(proc, init_request)
        if response:
            print(f"✓ Initialize response: {json.dumps(response, indent=2)}")
        else:
            print("✗ No response received for initialize")
        
        # Test 2: List tools
        print("\n2. Testing List Tools")
        tools_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        response = send_mcp_request(proc, tools_request)
        if response:
            print(f"✓ Tools list response: {json.dumps(response, indent=2)}")
        else:
            print("✗ No response received for tools/list")
        
        # Test 3: List available tasks
        print("\n3. Testing List Tasks")
        list_tasks_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "ratchet.list_tasks",
                "arguments": {}
            }
        }
        response = send_mcp_request(proc, list_tasks_request)
        if response:
            print(f"✓ List tasks response: {json.dumps(response, indent=2)}")
        else:
            print("✗ No response received for list_tasks")
        
        # Test 4: Execute addition task
        print("\n4. Testing Execute Addition Task")
        execute_request = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "ratchet.execute_task",
                "arguments": {
                    "task_name": "addition",
                    "input": {
                        "a": 10,
                        "b": 20
                    }
                }
            }
        }
        response = send_mcp_request(proc, execute_request)
        if response:
            print(f"✓ Execute task response: {json.dumps(response, indent=2)}")
        else:
            print("✗ No response received for execute_task")
        
        # Test 5: Test with authentication header
        print("\n5. Testing with Authentication")
        auth_request = {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "ratchet.execute_task",
                "arguments": {
                    "task_name": "addition",
                    "input": {
                        "a": 5,
                        "b": 7
                    }
                },
                "headers": {
                    "Authorization": "Bearer test-claude-client-key-12345"
                }
            }
        }
        response = send_mcp_request(proc, auth_request)
        if response:
            print(f"✓ Authenticated execute response: {json.dumps(response, indent=2)}")
        else:
            print("✗ No response received for authenticated execute")
        
        print("\n✅ MCP server tests completed")
        
    except Exception as e:
        print(f"Error during testing: {e}")
    finally:
        # Cleanup
        proc.terminate()
        proc.wait()
        print("\n✓ MCP server stopped")

if __name__ == "__main__":
    test_mcp_server()