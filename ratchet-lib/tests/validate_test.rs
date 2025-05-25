use ratchet_lib::task::{Task, TaskError};
use std::fs;
use tempfile::tempdir;

// Create a test task with valid files
fn create_valid_task() -> tempfile::TempDir {
    let temp_dir = tempdir().unwrap();
    let task_dir = temp_dir.path();
    
    // Create metadata.json
    let metadata = r#"{
        "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
        "version": "1.0.0",
        "label": "Addition Task",
        "description": "This is a sample task that adds two numbers together."
    }"#;
    fs::write(task_dir.join("metadata.json"), metadata).unwrap();
    
    // Create input.schema.json
    let input_schema = r#"{
        "type": "object",
        "properties": {
            "num1": { "type": "number" },
            "num2": { "type": "number" }
        },
        "required": ["num1", "num2"]
    }"#;
    fs::write(task_dir.join("input.schema.json"), input_schema).unwrap();
    
    // Create output.schema.json
    let output_schema = r#"{
        "type": "object",
        "properties": {
            "sum": { "type": "number" }
        },
        "required": ["sum"]
    }"#;
    fs::write(task_dir.join("output.schema.json"), output_schema).unwrap();
    
    // Create valid main.js
    let main_js = r#"(function(input) {
        const {num1, num2} = input;
        
        if (typeof num1 !== 'number' || typeof num2 !== 'number') {
          throw new Error('num1 and num2 must be numbers');
        }
        
        return {
          sum: num1 + num2
        }
    })"#;
    fs::write(task_dir.join("main.js"), main_js).unwrap();
    
    temp_dir
}

// Create a test task with invalid JavaScript
fn create_invalid_js_task() -> tempfile::TempDir {
    let temp_dir = tempdir().unwrap();
    let task_dir = temp_dir.path();
    
    // Create metadata.json
    let metadata = r#"{
        "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
        "version": "1.0.0",
        "label": "Invalid JS Task",
        "description": "This task has syntax errors in the JavaScript."
    }"#;
    fs::write(task_dir.join("metadata.json"), metadata).unwrap();
    
    // Create input.schema.json
    let input_schema = r#"{
        "type": "object",
        "properties": {
            "num1": { "type": "number" },
            "num2": { "type": "number" }
        },
        "required": ["num1", "num2"]
    }"#;
    fs::write(task_dir.join("input.schema.json"), input_schema).unwrap();
    
    // Create output.schema.json
    let output_schema = r#"{
        "type": "object",
        "properties": {
            "sum": { "type": "number" }
        },
        "required": ["sum"]
    }"#;
    fs::write(task_dir.join("output.schema.json"), output_schema).unwrap();
    
    // Create invalid main.js with syntax error
    let main_js = r#"(function(input) {
        const {num1, num2} = input;
        
        if (typeof num1 !== 'number' || typeof num2 !== 'number') {
          throw new Error('num1 and num2 must be numbers');
        
        // Missing closing brace for if statement
        
        return {
          sum: num1 + num2
        }
    })"#;
    fs::write(task_dir.join("main.js"), main_js).unwrap();
    
    temp_dir
}

// Create a test task with invalid JSON schema
fn create_invalid_schema_task() -> tempfile::TempDir {
    let temp_dir = tempdir().unwrap();
    let task_dir = temp_dir.path();
    
    // Create metadata.json
    let metadata = r#"{
        "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
        "version": "1.0.0",
        "label": "Invalid Schema Task",
        "description": "This task has an invalid JSON schema."
    }"#;
    fs::write(task_dir.join("metadata.json"), metadata).unwrap();
    
    // Create invalid input.schema.json (not a JSON object)
    let input_schema = r#""This is not a valid JSON schema""#;
    fs::write(task_dir.join("input.schema.json"), input_schema).unwrap();
    
    // Create valid output.schema.json
    let output_schema = r#"{
        "type": "object",
        "properties": {
            "sum": { "type": "number" }
        },
        "required": ["sum"]
    }"#;
    fs::write(task_dir.join("output.schema.json"), output_schema).unwrap();
    
    // Create valid main.js
    let main_js = r#"(function(input) {
        return { sum: 42 };
    })"#;
    fs::write(task_dir.join("main.js"), main_js).unwrap();
    
    temp_dir
}

// Create a task that doesn't return a function
fn create_non_function_task() -> tempfile::TempDir {
    let temp_dir = tempdir().unwrap();
    let task_dir = temp_dir.path();
    
    // Create metadata.json
    let metadata = r#"{
        "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
        "version": "1.0.0",
        "label": "Non-Function Task",
        "description": "This task doesn't return a function."
    }"#;
    fs::write(task_dir.join("metadata.json"), metadata).unwrap();
    
    // Create input.schema.json
    let input_schema = r#"{
        "type": "object",
        "properties": {
            "num1": { "type": "number" },
            "num2": { "type": "number" }
        },
        "required": ["num1", "num2"]
    }"#;
    fs::write(task_dir.join("input.schema.json"), input_schema).unwrap();
    
    // Create output.schema.json
    let output_schema = r#"{
        "type": "object",
        "properties": {
            "sum": { "type": "number" }
        },
        "required": ["sum"]
    }"#;
    fs::write(task_dir.join("output.schema.json"), output_schema).unwrap();
    
    // Create main.js that doesn't return a function
    let main_js = r#"
        // This just returns an object, not a function
        { sum: 42 }
    "#;
    fs::write(task_dir.join("main.js"), main_js).unwrap();
    
    temp_dir
}

#[test]
fn test_validate_valid_task() {
    let temp_dir = create_valid_task();
    let task_path = temp_dir.path();
    
    let mut task = Task::from_fs(task_path).unwrap();
    let result = task.validate();
    
    if let Err(ref e) = result {
        println!("Validation error: {:?}", e);
    }
    
    assert!(result.is_ok(), "Valid task should pass validation");
}

#[test]
fn test_validate_invalid_js() {
    let temp_dir = create_invalid_js_task();
    let task_path = temp_dir.path();
    
    let mut task = Task::from_fs(task_path).unwrap();
    let result = task.validate();
    
    assert!(result.is_err(), "Task with invalid JS should fail validation");
    match result {
        Err(TaskError::JavaScriptParseError(_)) => {}, // Expected error type
        err => panic!("Expected JavaScriptParseError, got {:?}", err),
    }
}

#[test]
fn test_validate_invalid_schema() {
    let temp_dir = create_invalid_schema_task();
    let task_path = temp_dir.path();
    
    let mut task = Task::from_fs(task_path).unwrap();
    let result = task.validate();
    
    assert!(result.is_err(), "Task with invalid schema should fail validation");
    match result {
        Err(TaskError::InvalidJsonSchema(_)) => {}, // Expected error type
        err => panic!("Expected InvalidJsonSchema, got {:?}", err),
    }
}

#[test]
fn test_validate_non_function() {
    let temp_dir = create_non_function_task();
    let task_path = temp_dir.path();
    
    let mut task = Task::from_fs(task_path).unwrap();
    let result = task.validate();
    
    assert!(result.is_err(), "Task not returning a function should fail validation");
    match result {
        Err(TaskError::JavaScriptParseError(_)) => {}, // Expected error type
        err => panic!("Expected JavaScriptParseError, got {:?}", err),
    }
}