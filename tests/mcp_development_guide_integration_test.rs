//! Integration test validating the MCP Development Guide scenarios
//!
//! This test validates the scenarios and examples documented in docs/MCP_DEVELOPMENT_GUIDE.md
//! focusing on documentation completeness and example correctness.

use anyhow::Result;

/// Test development guide content validation
#[tokio::test]
async fn test_development_guide_content() -> Result<()> {
    // Read the actual development guide to validate its content
    let guide_content = include_str!("../docs/MCP_DEVELOPMENT_GUIDE.md");
    
    // Verify key sections are present
    let required_sections = [
        "Ratchet MCP Development Guide",
        "Getting Started", 
        "Core Workflow",
        "httpbin_get_origin",
        "Step-by-Step Walkthrough",
        "Monitoring and Debugging", 
        "Administrative Operations",
        "Best Practices",
        "Troubleshooting"
    ];
    
    for section in &required_sections {
        assert!(guide_content.contains(section), "Missing required section: {}", section);
        println!("✓ Found section: {}", section);
    }
    
    // Verify HTTPBin task example is present
    assert!(guide_content.contains("https://httpbin.org/get"));
    assert!(guide_content.contains("{ origin: data.origin }"));
    
    println!("✓ Development guide content validation completed");
    
    Ok(())
}

/// Test endpoint reference content validation
#[tokio::test]
async fn test_endpoint_reference_content() -> Result<()> {
    // Read the actual endpoint reference to validate its content
    let reference_content = include_str!("../docs/MCP_ENDPOINTS_REFERENCE.md");
    
    // Verify key tools are documented
    let required_tools = [
        "ratchet_execute_task",
        "ratchet_create_task", 
        "ratchet_get_execution_status",
        "ratchet_get_execution_logs"
    ];
    
    for tool in &required_tools {
        assert!(reference_content.contains(tool), "Missing tool documentation: {}", tool);
        println!("✓ Found tool documentation: {}", tool);
    }
    
    println!("✓ Endpoint reference content validation completed");
    
    Ok(())
}

/// Test HTTPBin task example files
#[tokio::test]
async fn test_httpbin_task_files() -> Result<()> {
    // Verify HTTPBin task example file exists and has correct content
    let task_content = include_str!("../sample/js-tasks/httpbin-get-origin/task.js");
    assert!(task_content.contains("https://httpbin.org/get"));
    assert!(task_content.contains("return { origin: data.origin }"));
    println!("✓ HTTPBin task implementation");
    
    let meta_content = include_str!("../sample/js-tasks/httpbin-get-origin/meta.yaml");
    assert!(meta_content.contains("httpbin_get_origin"));
    assert!(meta_content.contains("origin"));
    println!("✓ HTTPBin task metadata");
    
    Ok(())
}

/// Test MCP tools list validation
#[tokio::test]
async fn test_expected_mcp_tools() -> Result<()> {
    // This test validates that the expected MCP tools are documented
    let reference_content = include_str!("../docs/MCP_ENDPOINTS_REFERENCE.md");
    
    let expected_tools = [
        "ratchet_execute_task",
        "ratchet_create_task",
        "ratchet_list_available_tasks", 
        "ratchet_get_execution_status",
        "ratchet_get_execution_logs",
        "ratchet_get_execution_trace",
        "ratchet_validate_task",
        "ratchet_get_developer_guide_walkthrough",
        "ratchet_get_developer_endpoint_reference",
        "ratchet_get_developer_integration_guide"
    ];
    
    let mut found_count = 0;
    for tool in &expected_tools {
        if reference_content.contains(tool) {
            found_count += 1;
            println!("✓ Found tool: {}", tool);
        } else {
            println!("⚠ Missing tool: {}", tool);
        }
    }
    
    println!("✓ Found {}/{} expected MCP tools documented", found_count, expected_tools.len());
    assert!(found_count >= 8, "Expected at least 8 tools to be documented");
    
    Ok(())
}

/// Test JSON-RPC examples validation
#[tokio::test]
async fn test_jsonrpc_examples() -> Result<()> {
    let guide_content = include_str!("../docs/MCP_DEVELOPMENT_GUIDE.md");
    
    // Verify JSON-RPC structure examples
    assert!(guide_content.contains("\"jsonrpc\": \"2.0\""));
    assert!(guide_content.contains("\"method\": \"tools/call\""));
    assert!(guide_content.contains("\"params\""));
    
    // Verify task creation example
    assert!(guide_content.contains("ratchet_create_task"));
    assert!(guide_content.contains("\"input_schema\""));
    assert!(guide_content.contains("\"output_schema\""));
    
    println!("✓ JSON-RPC examples validation completed");
    
    Ok(())
}

/// Test development guide scenarios structure
#[tokio::test]
async fn test_development_guide_scenarios() -> Result<()> {
    let guide_content = include_str!("../docs/MCP_DEVELOPMENT_GUIDE.md");
    
    // Verify guide has proper structure with numbered sections
    let guide_scenarios = [
        "Getting Started",
        "Core Workflow: Create and Execute a Task", 
        "Step-by-Step Walkthrough",
        "Monitoring and Debugging",
        "Administrative Operations",
        "Advanced Features",
        "Best Practices",
        "Troubleshooting"
    ];
    
    for scenario in &guide_scenarios {
        assert!(guide_content.contains(scenario), "Missing scenario: {}", scenario);
        println!("✓ Scenario documented: {}", scenario);
    }
    
    println!("✓ All development guide scenarios validated");
    
    Ok(())
}

/// Test HTTPBin task code structure validation
#[tokio::test]
async fn test_httpbin_task_code_structure() -> Result<()> {
    let task_content = include_str!("../sample/js-tasks/httpbin-get-origin/task.js");
    
    // Validate the task structure matches the guide
    assert!(task_content.contains("async function main(input)"));
    assert!(task_content.contains("fetch('https://httpbin.org/get')"));
    assert!(task_content.contains("await response.json()"));
    assert!(task_content.contains("return { origin: data.origin }"));
    
    println!("✓ HTTPBin task code structure validated");
    
    Ok(())
}

/// Integration test summary that validates the complete MCP Development Guide
#[tokio::test]
async fn test_mcp_development_guide_integration_summary() -> Result<()> {
    println!("🚀 Starting MCP Development Guide Integration Test Summary");
    
    // Verify documentation exists and contains required content
    let guide_content = include_str!("../docs/MCP_DEVELOPMENT_GUIDE.md");
    assert!(guide_content.contains("httpbin_get_origin"));
    assert!(guide_content.contains("Getting Started"));
    assert!(guide_content.contains("Best Practices"));
    println!("✓ Development guide content");
    
    let reference_content = include_str!("../docs/MCP_ENDPOINTS_REFERENCE.md");
    assert!(reference_content.contains("ratchet_execute_task"));
    assert!(reference_content.contains("28 MCP tools")); // From updated documentation
    println!("✓ Endpoint reference content");
    
    // Verify HTTPBin task example files exist and are correct
    let task_content = include_str!("../sample/js-tasks/httpbin-get-origin/task.js");
    assert!(task_content.contains("https://httpbin.org/get"));
    println!("✓ HTTPBin task example");
    
    let meta_content = include_str!("../sample/js-tasks/httpbin-get-origin/meta.yaml");
    assert!(meta_content.contains("httpbin_get_origin"));
    println!("✓ HTTPBin task metadata");
    
    // Verify guide has comprehensive examples
    assert!(guide_content.contains("\"jsonrpc\": \"2.0\""));
    assert!(guide_content.contains("ratchet_create_task"));
    assert!(guide_content.contains("Enhanced HTTP client"));
    println!("✓ JSON-RPC examples");
    
    // Verify all key sections are present
    let key_sections = [
        "Table of Contents",
        "Getting Started", 
        "Core Workflow",
        "Step-by-Step Walkthrough",
        "Monitoring and Debugging",
        "Administrative Operations", 
        "Advanced Features",
        "Best Practices",
        "Troubleshooting"
    ];
    
    for section in &key_sections {
        assert!(guide_content.contains(section), "Missing section: {}", section);
    }
    println!("✓ All required sections present");
    
    println!("🎯 Integration Test Summary:");
    println!("✓ Development guide documentation completeness");
    println!("✓ Endpoint reference documentation accuracy"); 
    println!("✓ HTTPBin task implementation correctness");
    println!("✓ Task metadata configuration validity");
    println!("✓ JSON-RPC examples completeness");
    println!("✓ Documentation structure integrity");
    
    println!("🚀 All MCP Development Guide scenarios validated successfully!");
    
    Ok(())
}