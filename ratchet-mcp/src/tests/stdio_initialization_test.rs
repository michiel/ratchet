//! Integration test for MCP server stdio initialization behavior
//!
//! This test verifies that the MCP server correctly handles initialization
//! and maintains session state across multiple requests via stdio transport.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

/// Test that the MCP server properly initializes and handles subsequent requests
/// over stdio transport without requiring the 'initialized' notification.
#[tokio::test]
#[ignore = "Integration test that spawns subprocess - use `cargo test -- --ignored` to run"]
async fn test_mcp_server_stdio_initialization_compatibility() {
    // Start the MCP server process
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
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server process");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Test sequence: initialize -> tools/list -> tools/call
    // This simulates the behavior of Claude Code which doesn't send 'initialized' notification

    // 1. Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "stdio-test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    let request_json = serde_json::to_string(&init_request).unwrap();
    writeln!(stdin, "{}", request_json).expect("Failed to write initialize request");

    // Read initialize response
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .expect("Failed to read initialize response");

    let init_response: Value =
        serde_json::from_str(response_line.trim()).expect("Failed to parse initialize response");

    // Verify initialize response
    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);
    assert!(init_response["result"].is_object());
    assert!(init_response["result"]["capabilities"]["tools"].is_object());
    assert_eq!(
        init_response["result"]["serverInfo"]["name"],
        "Ratchet MCP Server"
    );

    // 2. Send tools/list request immediately (without 'initialized' notification)
    let tools_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 2
    });

    let request_json = serde_json::to_string(&tools_request).unwrap();
    writeln!(stdin, "{}", request_json).expect("Failed to write tools/list request");

    // Read tools/list response
    response_line.clear();
    reader
        .read_line(&mut response_line)
        .expect("Failed to read tools/list response");

    let tools_response: Value =
        serde_json::from_str(response_line.trim()).expect("Failed to parse tools/list response");

    // Verify tools/list response (should NOT be "Server not initialized" error)
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

    // Verify we have the expected Ratchet tools
    let tool_names: Vec<String> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();

    assert!(tool_names.contains(&"ratchet_execute_task".to_string()));
    assert!(tool_names.contains(&"ratchet_list_available_tasks".to_string()));

    // 3. Send tools/call request to further verify session persistence
    let call_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "ratchet_list_available_tasks",
            "arguments": {
                "include_schemas": false
            }
        },
        "id": 3
    });

    let request_json = serde_json::to_string(&call_request).unwrap();
    writeln!(stdin, "{}", request_json).expect("Failed to write tools/call request");

    // Read tools/call response
    response_line.clear();
    reader
        .read_line(&mut response_line)
        .expect("Failed to read tools/call response");

    let call_response: Value =
        serde_json::from_str(response_line.trim()).expect("Failed to parse tools/call response");

    // Verify tools/call response
    assert_eq!(call_response["jsonrpc"], "2.0");
    assert_eq!(call_response["id"], 3);
    assert!(
        call_response["error"].is_null(),
        "Expected successful tools/call response, got error: {}",
        call_response["error"]
    );
    assert!(call_response["result"]["content"].is_array());
    assert_eq!(call_response["result"]["isError"], false);

    // Close stdin to signal server to shut down
    drop(stdin);

    // Wait for server to terminate
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

/// Test that the server still works correctly when clients DO send the 'initialized' notification
#[tokio::test]
#[ignore = "Integration test that spawns subprocess - use `cargo test -- --ignored` to run"]
async fn test_mcp_server_stdio_with_initialized_notification() {
    // Start the MCP server process
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
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server process");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Test the full MCP protocol handshake: initialize -> initialized -> tools/list

    // 1. Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "compliant-test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    let request_json = serde_json::to_string(&init_request).unwrap();
    writeln!(stdin, "{}", request_json).expect("Failed to write initialize request");

    // Read initialize response
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .expect("Failed to read initialize response");

    let init_response: Value =
        serde_json::from_str(response_line.trim()).expect("Failed to parse initialize response");

    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);
    assert!(init_response["result"].is_object());

    // 2. Send initialized notification (proper MCP protocol)
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "initialized"
    });

    let notification_json = serde_json::to_string(&initialized_notification).unwrap();
    writeln!(stdin, "{}", notification_json).expect("Failed to write initialized notification");

    // Notifications don't have responses, so proceed directly to tools/list

    // 3. Send tools/list request
    let tools_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 2
    });

    let request_json = serde_json::to_string(&tools_request).unwrap();
    writeln!(stdin, "{}", request_json).expect("Failed to write tools/list request");

    // Read tools/list response
    response_line.clear();
    reader
        .read_line(&mut response_line)
        .expect("Failed to read tools/list response");

    let tools_response: Value =
        serde_json::from_str(response_line.trim()).expect("Failed to parse tools/list response");

    // Verify tools/list response works correctly
    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert_eq!(tools_response["id"], 2);
    assert!(tools_response["error"].is_null());
    assert!(tools_response["result"]["tools"].is_array());

    // Close stdin and cleanup
    drop(stdin);

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
            let _ = child.kill();
            panic!("MCP server did not exit within timeout");
        }
    }
}
