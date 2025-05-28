use super::*;
use tempfile::tempdir;
use std::io::Write;
use std::fs;
use std::path::Path;

fn create_test_task_files() -> Result<std::path::PathBuf, std::io::Error> {
    let temp_dir = tempdir()?;
    let task_dir = temp_dir.path().to_path_buf();
    
    // Create metadata.json
    let metadata = r#"{
        "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
        "version": "1.0.0",
        "label": "Addition Task",
        "description": "This is a sample task that adds two numbers together."
    }"#;
    fs::write(task_dir.join("metadata.json"), metadata)?;
    
    // Create input.schema.json
    let input_schema = r#"{
        "type": "object",
        "properties": {
            "num1": { "type": "number" },
            "num2": { "type": "number" }
        },
        "required": ["num1", "num2"]
    }"#;
    fs::write(task_dir.join("input.schema.json"), input_schema)?;
    
    // Create output.schema.json
    let output_schema = r#"{
        "type": "object",
        "properties": {
            "sum": { "type": "number" }
        },
        "required": ["sum"]
    }"#;
    fs::write(task_dir.join("output.schema.json"), output_schema)?;
    
    // Create main.js
    let main_js = r#"function(input) {
        const {num1, num2} = input;
        
        if (typeof num1 !== 'number' || typeof num2 !== 'number') {
          throw new Error('num1 and num2 must be numbers');
        }
        
        return {
          sum: num1 + num2
        }
    }"#;
    fs::write(task_dir.join("main.js"), main_js)?;
    
    // Prevent temp_dir from being dropped so the files remain
    std::mem::forget(temp_dir);
    
    Ok(task_dir)
}

#[test]
fn test_load_from_sample() {
    // Test with the sample task in the project
    let sample_path = Path::new("/home/michiel/dev/ratchet/sample/js-tasks/addition");
    if sample_path.exists() {
        let mut task = Task::from_fs(sample_path).unwrap();
        
        assert_eq!(task.metadata.uuid.to_string(), "bd6c6f98-4896-44cc-8c82-30328c3aefda");
        assert_eq!(task.metadata.version, "1.0.0");
        assert_eq!(task.metadata.label, "Addition Task");
        
        match &task.task_type {
            TaskType::JsTask { path, content } => {
                assert!(path.contains("main.js"));
                assert!(content.is_none()); // Content should not be loaded initially
            }
        }
        
        // Test content loading
        task.ensure_content_loaded().unwrap();
        let content = task.get_js_content().unwrap();
        assert!(!content.is_empty());
        
        // Check schema properties
        assert!(task.input_schema.get("properties").is_some());
        assert!(task.output_schema.get("properties").is_some());
    }
}

#[test]
fn test_from_fs() {
    // Create test files
    let test_dir = create_test_task_files().unwrap();
    
    // Load the task
    let mut task = Task::from_fs(&test_dir).unwrap();
    
    // Verify the task properties
    assert_eq!(task.metadata.uuid.to_string(), "bd6c6f98-4896-44cc-8c82-30328c3aefda");
    assert_eq!(task.metadata.version, "1.0.0");
    assert_eq!(task.metadata.label, "Addition Task");
    assert_eq!(task.metadata.description, "This is a sample task that adds two numbers together.");
    
    match &task.task_type {
        TaskType::JsTask { path, content } => {
            assert!(path.contains("main.js"));
            assert!(content.is_none()); // Content should not be loaded initially
        }
    }
    
    // Test loading content
    task.ensure_content_loaded().unwrap();
    
    match &task.task_type {
        TaskType::JsTask { content, .. } => {
            assert!(content.is_some()); // Content should be loaded now
            let js_content = content.as_ref().unwrap();
            assert!(js_content.contains("function")); // Check content contains expected text
        }
    }
    
    // Test get_js_content
    let js_content = task.get_js_content().unwrap();
    assert!(js_content.contains("function"));
    
    // Test purge_content
    task.purge_content();
    
    match &task.task_type {
        TaskType::JsTask { content, .. } => {
            assert!(content.is_none()); // Content should be purged
        }
    }
    
    // Check schema properties
    let input_props = task.input_schema.get("properties").unwrap();
    assert!(input_props.get("num1").is_some());
    assert!(input_props.get("num2").is_some());
    
    let output_props = task.output_schema.get("properties").unwrap();
    assert!(output_props.get("sum").is_some());
}

#[test]
fn test_missing_files() {
    let temp_dir = tempdir().unwrap();
    let result = Task::from_fs(temp_dir.path());
    assert!(result.is_err());
    
    // Create just metadata.json
    let metadata = r#"{
        "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
        "version": "1.0.0",
        "label": "Addition Task",
        "description": "This is a sample task that adds two numbers together."
    }"#;
    fs::write(temp_dir.path().join("metadata.json"), metadata).unwrap();
    
    // Should still fail due to missing input schema
    let result = Task::from_fs(temp_dir.path());
    assert!(result.is_err());
}

// Creates a ZIP file with the specified task files
fn create_zip_task() -> Result<(std::path::PathBuf, tempfile::TempDir), std::io::Error> {
    // Create a temporary directory to store the ZIP file
    let temp_dir = tempdir()?;
    
    // Create task files in a subdirectory
    let task_dir = temp_dir.path().join("task");
    fs::create_dir(&task_dir)?;
    
    // Create metadata.json
    let metadata = r#"{
        "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
        "version": "1.0.0",
        "label": "ZIP Task",
        "description": "This is a sample task in a ZIP file."
    }"#;
    fs::write(task_dir.join("metadata.json"), metadata)?;
    
    // Create input.schema.json
    let input_schema = r#"{
        "type": "object",
        "properties": {
            "num1": { "type": "number" },
            "num2": { "type": "number" }
        },
        "required": ["num1", "num2"]
    }"#;
    fs::write(task_dir.join("input.schema.json"), input_schema)?;
    
    // Create output.schema.json
    let output_schema = r#"{
        "type": "object",
        "properties": {
            "sum": { "type": "number" }
        },
        "required": ["sum"]
    }"#;
    fs::write(task_dir.join("output.schema.json"), output_schema)?;
    
    // Create main.js
    let main_js = r#"(function(input) {
        const {num1, num2} = input;
        
        if (typeof num1 !== 'number' || typeof num2 !== 'number') {
          throw new Error('num1 and num2 must be numbers');
        }
        
        return {
          sum: num1 + num2
        }
    })"#;
    fs::write(task_dir.join("main.js"), main_js)?;
    
    // Create the ZIP file
    let zip_path = temp_dir.path().join("task.zip");
    let zip_file = fs::File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(zip_file);
    
    // Add files to the ZIP
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    
    // Add metadata.json
    zip.start_file("task/metadata.json", options)?;
    zip.write_all(metadata.as_bytes())?;
    
    // Add input.schema.json
    zip.start_file("task/input.schema.json", options)?;
    zip.write_all(input_schema.as_bytes())?;
    
    // Add output.schema.json
    zip.start_file("task/output.schema.json", options)?;
    zip.write_all(output_schema.as_bytes())?;
    
    // Add main.js
    zip.start_file("task/main.js", options)?;
    zip.write_all(main_js.as_bytes())?;
    
    // Finalize the ZIP file
    zip.finish()?;
    
    Ok((zip_path, temp_dir))
}

#[test]
fn test_from_zip() {
    // Create a ZIP file with a task
    let (zip_path, _temp_dir) = create_zip_task().unwrap();
    
    // Load the task from the ZIP file
    let mut task = Task::from_fs(&zip_path).unwrap();
    
    // Verify the task properties
    assert_eq!(task.metadata.uuid.to_string(), "bd6c6f98-4896-44cc-8c82-30328c3aefda");
    assert_eq!(task.metadata.version, "1.0.0");
    assert_eq!(task.metadata.label, "ZIP Task");
    assert_eq!(task.metadata.description, "This is a sample task in a ZIP file.");
    
    // The temporary directory should be stored in the task
    assert!(task._temp_dir.is_some());
    
    // Test loading content
    task.ensure_content_loaded().unwrap();
    let content = task.get_js_content().unwrap();
    assert!(content.contains("function"));
    
    // Check schema properties
    let input_props = task.input_schema.get("properties").unwrap();
    assert!(input_props.get("num1").is_some());
    assert!(input_props.get("num2").is_some());
    
    let output_props = task.output_schema.get("properties").unwrap();
    assert!(output_props.get("sum").is_some());
}

#[test]
fn test_sample_zip() {
    // Test with the sample ZIP task in the project
    let sample_path = Path::new("/home/michiel/dev/ratchet/sample/js-tasks/addition.zip");
    if sample_path.exists() {
        let mut task = Task::from_fs(sample_path).unwrap();
        
        assert_eq!(task.metadata.uuid.to_string(), "bd6c6f98-4896-44cc-8c82-30328c3aefda");
        assert_eq!(task.metadata.version, "1.0.0");
        assert_eq!(task.metadata.label, "Addition Task");
        
        // The temporary directory should be stored in the task
        assert!(task._temp_dir.is_some());
        
        // Test content loading
        task.ensure_content_loaded().unwrap();
        let content = task.get_js_content().unwrap();
        assert!(!content.is_empty());
        
        // Check schema properties
        assert!(task.input_schema.get("properties").is_some());
        assert!(task.output_schema.get("properties").is_some());
    }
}