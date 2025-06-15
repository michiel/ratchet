//! Simple integration test for MCP server stdio functionality
//!
//! This test ensures that the MCP server binary can be spawned successfully
//! and handles basic JSON-RPC requests over stdio.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

/// Test that the MCP server can start and handle a basic initialize request
#[tokio::test]
#[ignore = "Integration test that spawns subprocess - use `cargo test -- --ignored` to run"]
async fn test_mcp_server_basic_stdio_functionality() {
    // Start the MCP server process with explicit stdio transport
    let mut child = Command::new("cargo")
        .args([
            "run",
            "--package",
            "ratchet",
            "--bin",
            "ratchet",
            "--features",
            "mcp-server",
            "--",
            "mcp-serve",
            "--transport",
            "stdio",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // Ignore stderr to avoid noise
        .spawn()
        .expect("Failed to start MCP server process");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    let request_json = serde_json::to_string(&init_request).unwrap();
    
    // Write request with timeout
    let write_result = timeout(Duration::from_secs(5), async {
        writeln!(stdin, "{}", request_json)
    }).await;
    
    match write_result {
        Ok(Ok(_)) => {},
        Ok(Err(e)) => {
            let _ = child.kill();
            panic!("Failed to write initialize request: {}", e);
        },
        Err(_) => {
            let _ = child.kill();
            panic!("Timeout writing initialize request");
        }
    }

    // Read response with timeout
    let mut response_line = String::new();
    let read_result = timeout(Duration::from_secs(10), async {
        reader.read_line(&mut response_line)
    }).await;
    
    match read_result {
        Ok(Ok(_)) => {},
        Ok(Err(e)) => {
            let _ = child.kill();
            panic!("Failed to read initialize response: {}", e);
        },
        Err(_) => {
            let _ = child.kill();
            panic!("Timeout reading initialize response");
        }
    }

    // Parse and verify response
    let init_response: Value = serde_json::from_str(response_line.trim())
        .expect("Failed to parse initialize response");

    // Basic verification
    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);
    assert!(init_response["result"].is_object());
    assert!(init_response["result"]["capabilities"]["tools"].is_object());

    // Send tools/list request without 'initialized' notification (Claude Code behavior)
    let tools_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 2
    });

    let request_json = serde_json::to_string(&tools_request).unwrap();
    
    // Write tools request with timeout
    let write_result = timeout(Duration::from_secs(5), async {
        writeln!(stdin, "{}", request_json)
    }).await;
    
    match write_result {
        Ok(Ok(_)) => {},
        Ok(Err(e)) => {
            let _ = child.kill();
            panic!("Failed to write tools/list request: {}", e);
        },
        Err(_) => {
            let _ = child.kill();
            panic!("Timeout writing tools/list request");
        }
    }

    // Read tools response with timeout
    response_line.clear();
    let read_result = timeout(Duration::from_secs(10), async {
        reader.read_line(&mut response_line)
    }).await;
    
    match read_result {
        Ok(Ok(_)) => {},
        Ok(Err(e)) => {
            let _ = child.kill();
            panic!("Failed to read tools/list response: {}", e);
        },
        Err(_) => {
            let _ = child.kill();
            panic!("Timeout reading tools/list response");
        }
    }

    let tools_response: Value = serde_json::from_str(response_line.trim())
        .expect("Failed to parse tools/list response");

    // Verify tools/list response works (no "Server not initialized" error)
    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert_eq!(tools_response["id"], 2);
    assert!(
        tools_response["error"].is_null(),
        "Expected successful tools/list response, got error: {}",
        tools_response["error"]
    );
    assert!(tools_response["result"]["tools"].is_array());

    let tools = tools_response["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty(), "Expected non-empty tools list");

    // Verify expected Ratchet tools are present
    let tool_names: Vec<String> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();

    assert!(tool_names.contains(&"ratchet_execute_task".to_string()));
    assert!(tool_names.contains(&"ratchet_list_available_tasks".to_string()));

    // Close stdin to signal server to shut down
    drop(stdin);

    // Wait for server to terminate with timeout
    let timeout_duration = Duration::from_secs(5);
    let exit_status = timeout(timeout_duration, async {
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status,
                Ok(None) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                Err(e) => panic!("Error waiting for child process: {}", e),
            }
        }
    })
    .await;

    match exit_status {
        Ok(status) => {
            assert!(status.success(), "MCP server exited with error: {}", status);
        }
        Err(_) => {
            // Kill the process if it didn't exit gracefully
            let _ = child.kill();
            panic!("MCP server did not exit within timeout");
        }
    }
}