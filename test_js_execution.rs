use ratchet_js::{JsTask, JsTaskRunner, ExecutionContext as JsExecutionContext};
use serde_json::json;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let heartbeat_js = r#"
async function main(input) {
    const startTime = Date.now();
    
    try {
        // Basic system status
        const timestamp = new Date().toISOString();
        
        // Get process uptime (simulated for Boa engine)
        const uptimeSeconds = Math.floor(Math.random() * 10000); // Simulated uptime
        
        // Basic system information
        const systemInfo = {
            version: "ratchet-0.1.0",
            uptime_seconds: uptimeSeconds,
            active_jobs: 0 // This would be populated by the execution context
        };
        
        // Calculate execution time
        const executionTime = Date.now() - startTime;
        
        // Return success response
        return {
            status: "ok",
            timestamp: timestamp,
            message: `Heartbeat successful - system running normally (${executionTime}ms)`,
            system_info: systemInfo
        };
        
    } catch (error) {
        // Return error response if something goes wrong
        return {
            status: "error",
            timestamp: new Date().toISOString(),
            message: `Heartbeat failed: ${error.message}`,
            system_info: {
                version: "ratchet-0.1.0",
                uptime_seconds: 0,
                active_jobs: 0
            }
        };
    }
}

// Export for CommonJS compatibility
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { main };
}
"#;

    let js_task = JsTask {
        name: "heartbeat".to_string(),
        content: heartbeat_js.to_string(),
        input_schema: None,
        output_schema: None,
    };

    let execution_context = Some(JsExecutionContext {
        execution_id: "test-exec-123".to_string(),
        task_id: "heartbeat".to_string(),
        task_version: "1.0.0".to_string(),
        job_id: None,
    });

    let runner = JsTaskRunner::new();
    
    println!("Executing JavaScript task...");
    let result = runner.execute_task(&js_task, json!({}), execution_context).await;
    
    match result {
        Ok(output) => {
            println!("✅ Success! Output: {}", serde_json::to_string_pretty(&output).unwrap());
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
}