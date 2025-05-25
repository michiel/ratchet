use crate::js_executor::{execute_task, JsExecutionError};
use crate::task::{Task, TaskError};
use anyhow::Result;
use serde_json::{Value as JsonValue};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during test execution
#[derive(Error, Debug)]
pub enum TestError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Task error: {0}")]
    TaskError(#[from] TaskError),

    #[error("JSON parse error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Execution error: {0}")]
    ExecutionError(#[from] JsExecutionError),

    #[error("Invalid test file: {0}")]
    InvalidTestFile(String),

    #[error("No tests directory found")]
    NoTestsDirectory,
}

/// Represents a single test case
#[derive(Debug)]
pub struct TestCase {
    pub file_path: PathBuf,
    pub input: JsonValue,
    pub expected_output: JsonValue,
    pub mock: Option<JsonValue>,
}

/// Represents the result of a test case
#[derive(Debug)]
pub struct TestResult {
    pub file_path: PathBuf,
    pub passed: bool,
    pub actual_output: Option<JsonValue>,
    pub error_message: Option<String>,
}

/// Represents a test summary
#[derive(Debug)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<TestResult>,
}

impl TestSummary {
    /// Returns true if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

/// Load test cases from a tests directory
pub fn load_test_cases(task_path: &Path) -> Result<Vec<TestCase>, TestError> {
    let tests_dir = task_path.join("tests");
    
    if !tests_dir.exists() || !tests_dir.is_dir() {
        return Err(TestError::NoTestsDirectory);
    }
    
    let mut test_cases = Vec::new();
    
    for entry in fs::read_dir(&tests_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        // Skip non-JSON files
        if path.extension().map_or(true, |ext| ext != "json") {
            continue;
        }
        
        // Read the test file
        let content = fs::read_to_string(&path)?;
        let test_json: JsonValue = serde_json::from_str(&content)?;
        
        // Validate test file structure
        let input = test_json.get("input").ok_or_else(|| {
            TestError::InvalidTestFile(format!("Missing 'input' field in test file: {:?}", path))
        })?;
        
        let expected_output = test_json.get("expected_output").ok_or_else(|| {
            TestError::InvalidTestFile(format!("Missing 'expected_output' field in test file: {:?}", path))
        })?;
        
        // Check for optional mock data
        let mock = test_json.get("mock").map(|m| m.clone());
        
        test_cases.push(TestCase {
            file_path: path,
            input: input.clone(),
            expected_output: expected_output.clone(),
            mock,
        });
    }
    
    // Sort test cases by file name for consistent execution order
    test_cases.sort_by(|a, b| {
        a.file_path.file_name().unwrap().cmp(b.file_path.file_name().unwrap())
    });
    
    Ok(test_cases)
}

/// Run a single test case
pub async fn run_test_case(task: &mut Task, test_case: &TestCase) -> TestResult {
    // Setup mock data if provided
    if let Some(mock) = &test_case.mock {
        crate::http::set_mock_http_data(Some(mock.clone()));
    } else {
        crate::http::set_mock_http_data(None);
    }
    
    // Execute the task
    let result = execute_task(task, test_case.input.clone()).await;
    
    // Clear mock data after the test
    crate::http::set_mock_http_data(None);
    
    match result {
        Ok(output) => {
            // Compare actual output with expected output
            let passed = output == test_case.expected_output;
            
            TestResult {
                file_path: test_case.file_path.clone(),
                passed,
                actual_output: Some(output),
                error_message: None,
            }
        },
        Err(err) => {
            TestResult {
                file_path: test_case.file_path.clone(),
                passed: false,
                actual_output: None,
                error_message: Some(err.to_string()),
            }
        }
    }
}

/// Run all test cases for a task
pub async fn run_tests(task_path: &str) -> Result<TestSummary, TestError> {
    // Load the task
    let mut task = Task::from_fs(task_path)?;
    
    // Load test cases
    let test_cases = load_test_cases(&task.path)?;
    
    if test_cases.is_empty() {
        return Ok(TestSummary {
            total: 0,
            passed: 0,
            failed: 0,
            results: Vec::new(),
        });
    }
    
    // Run each test case
    let mut results = Vec::new();
    for test_case in &test_cases {
        let result = run_test_case(&mut task, test_case).await;
        results.push(result);
    }
    
    // Count passed and failed tests
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;
    
    Ok(TestSummary {
        total,
        passed,
        failed,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio_test::block_on;
    use crate::task::{TaskMetadata, TaskType};
    use uuid::Uuid;
    use std::fs::File;
    use std::io::Write;
    
    fn create_test_task_with_tests() -> Result<(PathBuf, tempfile::TempDir), std::io::Error> {
        let temp_dir = tempdir()?;
        let task_dir = temp_dir.path().to_path_buf();
        
        // Create metadata.json
        let metadata = r#"{
            "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
            "version": "1.0.0",
            "label": "Test Task",
            "description": "Test task for unit testing"
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
        
        // Create tests directory
        let tests_dir = task_dir.join("tests");
        fs::create_dir(&tests_dir)?;
        
        // Create test files
        let test1 = r#"{
            "input": {
                "num1": 5,
                "num2": 7
            },
            "expected_output": {
                "sum": 12
            }
        }"#;
        fs::write(tests_dir.join("test-001.json"), test1)?;
        
        let test2 = r#"{
            "input": {
                "num1": 10,
                "num2": 20
            },
            "expected_output": {
                "sum": 30
            }
        }"#;
        fs::write(tests_dir.join("test-002.json"), test2)?;
        
        // Create a failing test
        let test3 = r#"{
            "input": {
                "num1": 1,
                "num2": 2
            },
            "expected_output": {
                "sum": 1234
            }
        }"#;
        fs::write(tests_dir.join("test-003.json"), test3)?;
        
        // Create an invalid test (missing expected_output)
        let invalid_test = r#"{
            "input": {
                "num1": 1,
                "num2": 2
            }
        }"#;
        fs::write(tests_dir.join("invalid-test.json"), invalid_test)?;
        
        Ok((task_dir, temp_dir))
    }
    
    #[test]
    fn test_load_test_cases() {
        if let Ok((task_dir, _temp_dir)) = create_test_task_with_tests() {
            let test_cases = load_test_cases(&task_dir).unwrap();
            
            // We should have 3 valid test cases (the invalid one should be rejected)
            assert_eq!(test_cases.len(), 3);
            
            // Check first test case
            assert_eq!(test_cases[0].input["num1"], 5);
            assert_eq!(test_cases[0].input["num2"], 7);
            assert_eq!(test_cases[0].expected_output["sum"], 12);
            
            // Attempting to load the invalid test file should result in an error
            let invalid_test_path = task_dir.join("tests").join("invalid-test.json");
            let test_json: JsonValue = serde_json::from_str(
                &fs::read_to_string(invalid_test_path).unwrap()
            ).unwrap();
            
            let input = test_json.get("input").unwrap();
            let expected_output = test_json.get("expected_output");
            assert!(expected_output.is_none());
        }
    }
    
    #[test]
    fn test_run_tests() {
        block_on(async {
            if let Ok((task_dir, _temp_dir)) = create_test_task_with_tests() {
                let summary = run_tests(task_dir.to_str().unwrap()).await.unwrap();
                
                // We should have 3 test cases
                assert_eq!(summary.total, 3);
                
                // Two tests should pass and one should fail
                assert_eq!(summary.passed, 2);
                assert_eq!(summary.failed, 1);
                
                // Check individual results
                let passed_tests = summary.results.iter().filter(|r| r.passed).count();
                assert_eq!(passed_tests, 2);
                
                let failed_tests = summary.results.iter().filter(|r| !r.passed).count();
                assert_eq!(failed_tests, 1);
                
                // The failing test should be the third one
                let failing_test = summary.results.iter().find(|r| !r.passed).unwrap();
                assert_eq!(failing_test.file_path.file_name().unwrap(), "test-003.json");
                assert!(failing_test.actual_output.is_some());
                assert_eq!(failing_test.actual_output.as_ref().unwrap()["sum"], 3);
                assert_ne!(failing_test.actual_output.as_ref().unwrap()["sum"], 1234);
            }
        });
    }
    
    #[test]
    fn test_no_tests_directory() {
        block_on(async {
            let temp_dir = tempdir().unwrap();
            let task_dir = temp_dir.path().to_path_buf();
            
            // Create a task without a tests directory
            let metadata = r#"{
                "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
                "version": "1.0.0",
                "label": "Test Task",
                "description": "Test task for unit testing"
            }"#;
            fs::write(task_dir.join("metadata.json"), metadata).unwrap();
            
            let result = run_tests(task_dir.to_str().unwrap()).await;
            assert!(matches!(result, Err(TestError::NoTestsDirectory)));
        });
    }
}