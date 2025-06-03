use ratchet_lib::{
    config::DatabaseConfig,
    database::{connection::DatabaseConnection, repositories::RepositoryFactory},
    task::loader::load_from_directory,
    output::OutputDestinationConfig,
    types::HttpMethod,
};
use axum::{
    routing::post,
    extract::{Json, State},
    Router,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::net::TcpListener;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WebhookPayload {
    result: Option<Value>,
}

#[derive(Clone)]
struct WebhookState {
    received_payloads: Arc<Mutex<Vec<WebhookPayload>>>,
}

async fn webhook_handler(
    State(state): State<WebhookState>,
    Json(payload): Json<Value>,
) -> StatusCode {
    println!("Webhook received: {:?}", payload);
    state.received_payloads.lock().unwrap().push(WebhookPayload {
        result: Some(payload),
    });
    StatusCode::OK
}

async fn start_webhook_server() -> (SocketAddr, WebhookState) {
    let state = WebhookState {
        received_payloads: Arc::new(Mutex::new(Vec::new())),
    };
    
    let app = Router::new()
        .route("/webhook", post(webhook_handler))
        .with_state(state.clone());
    
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        axum::Server::from_tcp(listener.into_std().unwrap())
            .unwrap()
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
    
    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    (addr, state)
}

#[tokio::test]
async fn test_addition_task_with_webhook() {
    // Start webhook server
    let (webhook_addr, webhook_state) = start_webhook_server().await;
    let webhook_url = format!("http://{}/webhook", webhook_addr);
    println!("Webhook server listening on: {}", webhook_url);
    
    // Set up test database
    let db_config = DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        max_connections: 5,
        connection_timeout: Duration::from_secs(10),
    };
    
    let db_connection = DatabaseConnection::new(db_config.clone()).await.unwrap();
    let repositories = RepositoryFactory::new(db_connection.clone());
    
    // Run migrations
    use ratchet_lib::database::migrations::Migrator;
    use sea_orm_migration::MigratorTrait;
    Migrator::up(db_connection.get_connection(), None)
        .await
        .unwrap();
    
    // Get the path to sample tasks
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let sample_tasks_path = project_root.join("sample").join("js-tasks");
    
    // Load the addition task directly
    let addition_task_path = sample_tasks_path.join("addition");
    let addition_task = load_from_directory(&addition_task_path)
        .expect("Failed to load addition task");
    
    println!("Loaded task: {} ({})", addition_task.metadata.label, addition_task.metadata.uuid);
    
    // Create task in database
    use ratchet_lib::database::entities::tasks::ActiveModel as TaskActiveModel;
    use sea_orm::{ActiveModelTrait, Set};
    
    let task_model = TaskActiveModel {
        uuid: Set(addition_task.uuid()),
        name: Set(addition_task.metadata.label.clone()),
        description: Set(Some(addition_task.metadata.description.clone())),
        version: Set(addition_task.metadata.version.clone()),
        path: Set(addition_task_path.to_string_lossy().to_string()),
        metadata: Set(serde_json::to_value(&addition_task.metadata).unwrap()),
        input_schema: Set(addition_task.input_schema.clone()),
        output_schema: Set(addition_task.output_schema.clone()),
        enabled: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        validated_at: Set(Some(Utc::now())),
        ..Default::default()
    };
    
    let created_task = task_model.insert(db_connection.get_connection()).await.unwrap();
    println!("Created task with ID: {}", created_task.id);
    
    // Create a job with webhook output destination
    use ratchet_lib::database::entities::jobs::{Model as Job, JobPriority};
    
    let output_destinations = vec![
        OutputDestinationConfig::Webhook {
            url: webhook_url.clone(),
            method: HttpMethod::Post,
            headers: std::collections::HashMap::new(),
            timeout: Duration::from_secs(30),
            retry_policy: ratchet_lib::output::RetryPolicy::default(),
            auth: None,
            content_type: Some("application/json".to_string()),
        }
    ];
    
    let mut job = Job::new(
        created_task.id,
        json!({
            "num1": 1,
            "num2": 2
        }),
        JobPriority::Normal,
    );
    job.output_destinations = Some(serde_json::to_value(&output_destinations).unwrap());
    
    let created_job = repositories.job_repository()
        .create(job)
        .await
        .unwrap();
    
    println!("Created job with ID: {} (UUID: {})", created_job.id, created_job.uuid);
    
    // Execute the task using the JS executor directly
    use ratchet_lib::js_executor::execute_task;
    use ratchet_lib::http::HttpManager;
    
    let http_manager = HttpManager::new();
    let input_data = json!({
        "num1": 1,
        "num2": 2
    });
    
    match execute_task(
        &mut addition_task.clone(),
        input_data.clone(),
        &http_manager,
    ).await {
        Ok(result) => {
            println!("Task execution succeeded: {:?}", result);
            assert_eq!(result, json!({"sum": 3}));
            
            // Mark job as completed
            repositories.job_repository()
                .mark_completed(created_job.id)
                .await
                .unwrap();
            
            // Manually deliver the output to webhook
            let webhook_payload = json!({
                "job_id": created_job.uuid.to_string(),
                "task_id": created_task.id.to_string(),
                "task_name": created_task.name,
                "status": "completed",
                "output": result,
                "timestamp": Utc::now().to_rfc3339(),
            });
            
            // Send to webhook
            let client = reqwest::Client::new();
            let response = client
                .post(&webhook_url)
                .json(&webhook_payload)
                .send()
                .await
                .unwrap();
            
            assert_eq!(response.status(), 200);
            
            // Wait a bit for processing
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Check that the webhook was called
            let payloads = webhook_state.received_payloads.lock().unwrap();
            assert!(!payloads.is_empty(), "No webhook payloads received");
            
            let payload = &payloads[0].result;
            assert!(payload.is_some());
            
            let webhook_data = payload.as_ref().unwrap();
            assert_eq!(webhook_data["status"], "completed");
            assert_eq!(webhook_data["output"]["sum"], 3);
        }
        Err(e) => {
            panic!("Task execution failed: {:?}", e);
        }
    }
    
    println!("Integration test completed successfully!");
}