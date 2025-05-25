use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ratchet_lib::task::Task;
use serde_json::{from_str, json, to_string_pretty, Value as JsonValue};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single task from a file system path
    RunOnce {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,
        
        /// JSON input for the task (example: --input-json='{"num1":5,"num2":10}')
        #[arg(long, value_name = "JSON")]
        input_json: Option<String>,
    },
    
    /// Validate a task's structure and syntax
    Validate {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,
    },
    
    /// Run tests for a task
    Test {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,
    },
}

/// Parse JSON input string into a JsonValue
fn parse_input_json(input: Option<&String>) -> Result<JsonValue> {
    match input {
        Some(json_str) => {
            from_str(json_str).context("Failed to parse input JSON")
        }
        None => {
            // Default empty JSON object if no input provided
            Ok(json!({}))
        }
    }
}

/// Run a task with the given input
async fn run_task(task_path: &str, input_json: &JsonValue) -> Result<JsonValue> {
    // Load the task from the filesystem
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;
    
    // Execute the task with the provided input
    let result = ratchet_lib::js_executor::execute_task(&mut task, input_json.clone())
        .await
        .context("Failed to execute task")?;
    
    Ok(result)
}

/// Validate a task's structure and syntax
fn validate_task(task_path: &str) -> Result<()> {
    println!("Validating task at: {}", task_path);
    
    // Load the task from the filesystem
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;
    
    // Validate the task
    task.validate()
        .context("Task validation failed")?;
    
    println!("✓ Task validated successfully!");
    println!("  UUID: {}", task.uuid());
    println!("  Label: {}", task.metadata.label);
    println!("  Version: {}", task.metadata.version);
    println!("  Description: {}", task.metadata.description);
    
    Ok(())
}

/// Run tests for a task
async fn test_task(task_path: &str) -> Result<()> {
    println!("Running tests for task at: {}", task_path);
    
    // First validate the task
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;
    
    task.validate()
        .context("Task validation failed")?;
    
    println!("Task validated successfully!");
    println!("  UUID: {}", task.uuid());
    println!("  Label: {}", task.metadata.label);
    println!("  Version: {}", task.metadata.version);
    
    // Run tests
    match ratchet_lib::test::run_tests(task_path).await {
        Ok(summary) => {
            println!("\nTest Results:");
            println!("-------------");
            println!("Total tests: {}", summary.total);
            println!("Passed: {}", summary.passed);
            println!("Failed: {}", summary.failed);
            println!("-------------");
            
            // Print details of failed tests
            if summary.failed > 0 {
                println!("\nFailed Tests:");
                for (i, result) in summary.results.iter().enumerate() {
                    if !result.passed {
                        let file_name = result.file_path.file_name().unwrap().to_string_lossy();
                        println!("\n{}. Test: {}", i + 1, file_name);
                        
                        if let Some(actual) = &result.actual_output {
                            // Get the expected output from the test file
                            let test_file_content = std::fs::read_to_string(&result.file_path)
                                .context(format!("Failed to read test file: {:?}", result.file_path))?;
                            let test_json: JsonValue = serde_json::from_str(&test_file_content)
                                .context(format!("Failed to parse test file: {:?}", result.file_path))?;
                            let expected = test_json.get("expected_output").unwrap();
                            
                            println!("   Expected: {}", serde_json::to_string_pretty(expected)?);
                            println!("   Actual: {}", serde_json::to_string_pretty(actual)?);
                        } else if let Some(error) = &result.error_message {
                            println!("   Error: {}", error);
                        }
                    }
                }
                
                // Return non-zero exit code for CI/CD pipelines
                std::process::exit(1);
            } else if summary.total == 0 {
                println!("\nNo tests found. Create test files in the 'tests' directory.");
            } else {
                println!("\nAll tests passed! ✓");
            }
            
            Ok(())
        },
        Err(err) => {
            match err {
                ratchet_lib::test::TestError::NoTestsDirectory => {
                    println!("\nNo tests directory found.");
                    println!("Create a 'tests' directory with JSON test files to run tests.");
                    println!("Each test file should contain 'input' and 'expected_output' fields.");
                    println!("Example: {{ \"input\": {{ \"num1\": 5, \"num2\": 10 }}, \"expected_output\": {{ \"sum\": 15 }} }}");
                    Ok(())
                },
                _ => {
                    Err(err).context("Test execution failed")
                }
            }
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    // Create a tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    match &cli.command {
        Some(Commands::RunOnce { from_fs, input_json }) => {
            println!("Running task from file system path: {}", from_fs);
            
            // Parse input JSON
            let input = parse_input_json(input_json.as_ref())?;
            
            if input_json.is_some() {
                println!("Using provided input: {}", input_json.as_ref().unwrap());
            }
            
            // Run the task
            let result = runtime.block_on(run_task(from_fs, &input))?;
            
            // Pretty-print the result
            let formatted = to_string_pretty(&result)
                .context("Failed to format result as JSON")?;
                
            println!("Result: {}", formatted);
            Ok(())
        },
        Some(Commands::Validate { from_fs }) => {
            validate_task(from_fs)
        },
        Some(Commands::Test { from_fs }) => {
            runtime.block_on(test_task(from_fs))
        },
        None => {
            println!("No command specified. Use --help to see available commands.");
            Ok(())
        }
    }
}