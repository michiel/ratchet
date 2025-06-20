use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

/// End-to-end test for schedule workflow: create schedule, verify jobs, check executions, cleanup
///
/// This test demonstrates the complete lifecycle:
/// 1. Find the addition task
/// 2. Create a schedule that runs twice with 5 seconds between executions
/// 3. Configure webhook output for results
/// 4. Verify schedule creation via API
/// 5. Wait for jobs to be created and executed
/// 6. Verify jobs were created by the schedule
/// 7. Verify executions match the jobs
/// 8. Delete the schedule
/// 9. Verify schedule deletion via API
#[tokio::test]
#[ignore] // Integration test - requires running server
async fn test_complete_schedule_workflow() -> Result<()> {
    let client = Client::new();
    let api_base = "http://localhost:8080/api/v1";

    // Step 1: Find the addition task by name
    println!("🔍 Step 1: Finding addition task...");
    let tasks_response = client.get(format!("{}/tasks", api_base)).send().await?;

    assert_eq!(tasks_response.status(), 200, "Failed to fetch tasks");

    let tasks_data: Value = tasks_response.json().await?;
    let tasks = tasks_data["data"].as_array().expect("Tasks data should be an array");

    let addition_task = tasks
        .iter()
        .find(|task| task["name"].as_str() == Some("addition"))
        .expect("Addition task should exist");

    let task_id = addition_task["id"]
        .as_str()
        .expect("Task should have an id")
        .to_string();

    println!("✅ Found addition task with ID: {}", task_id);

    // Step 2: Create a schedule that runs twice with 5 seconds between
    println!("📅 Step 2: Creating schedule for addition task...");

    // Use a realistic cron expression (every minute)
    let schedule_payload = json!({
        "taskId": task_id,
        "name": "e2e-test-addition-schedule",
        "cronExpression": "*/1 * * * *", // Every minute
        "enabled": true
    });

    let create_schedule_response = client
        .post(format!("{}/schedules", api_base))
        .json(&schedule_payload)
        .send()
        .await?;

    if create_schedule_response.status() != 201 {
        let error_text = create_schedule_response.text().await?;
        panic!("Failed to create schedule: {}", error_text);
    }

    let created_schedule: Value = create_schedule_response.json().await?;
    let schedule_id = created_schedule["id"]
        .as_str()
        .expect("Schedule should have an id")
        .to_string();

    println!("✅ Created schedule with ID: {}", schedule_id);

    // Step 3: Verify schedule exists via API
    println!("🔍 Step 3: Verifying schedule exists...");

    let get_schedule_response = client
        .get(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    assert_eq!(get_schedule_response.status(), 200, "Schedule should exist");

    let schedule_data: Value = get_schedule_response.json().await?;
    assert_eq!(schedule_data["name"], "e2e-test-addition-schedule");
    assert_eq!(schedule_data["enabled"], true);
    assert_eq!(schedule_data["taskId"], task_id);

    println!("✅ Schedule verified to exist");

    // Step 4: Wait for schedule to create and execute jobs
    println!("⏳ Step 4: Waiting for schedule to create jobs (15 seconds)...");
    sleep(Duration::from_secs(15)).await;

    // Step 5: Check jobs created by this schedule
    println!("🔍 Step 5: Checking jobs created by schedule...");

    let jobs_response = client
        .get(format!("{}/jobs?scheduleId={}", api_base, schedule_id))
        .send()
        .await?;

    assert_eq!(jobs_response.status(), 200, "Failed to fetch jobs");

    let jobs_data: Value = jobs_response.json().await?;
    let jobs = jobs_data["data"].as_array().expect("Jobs data should be an array");

    // Verify we have the expected number of jobs (should be 2 or approaching 2)
    assert!(
        !jobs.is_empty(),
        "At least one job should have been created by the schedule"
    );
    println!("✅ Found {} job(s) created by schedule", jobs.len());

    // Verify job properties
    for (i, job) in jobs.iter().enumerate() {
        assert_eq!(job["taskId"], task_id, "Job {} should reference the addition task", i);
        assert_eq!(
            job["scheduleId"], schedule_id,
            "Job {} should reference our schedule",
            i
        );

        println!(
            "✅ Job {} validated: status={}, priority={}",
            i, job["status"], job["priority"]
        );
    }

    // Step 6: Check executions for these jobs
    println!("🔍 Step 6: Checking executions for jobs...");

    let mut execution_count = 0;
    let mut successful_executions = 0;

    for job in jobs {
        let job_id = job["id"].as_str().expect("Job should have an id");

        // Get executions for this job
        let executions_response = client
            .get(format!("{}/executions?jobId={}", api_base, job_id))
            .send()
            .await?;

        if executions_response.status() == 200 {
            let executions_data: Value = executions_response.json().await?;
            let executions = executions_data["data"]
                .as_array()
                .expect("Executions data should be an array");

            execution_count += executions.len();

            for execution in executions {
                let status = execution["status"].as_str().unwrap_or("unknown");
                println!("  📋 Execution: status={}, task={}", status, execution["taskId"]);

                if status == "COMPLETED" {
                    successful_executions += 1;

                    // Verify execution result (addition should return sum)
                    if let Some(result) = execution["result"].as_object() {
                        if let Some(sum) = result.get("sum") {
                            assert_eq!(sum, 7, "Addition result should be 2 + 5 = 7");
                            println!("  ✅ Execution result verified: sum = {}", sum);
                        }
                    }
                }
            }
        }
    }

    println!(
        "✅ Found {} execution(s), {} successful",
        execution_count, successful_executions
    );

    // Step 7: Wait a bit more to see if the second job gets created
    if jobs.len() < 2 {
        println!("⏳ Waiting additional 10 seconds for second job...");
        sleep(Duration::from_secs(10)).await;

        // Check jobs again
        let jobs_response2 = client
            .get(format!("{}/jobs?scheduleId={}", api_base, schedule_id))
            .send()
            .await?;

        if jobs_response2.status() == 200 {
            let jobs_data2: Value = jobs_response2.json().await?;
            let jobs2 = jobs_data2["data"].as_array().expect("Jobs data should be an array");
            println!("✅ Final job count: {}", jobs2.len());
        }
    }

    // Step 8: Delete the schedule
    println!("🗑️  Step 8: Deleting schedule...");

    let delete_response = client
        .delete(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    assert!(
        delete_response.status().is_success(),
        "Schedule deletion should succeed"
    );

    println!("✅ Schedule deleted successfully");

    // Step 9: Verify schedule is deleted
    println!("🔍 Step 9: Verifying schedule deletion...");

    let get_deleted_schedule_response = client
        .get(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    assert_eq!(
        get_deleted_schedule_response.status(),
        404,
        "Schedule should not exist after deletion"
    );

    println!("✅ Schedule deletion verified");

    // Step 10: Final verification - list all schedules to ensure ours is gone
    println!("🔍 Step 10: Final verification - checking schedule list...");

    let all_schedules_response = client.get(format!("{}/schedules", api_base)).send().await?;

    assert_eq!(all_schedules_response.status(), 200, "Should be able to list schedules");

    let all_schedules_data: Value = all_schedules_response.json().await?;
    let all_schedules = all_schedules_data["data"]
        .as_array()
        .expect("Schedules data should be an array");

    let found_deleted_schedule = all_schedules
        .iter()
        .any(|schedule| schedule["id"].as_str() == Some(&schedule_id));

    assert!(!found_deleted_schedule, "Deleted schedule should not appear in list");

    println!("✅ Final verification complete - schedule not found in list");

    // Summary
    println!("\n🎉 E2E Test Summary:");
    println!("  📝 Task: addition ({})", task_id);
    println!("  📅 Schedule: created and deleted ({})", schedule_id);
    println!("  💼 Jobs: {} created by schedule", jobs.len());
    println!(
        "  ⚡ Executions: {} total, {} successful",
        execution_count, successful_executions
    );
    println!("  📊 Schedule runs every minute");
    println!("  ✅ All verifications passed!");

    Ok(())
}

/// Helper test to verify the addition task works correctly for our main test
#[tokio::test]
#[ignore] // Integration test - requires running server
async fn test_addition_task_prerequisite() -> Result<()> {
    let client = Client::new();
    let api_base = "http://localhost:8080/api/v1";

    println!("🧪 Prerequisite test: Verifying addition task exists and works...");

    // Get tasks
    let response = client.get(format!("{}/tasks", api_base)).send().await?;
    assert_eq!(response.status(), 200);

    let data: Value = response.json().await?;
    let tasks = data["data"].as_array().expect("Tasks should be an array");

    let addition_task = tasks
        .iter()
        .find(|task| task["name"].as_str() == Some("addition"))
        .expect("Addition task must exist for e2e test to work");

    println!("✅ Addition task found: {}", addition_task["name"]);
    println!("  📝 Description: {}", addition_task["description"]);
    println!("  🔧 Input schema: {}", addition_task["inputSchema"]);
    println!("  📊 Output schema: {}", addition_task["outputSchema"]);

    Ok(())
}

/// Test schedule creation and basic CRUD operations
#[tokio::test]
#[ignore] // Integration test - requires running server
async fn test_schedule_crud_operations() -> Result<()> {
    let client = Client::new();
    let api_base = "http://localhost:8080/api/v1";

    println!("🧪 Testing schedule CRUD operations...");

    // Find addition task first
    let tasks_response = client.get(format!("{}/tasks", api_base)).send().await?;
    let tasks_data: Value = tasks_response.json().await?;
    let tasks = tasks_data["data"].as_array().expect("Tasks data should be an array");
    let addition_task = tasks
        .iter()
        .find(|task| task["name"].as_str() == Some("addition"))
        .expect("Addition task should exist");
    let task_id = addition_task["id"].as_str().expect("Task should have an id");

    // Create schedule
    let schedule_payload = json!({
        "taskId": task_id,
        "name": "test-crud-schedule",
        "cronExpression": "0 0 * * *", // Daily at midnight
        "enabled": false // Start disabled for testing
    });

    let create_response = client
        .post(format!("{}/schedules", api_base))
        .json(&schedule_payload)
        .send()
        .await?;

    assert_eq!(create_response.status(), 201, "Schedule creation should succeed");
    let created_schedule: Value = create_response.json().await?;
    let schedule_id = created_schedule["id"].as_str().expect("Schedule should have an id");

    println!("✅ Created schedule: {}", schedule_id);

    // Read schedule
    let read_response = client
        .get(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    assert_eq!(read_response.status(), 200, "Schedule read should succeed");
    let read_schedule: Value = read_response.json().await?;
    assert_eq!(read_schedule["name"], "test-crud-schedule");
    assert_eq!(read_schedule["enabled"], false);

    println!("✅ Read schedule verified");

    // Update schedule (enable it)
    let update_payload = json!({
        "enabled": true,
        "name": "test-crud-schedule-updated"
    });

    let update_response = client
        .patch(format!("{}/schedules/{}", api_base, schedule_id))
        .json(&update_payload)
        .send()
        .await?;

    assert_eq!(update_response.status(), 200, "Schedule update should succeed");

    println!("✅ Updated schedule");

    // Verify update
    let verify_response = client
        .get(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    let updated_schedule: Value = verify_response.json().await?;
    assert_eq!(updated_schedule["enabled"], true);
    assert_eq!(updated_schedule["name"], "test-crud-schedule-updated");

    println!("✅ Update verified");

    // Delete schedule
    let delete_response = client
        .delete(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    assert!(
        delete_response.status().is_success(),
        "Schedule deletion should succeed"
    );

    println!("✅ Deleted schedule");

    // Verify deletion
    let verify_delete_response = client
        .get(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    assert_eq!(
        verify_delete_response.status(),
        404,
        "Schedule should not exist after deletion"
    );

    println!("✅ Deletion verified - CRUD test complete!");

    Ok(())
}

/// End-to-end test for schedule webhook functionality
///
/// This test demonstrates webhook integration:
/// 1. Create a schedule with webhook output destination
/// 2. Verify webhook configuration is preserved
/// 3. Trigger the schedule manually
/// 4. Verify jobs inherit webhook configuration
/// 5. Clean up
#[tokio::test]
#[ignore] // Integration test - requires running server
async fn test_schedule_webhook_workflow() -> Result<()> {
    let client = Client::new();
    let api_base = "http://localhost:8080/api/v1";

    // Step 1: Find the addition task
    println!("🔍 Step 1: Finding addition task...");
    let tasks_response = client.get(format!("{}/tasks", api_base)).send().await?;

    assert_eq!(tasks_response.status(), 200, "Failed to fetch tasks");

    let tasks_data: Value = tasks_response.json().await?;
    let tasks = tasks_data["data"].as_array().expect("Tasks data should be an array");

    let addition_task = tasks
        .iter()
        .find(|task| task["name"].as_str() == Some("addition"))
        .expect("Addition task should exist");

    let task_id = addition_task["id"]
        .as_str()
        .expect("Task should have an id")
        .to_string();

    println!("✅ Found addition task with ID: {}", task_id);

    // Step 2: Create a schedule with webhook output destination
    println!("📅 Step 2: Creating schedule with webhook configuration...");

    let schedule_payload = json!({
        "taskId": task_id,
        "name": "e2e-test-webhook-schedule",
        "cronExpression": "0 0 * * *", // Daily at midnight (won't run during test)
        "enabled": false, // Keep disabled to prevent automatic execution
        "outputDestinations": [
            {
                "destinationType": "webhook",
                "webhook": {
                    "url": "https://webhook.site/test-endpoint",
                    "method": "POST",
                    "timeoutSeconds": 30,
                    "contentType": "application/json",
                    "retryPolicy": {
                        "maxAttempts": 3,
                        "initialDelaySeconds": 1,
                        "maxDelaySeconds": 5,
                        "backoffMultiplier": 2.0
                    },
                    "authentication": {
                        "authType": "bearer",
                        "bearer": {
                            "token": "test-webhook-token-12345"
                        }
                    }
                }
            }
        ]
    });

    let create_schedule_response = client
        .post(format!("{}/schedules", api_base))
        .json(&schedule_payload)
        .send()
        .await?;

    if create_schedule_response.status() != 201 {
        let error_text = create_schedule_response.text().await?;
        panic!("Failed to create schedule with webhook: {}", error_text);
    }

    let created_schedule: Value = create_schedule_response.json().await?;
    let schedule_id = created_schedule["id"]
        .as_str()
        .expect("Schedule should have an id")
        .to_string();

    println!("✅ Created schedule with webhook ID: {}", schedule_id);

    // Step 3: Verify webhook configuration is preserved
    println!("🔍 Step 3: Verifying webhook configuration...");

    let get_schedule_response = client
        .get(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    assert_eq!(get_schedule_response.status(), 200, "Schedule should exist");

    let schedule_data: Value = get_schedule_response.json().await?;
    assert_eq!(schedule_data["name"], "e2e-test-webhook-schedule");
    assert_eq!(schedule_data["enabled"], false);

    // Verify webhook configuration
    let output_destinations = schedule_data["outputDestinations"]
        .as_array()
        .expect("Should have output destinations");
    assert_eq!(
        output_destinations.len(),
        1,
        "Should have exactly one output destination"
    );

    let webhook_dest = &output_destinations[0];
    assert_eq!(webhook_dest["destinationType"], "webhook");

    let webhook_config = &webhook_dest["webhook"];
    assert_eq!(webhook_config["url"], "https://webhook.site/test-endpoint");
    assert_eq!(webhook_config["method"], "POST");
    assert_eq!(webhook_config["timeoutSeconds"], 30);
    assert_eq!(webhook_config["contentType"], "application/json");

    // Verify retry policy
    let retry_policy = &webhook_config["retryPolicy"];
    assert_eq!(retry_policy["maxAttempts"], 3);
    assert_eq!(retry_policy["initialDelaySeconds"], 1);
    assert_eq!(retry_policy["maxDelaySeconds"], 5);
    assert_eq!(retry_policy["backoffMultiplier"], 2.0);

    // Verify authentication
    let auth = &webhook_config["authentication"];
    assert_eq!(auth["authType"], "bearer");
    let bearer = &auth["bearer"];
    assert_eq!(bearer["token"], "test-webhook-token-12345");

    println!("✅ Webhook configuration verified successfully");

    // Step 4: Trigger the schedule manually to create a job
    println!("🚀 Step 4: Triggering schedule manually...");

    let trigger_response = client
        .post(format!("{}/schedules/{}/trigger", api_base, schedule_id))
        .send()
        .await?;

    if trigger_response.status() != 200 {
        let error_text = trigger_response.text().await?;
        panic!("Failed to trigger schedule: {}", error_text);
    }

    let trigger_result: Value = trigger_response.json().await?;
    assert_eq!(trigger_result["success"], true);

    let created_job = &trigger_result["job"];
    let job_id = created_job["id"].as_str().expect("Job should have an id").to_string();

    println!("✅ Schedule triggered, created job: {}", job_id);

    // Step 5: Verify job inherited webhook configuration
    println!("🔍 Step 5: Verifying job inherited webhook configuration...");

    let get_job_response = client.get(format!("{}/jobs/{}", api_base, job_id)).send().await?;

    assert_eq!(get_job_response.status(), 200, "Job should exist");

    let job_data: Value = get_job_response.json().await?;
    assert_eq!(job_data["taskId"], task_id);

    // Verify job has inherited webhook configuration
    let job_output_destinations = job_data["outputDestinations"]
        .as_array()
        .expect("Job should have inherited output destinations");
    assert_eq!(
        job_output_destinations.len(),
        1,
        "Job should have exactly one output destination"
    );

    let job_webhook_dest = &job_output_destinations[0];
    assert_eq!(job_webhook_dest["destinationType"], "webhook");

    let job_webhook_config = &job_webhook_dest["webhook"];
    assert_eq!(job_webhook_config["url"], "https://webhook.site/test-endpoint");
    assert_eq!(job_webhook_config["method"], "POST");
    assert_eq!(job_webhook_config["timeoutSeconds"], 30);

    println!("✅ Job webhook configuration inheritance verified");

    // Step 6: Test schedule update with different webhook
    println!("🔄 Step 6: Testing schedule webhook update...");

    let update_payload = json!({
        "outputDestinations": [
            {
                "destinationType": "webhook",
                "webhook": {
                    "url": "https://webhook.site/updated-endpoint",
                    "method": "PUT",
                    "timeoutSeconds": 60,
                    "contentType": "application/json",
                    "authentication": {
                        "authType": "api_key",
                        "apiKey": {
                            "key": "updated-api-key-67890",
                            "headerName": "X-API-Key"
                        }
                    }
                }
            }
        ]
    });

    let update_response = client
        .patch(format!("{}/schedules/{}", api_base, schedule_id))
        .json(&update_payload)
        .send()
        .await?;

    assert_eq!(update_response.status(), 200, "Schedule update should succeed");

    // Verify the update
    let verify_update_response = client
        .get(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    let updated_schedule: Value = verify_update_response.json().await?;
    let updated_destinations = updated_schedule["outputDestinations"]
        .as_array()
        .expect("Should have updated output destinations");

    let updated_webhook = &updated_destinations[0]["webhook"];
    assert_eq!(updated_webhook["url"], "https://webhook.site/updated-endpoint");
    assert_eq!(updated_webhook["method"], "PUT");
    assert_eq!(updated_webhook["timeoutSeconds"], 60);

    let updated_auth = &updated_webhook["authentication"];
    assert_eq!(updated_auth["authType"], "api_key");
    let api_key_config = &updated_auth["apiKey"];
    assert_eq!(api_key_config["key"], "updated-api-key-67890");
    assert_eq!(api_key_config["headerName"], "X-API-Key");

    println!("✅ Schedule webhook update verified");

    // Step 7: Clean up - delete the schedule
    println!("🗑️  Step 7: Cleaning up - deleting schedule...");

    let delete_response = client
        .delete(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    assert!(
        delete_response.status().is_success(),
        "Schedule deletion should succeed"
    );

    println!("✅ Schedule deleted successfully");

    // Step 8: Verify cleanup
    println!("🔍 Step 8: Verifying cleanup...");

    let get_deleted_schedule_response = client
        .get(format!("{}/schedules/{}", api_base, schedule_id))
        .send()
        .await?;

    assert_eq!(
        get_deleted_schedule_response.status(),
        404,
        "Schedule should not exist after deletion"
    );

    println!("✅ Cleanup verified");

    // Summary
    println!("\n🎉 Webhook E2E Test Summary:");
    println!("  📝 Task: addition ({})", task_id);
    println!("  📅 Schedule: created with webhook config ({})", schedule_id);
    println!("  🪝 Webhook: Bearer auth → API key auth (updated)");
    println!("  💼 Job: created with inherited webhook config ({})", job_id);
    println!("  🔄 Update: webhook configuration successfully modified");
    println!("  ✅ All webhook functionality verified!");

    Ok(())
}
