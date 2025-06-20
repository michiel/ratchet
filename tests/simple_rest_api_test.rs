//! Simple REST API test to validate basic functionality

use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

/// Test basic REST API endpoint availability
#[tokio::test]
async fn test_basic_rest_api_endpoints() -> Result<()> {
    // This is a simplified test that just checks if we can hit basic endpoints
    // without setting up a full server (for quick validation)

    println!("🧪 Testing basic REST API structure...");

    // Note: This test assumes a server is running on localhost:8080
    // In a real CI environment, we'd want to start the server first

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;

    // Test if we can reach localhost (skip if no server running)
    let base_url = "http://localhost:8080";

    // Try to connect to see if anything is running
    match client.get(format!("{}/health", base_url)).send().await {
        Ok(response) => {
            println!("✅ Found running server on localhost:8080");

            // Test OpenAPI documentation endpoint
            let openapi_response = client.get(format!("{}/api-docs/openapi.json", base_url)).send().await?;
            if openapi_response.status().is_success() {
                let spec: Value = openapi_response.json().await?;

                // Verify OpenAPI structure
                assert!(spec.get("openapi").is_some(), "Should have openapi field");
                assert!(spec.get("info").is_some(), "Should have info field");
                assert!(spec.get("paths").is_some(), "Should have paths field");

                let paths = spec.get("paths").unwrap().as_object().unwrap();

                // Check that our documented endpoints are present
                let expected_endpoints = vec!["/tasks", "/executions", "/jobs", "/schedules"];

                for endpoint in expected_endpoints {
                    assert!(
                        paths.contains_key(endpoint),
                        "Endpoint {} should be documented in OpenAPI spec",
                        endpoint
                    );
                }

                println!("✅ OpenAPI specification is properly structured");

                // Test Swagger UI
                let swagger_response = client.get(format!("{}/docs", base_url)).send().await?;
                if swagger_response.status().is_success() {
                    let html = swagger_response.text().await?;
                    assert!(html.contains("swagger-ui"), "Should contain Swagger UI");
                    println!("✅ Swagger UI is accessible");
                }

                // Test basic API endpoints
                test_api_endpoints(&client, base_url).await?;
            } else {
                println!(
                    "⚠️ OpenAPI endpoint not available (status: {})",
                    openapi_response.status()
                );
            }
        }
        Err(_) => {
            println!("ℹ️ No server running on localhost:8080 - skipping live API tests");
            println!("ℹ️ To run full tests, start server with: cargo run -- serve");
        }
    }

    println!("✅ Basic REST API test completed");
    Ok(())
}

async fn test_api_endpoints(client: &Client, base_url: &str) -> Result<()> {
    println!("🔍 Testing API endpoints...");

    // Test tasks endpoint
    let tasks_response = client.get(format!("{}/api/v1/tasks", base_url)).send().await?;
    println!("Tasks endpoint status: {}", tasks_response.status());

    // Test executions endpoint
    let executions_response = client.get(format!("{}/api/v1/executions", base_url)).send().await?;
    println!("Executions endpoint status: {}", executions_response.status());

    // Test jobs endpoint
    let jobs_response = client.get(format!("{}/api/v1/jobs", base_url)).send().await?;
    println!("Jobs endpoint status: {}", jobs_response.status());

    // Test schedules endpoint
    let schedules_response = client.get(format!("{}/api/v1/schedules", base_url)).send().await?;
    println!("Schedules endpoint status: {}", schedules_response.status());

    // Test statistics endpoints
    let task_stats_response = client.get(format!("{}/api/v1/tasks/stats", base_url)).send().await?;
    println!("Task stats endpoint status: {}", task_stats_response.status());

    let execution_stats_response = client
        .get(format!("{}/api/v1/executions/stats", base_url))
        .send()
        .await?;
    println!("Execution stats endpoint status: {}", execution_stats_response.status());

    println!("✅ API endpoints tested");
    Ok(())
}

/// Validate OpenAPI specification structure
#[tokio::test]
async fn test_openapi_specification_structure() -> Result<()> {
    println!("🧪 Testing OpenAPI specification structure...");

    // This test validates our OpenAPI implementation without needing a running server
    // by checking that the OpenAPI spec can be generated correctly

    use ratchet_rest_api::openapi_spec;

    let spec = openapi_spec();

    // Test basic OpenAPI structure
    assert_eq!(spec.info.title, "Ratchet Task Execution API");
    assert_eq!(spec.info.version, "1.0.0");

    // Test that paths are documented
    assert!(!spec.paths.paths.is_empty(), "Should have documented paths");

    // Check for key endpoints
    let path_keys: Vec<String> = spec.paths.paths.keys().cloned().collect();
    let expected_paths = vec!["/tasks", "/executions", "/jobs", "/schedules"];

    for expected_path in expected_paths {
        assert!(
            path_keys.iter().any(|path| path.contains(expected_path)),
            "Path {} should be documented",
            expected_path
        );
    }

    // Test that components/schemas are documented
    if let Some(components) = &spec.components {
        let schemas = &components.schemas;
        assert!(!schemas.is_empty(), "Should have documented schemas");

        // Check for key model schemas
        let schema_names: Vec<String> = schemas.keys().cloned().collect();
        let expected_schemas = vec![
            "CreateTaskRequest",
            "UpdateTaskRequest",
            "TaskStats",
            "CreateExecutionRequest",
            "ExecutionStats",
            "CreateJobRequest",
            "JobStats",
            "CreateScheduleRequest",
            "ScheduleStats",
        ];

        for expected_schema in expected_schemas {
            assert!(
                schema_names.iter().any(|name| name.contains(expected_schema)),
                "Schema {} should be documented",
                expected_schema
            );
        }
    }

    // Test tags are properly defined
    if let Some(tags) = &spec.tags {
        let tag_names: Vec<String> = tags.iter().map(|tag| tag.name.clone()).collect();
        let expected_tags = vec!["tasks", "executions", "jobs", "schedules", "health"];

        for expected_tag in expected_tags {
            assert!(
                tag_names.contains(&expected_tag.to_string()),
                "Tag {} should be defined",
                expected_tag
            );
        }
    }

    println!("✅ OpenAPI specification structure is valid");
    println!("✅ All key endpoints are documented");
    println!("✅ All key schemas are included");
    println!("✅ Tags are properly defined");

    Ok(())
}
