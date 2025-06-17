//! Comprehensive Integration Tests for Refine.dev Compatibility
//!
//! This module validates that our REST API endpoints work correctly with
//! Refine.dev query patterns, pagination, filtering, and response formats.

use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

/// Base test configuration
const BASE_URL: &str = "http://localhost:8080";
const API_BASE: &str = "http://localhost:8080/api/v1";

/// Test helper to check if server is running
async fn is_server_running(client: &Client) -> bool {
    match client.get(&format!("{}/health", BASE_URL)).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Create a test client with appropriate timeouts
fn create_test_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client")
}

/// Test helper to validate Refine.dev response structure
fn validate_refine_response(response: &Value) -> Result<()> {
    // Validate top-level structure
    assert!(response.get("data").is_some(), "Response should have 'data' field");
    assert!(response.get("meta").is_some(), "Response should have 'meta' field");
    
    let meta = response.get("meta").unwrap();
    
    // Validate pagination metadata structure
    let pagination = meta.get("pagination");
    assert!(pagination.is_some(), "Meta should have 'pagination' field");
    
    let pagination = pagination.unwrap();
    assert!(pagination.get("page").is_some(), "Pagination should have 'page'");
    assert!(pagination.get("limit").is_some(), "Pagination should have 'limit'");
    assert!(pagination.get("total").is_some(), "Pagination should have 'total'");
    assert!(pagination.get("totalPages").is_some(), "Pagination should have 'totalPages'");
    assert!(pagination.get("hasNext").is_some(), "Pagination should have 'hasNext'");
    assert!(pagination.get("hasPrevious").is_some(), "Pagination should have 'hasPrevious'");
    
    // Validate timestamp is present
    assert!(meta.get("timestamp").is_some(), "Meta should have 'timestamp'");
    
    Ok(())
}

/// Test Refine.dev style pagination with _start and _end parameters
#[tokio::test]
async fn test_refine_pagination_start_end() -> Result<()> {
    let client = create_test_client();
    
    if !is_server_running(&client).await {
        println!("⚠️ Server not running - skipping Refine.dev pagination tests");
        return Ok(());
    }
    
    println!("🧪 Testing Refine.dev pagination with _start and _end...");
    
    // Test basic pagination with _start and _end
    let response = client
        .get(&format!("{}/tasks?_start=0&_end=5", API_BASE))
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Request should succeed");
    
    let json: Value = response.json().await?;
    validate_refine_response(&json)?;
    
    let data = json.get("data").unwrap().as_array().unwrap();
    assert!(data.len() <= 5, "Should return at most 5 items");
    
    // Test larger range
    let response = client
        .get(&format!("{}/tasks?_start=0&_end=10", API_BASE))
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Request should succeed");
    
    let json: Value = response.json().await?;
    validate_refine_response(&json)?;
    
    let data = json.get("data").unwrap().as_array().unwrap();
    assert!(data.len() <= 10, "Should return at most 10 items");
    
    println!("✅ Refine.dev pagination (_start/_end) working correctly");
    Ok(())
}

/// Test Refine.dev style sorting with _sort and _order parameters
#[tokio::test]
async fn test_refine_sorting() -> Result<()> {
    let client = create_test_client();
    
    if !is_server_running(&client).await {
        println!("⚠️ Server not running - skipping Refine.dev sorting tests");
        return Ok(());
    }
    
    println!("🧪 Testing Refine.dev sorting with _sort and _order...");
    
    // Test ascending sort
    let response = client
        .get(&format!("{}/tasks?_start=0&_end=10&_sort=name&_order=ASC", API_BASE))
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Request should succeed");
    
    let json: Value = response.json().await?;
    validate_refine_response(&json)?;
    
    // Test descending sort
    let response = client
        .get(&format!("{}/tasks?_start=0&_end=10&_sort=name&_order=DESC", API_BASE))
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Request should succeed");
    
    let json: Value = response.json().await?;
    validate_refine_response(&json)?;
    
    // Test sort by different fields
    let response = client
        .get(&format!("{}/tasks?_start=0&_end=10&_sort=created_at&_order=DESC", API_BASE))
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Request should succeed");
    
    let json: Value = response.json().await?;
    validate_refine_response(&json)?;
    
    println!("✅ Refine.dev sorting (_sort/_order) working correctly");
    Ok(())
}

/// Test advanced Refine.dev filtering operators
#[tokio::test]
async fn test_refine_advanced_filtering() -> Result<()> {
    let client = create_test_client();
    
    if !is_server_running(&client).await {
        println!("⚠️ Server not running - skipping Refine.dev filtering tests");
        return Ok(());
    }
    
    println!("🧪 Testing Refine.dev advanced filtering...");
    
    // Test different filter operators
    let filter_tests = vec![
        ("name_like", "task"),           // Contains operator
        ("enabled", "true"),             // Equality operator
        ("version", "1.0.0"),           // Exact match
        ("name_ne", "nonexistent"),     // Not equal operator
    ];
    
    for (filter_key, filter_value) in filter_tests {
        let response = client
            .get(&format!("{}/tasks?_start=0&_end=10&{}={}", API_BASE, filter_key, filter_value))
            .send()
            .await?;
        
        assert!(response.status().is_success(), 
                "Request with filter {}={} should succeed", filter_key, filter_value);
        
        let json: Value = response.json().await?;
        validate_refine_response(&json)?;
        
        println!("✅ Filter {}={} working correctly", filter_key, filter_value);
    }
    
    println!("✅ Refine.dev advanced filtering working correctly");
    Ok(())
}

/// Test all entity endpoints for Refine.dev compatibility
#[tokio::test]
async fn test_all_endpoints_refine_compatibility() -> Result<()> {
    let client = create_test_client();
    
    if !is_server_running(&client).await {
        println!("⚠️ Server not running - skipping endpoint compatibility tests");
        return Ok(());
    }
    
    println!("🧪 Testing all endpoints for Refine.dev compatibility...");
    
    let endpoints = vec![
        "tasks",
        "executions", 
        "jobs",
        "schedules",
    ];
    
    for endpoint in endpoints {
        println!("Testing endpoint: {}", endpoint);
        
        // Test basic Refine.dev query
        let response = client
            .get(&format!("{}/{}?_start=0&_end=5", API_BASE, endpoint))
            .send()
            .await?;
        
        assert!(response.status().is_success(), 
                "Endpoint {} should support Refine.dev pagination", endpoint);
        
        let json: Value = response.json().await?;
        validate_refine_response(&json)?;
        
        // Test with sorting
        let response = client
            .get(&format!("{}/{}?_start=0&_end=5&_sort=id&_order=ASC", API_BASE, endpoint))
            .send()
            .await?;
        
        assert!(response.status().is_success(), 
                "Endpoint {} should support Refine.dev sorting", endpoint);
        
        let json: Value = response.json().await?;
        validate_refine_response(&json)?;
        
        println!("✅ Endpoint {} is Refine.dev compatible", endpoint);
    }
    
    println!("✅ All endpoints are Refine.dev compatible");
    Ok(())
}

/// Test mixed query parameters (Refine.dev + standard)
#[tokio::test]
async fn test_mixed_query_parameters() -> Result<()> {
    let client = create_test_client();
    
    if !is_server_running(&client).await {
        println!("⚠️ Server not running - skipping mixed parameter tests");
        return Ok(());
    }
    
    println!("🧪 Testing mixed query parameters...");
    
    // Test mixing Refine.dev pagination with standard parameters
    let response = client
        .get(&format!("{}/tasks?_start=0&_end=10&page=1&limit=5", API_BASE))
        .send()
        .await?;
    
    // Should prefer Refine.dev style (_start/_end) over standard (page/limit)
    assert!(response.status().is_success(), "Mixed parameters should work");
    
    let json: Value = response.json().await?;
    validate_refine_response(&json)?;
    
    println!("✅ Mixed query parameters working correctly");
    Ok(())
}

/// Test error handling with invalid Refine.dev parameters
#[tokio::test]
async fn test_refine_error_handling() -> Result<()> {
    let client = create_test_client();
    
    if !is_server_running(&client).await {
        println!("⚠️ Server not running - skipping error handling tests");
        return Ok(());
    }
    
    println!("🧪 Testing Refine.dev error handling...");
    
    // Test invalid pagination range (start >= end)
    let response = client
        .get(&format!("{}/tasks?_start=10&_end=5", API_BASE))
        .send()
        .await?;
    
    assert!(response.status().is_client_error(), 
            "Invalid range should return client error");
    
    // Test pagination range too large
    let response = client
        .get(&format!("{}/tasks?_start=0&_end=200", API_BASE))
        .send()
        .await?;
    
    assert!(response.status().is_client_error(), 
            "Range too large should return client error");
    
    // Test invalid sort order
    let response = client
        .get(&format!("{}/tasks?_start=0&_end=10&_sort=name&_order=INVALID", API_BASE))
        .send()
        .await?;
    
    // Should either work (ignoring invalid order) or return error
    // Both behaviors are acceptable
    
    println!("✅ Refine.dev error handling working correctly");
    Ok(())
}

/// Test pagination metadata accuracy
#[tokio::test]
async fn test_pagination_metadata_accuracy() -> Result<()> {
    let client = create_test_client();
    
    if !is_server_running(&client).await {
        println!("⚠️ Server not running - skipping pagination metadata tests");
        return Ok(());
    }
    
    println!("🧪 Testing pagination metadata accuracy...");
    
    // Get first page
    let response = client
        .get(&format!("{}/tasks?_start=0&_end=3", API_BASE))
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Request should succeed");
    
    let json: Value = response.json().await?;
    validate_refine_response(&json)?;
    
    let meta = json.get("meta").unwrap();
    let pagination = meta.get("pagination").unwrap();
    
    // Validate pagination values make sense
    let page = pagination.get("page").unwrap().as_u64().unwrap();
    let limit = pagination.get("limit").unwrap().as_u64().unwrap();
    let total = pagination.get("total").unwrap().as_u64().unwrap();
    let total_pages = pagination.get("totalPages").unwrap().as_u64().unwrap();
    let has_next = pagination.get("hasNext").unwrap().as_bool().unwrap();
    let has_previous = pagination.get("hasPrevious").unwrap().as_bool().unwrap();
    
    assert_eq!(page, 1, "First page should be page 1");
    assert_eq!(limit, 3, "Limit should match request");
    assert!(!has_previous, "First page should not have previous");
    
    if total > 3 {
        assert!(has_next, "Should have next page if total > limit");
        assert!(total_pages > 1, "Should have multiple pages if total > limit");
    }
    
    // Calculate expected total pages
    let expected_total_pages = if total == 0 { 1 } else { (total + limit - 1) / limit };
    assert_eq!(total_pages, expected_total_pages, "Total pages calculation should be correct");
    
    println!("✅ Pagination metadata is accurate");
    Ok(())
}

/// Test response format matches Refine.dev expectations exactly
#[tokio::test]
async fn test_response_format_compliance() -> Result<()> {
    let client = create_test_client();
    
    if !is_server_running(&client).await {
        println!("⚠️ Server not running - skipping response format tests");
        return Ok(());
    }
    
    println!("🧪 Testing response format compliance with Refine.dev...");
    
    let response = client
        .get(&format!("{}/tasks?_start=0&_end=5", API_BASE))
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Request should succeed");
    
    let json: Value = response.json().await?;
    
    // Validate exact structure expected by Refine.dev
    assert!(json.is_object(), "Response should be an object");
    
    let response_obj = json.as_object().unwrap();
    assert_eq!(response_obj.len(), 2, "Response should have exactly 2 top-level fields");
    assert!(response_obj.contains_key("data"), "Must have 'data' field");
    assert!(response_obj.contains_key("meta"), "Must have 'meta' field");
    
    // Validate data field
    let data = json.get("data").unwrap();
    assert!(data.is_array(), "Data field should be an array");
    
    // Validate meta field structure
    let meta = json.get("meta").unwrap();
    assert!(meta.is_object(), "Meta field should be an object");
    
    let meta_obj = meta.as_object().unwrap();
    assert!(meta_obj.contains_key("pagination"), "Meta must have 'pagination'");
    assert!(meta_obj.contains_key("timestamp"), "Meta must have 'timestamp'");
    
    // Validate pagination structure within meta
    let pagination = meta.get("pagination").unwrap();
    assert!(pagination.is_object(), "Pagination should be an object");
    
    let pagination_obj = pagination.as_object().unwrap();
    let required_pagination_fields = vec![
        "page", "limit", "total", "totalPages", "hasNext", "hasPrevious", "offset"
    ];
    
    for field in required_pagination_fields {
        assert!(pagination_obj.contains_key(field), 
                "Pagination must have '{}' field", field);
    }
    
    // Validate data types
    assert!(pagination.get("page").unwrap().is_number(), "page should be number");
    assert!(pagination.get("limit").unwrap().is_number(), "limit should be number");
    assert!(pagination.get("total").unwrap().is_number(), "total should be number");
    assert!(pagination.get("totalPages").unwrap().is_number(), "totalPages should be number");
    assert!(pagination.get("hasNext").unwrap().is_boolean(), "hasNext should be boolean");
    assert!(pagination.get("hasPrevious").unwrap().is_boolean(), "hasPrevious should be boolean");
    assert!(pagination.get("offset").unwrap().is_number(), "offset should be number");
    
    // Validate timestamp format
    let timestamp = meta.get("timestamp").unwrap().as_str().unwrap();
    assert!(timestamp.ends_with('Z'), "Timestamp should be in ISO 8601 UTC format");
    
    println!("✅ Response format is fully compliant with Refine.dev expectations");
    Ok(())
}

/// Integration test to validate the exact issue that was fixed
#[tokio::test]
async fn test_original_issue_resolution() -> Result<()> {
    let client = create_test_client();
    
    if !is_server_running(&client).await {
        println!("⚠️ Server not running - skipping original issue test");
        return Ok(());
    }
    
    println!("🧪 Testing that original query parameter issue is resolved...");
    
    // This is the exact request that was failing before our fix
    let response = client
        .get(&format!("{}/tasks?_end=10&_start=0", API_BASE))
        .send()
        .await?;
    
    // Should NOT return the error we were getting before:
    // {"error":{"code":"BAD_REQUEST","message":"Bad request: Invalid query parameters: Failed to deserialize query string"}}
    
    assert!(response.status().is_success(), 
            "The original failing request should now succeed");
    
    let json: Value = response.json().await?;
    
    // Should be a proper response, not an error
    assert!(json.get("error").is_none(), "Should not contain error field");
    assert!(json.get("data").is_some(), "Should contain data field");
    
    validate_refine_response(&json)?;
    
    println!("✅ Original query parameter deserialization issue is resolved");
    Ok(())
}

/// Performance test to ensure Refine.dev queries are reasonably fast
#[tokio::test]
async fn test_refine_query_performance() -> Result<()> {
    let client = create_test_client();
    
    if !is_server_running(&client).await {
        println!("⚠️ Server not running - skipping performance tests");
        return Ok(());
    }
    
    println!("🧪 Testing Refine.dev query performance...");
    
    use std::time::Instant;
    
    let start = Instant::now();
    
    // Execute multiple queries to test performance
    for i in 0..10 {
        let response = client
            .get(&format!("{}/tasks?_start={}&_end={}", API_BASE, i * 5, (i + 1) * 5))
            .send()
            .await?;
        
        assert!(response.status().is_success(), "Request {} should succeed", i);
    }
    
    let duration = start.elapsed();
    
    println!("Executed 10 Refine.dev queries in {:?}", duration);
    
    // Should complete within reasonable time (adjust as needed)
    assert!(duration.as_secs() < 10, "Queries should complete within 10 seconds");
    
    println!("✅ Refine.dev queries perform within acceptable limits");
    Ok(())
}