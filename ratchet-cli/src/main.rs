use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ratchet_lib::task::Task;
use serde_json::{from_str, json, to_string_pretty, Value as JsonValue};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use std::path::PathBuf;
use std::fs;
use uuid::Uuid;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set the log level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL", global = true)]
    log_level: Option<String>,

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
        
        /// Record execution to directory with timestamp
        #[arg(long, value_name = "PATH")]
        record: Option<PathBuf>,
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

    /// Replay a task using recorded inputs from a previous session
    Replay {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,

        /// Path to the recording directory with input.json, output.json, etc.
        #[arg(long, value_name = "PATH")]
        recording: PathBuf,
    },

    /// Generate task template files
    Generate {
        #[command(subcommand)]
        generate_cmd: GenerateCommands,
    },
}

#[derive(Subcommand)]
enum GenerateCommands {
    /// Generate a new task template with stub files
    Task {
        /// Path where to create the task directory
        #[arg(long, value_name = "PATH")]
        path: PathBuf,

        /// Task label/name
        #[arg(long, value_name = "STRING")]
        label: Option<String>,

        /// Task description
        #[arg(long, value_name = "STRING")]
        description: Option<String>,

        /// Task version
        #[arg(long, value_name = "STRING")]
        version: Option<String>,
    },
}

/// Initialize tracing with environment variable override support
fn init_tracing(log_level: Option<&String>, record_dir: Option<&PathBuf>) -> Result<()> {
    let env_filter = match log_level {
        Some(level) => {
            // Use provided log level
            EnvFilter::try_new(level).unwrap_or_else(|_| {
                eprintln!("Invalid log level '{}', falling back to 'info'", level);
                EnvFilter::new("info")
            })
        }
        None => {
            // Try environment variable first, then default to info
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        }
    };
    
    if let Some(record_path) = record_dir {
        // Create timestamp directory
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let session_dir = record_path.join(format!("ratchet_session_{}", timestamp));
        fs::create_dir_all(&session_dir).context("Failed to create recording directory")?;
        
        // Create log file appender
        let file_appender = tracing_appender::rolling::never(&session_dir, "tracing.log");
        
        // Setup tracing with both console and file output
        use tracing_subscriber::fmt::writer::MakeWriterExt;
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(std::io::stdout.and(file_appender))
            .init();
            
        // Store the session directory for use by other components
        ratchet_lib::recording::set_recording_dir(session_dir)?;
        
        info!("Recording session to: {:?}", record_path.join(format!("ratchet_session_{}", timestamp)));
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }

    debug!("Tracing initialized");
    Ok(())
}

/// Parse JSON input string into a JsonValue
fn parse_input_json(input: Option<&String>) -> Result<JsonValue> {
    match input {
        Some(json_str) => {
            debug!("Parsing input JSON: {}", json_str);
            from_str(json_str).context("Failed to parse input JSON")
        }
        None => {
            debug!("No input JSON provided, using empty object");
            // Default empty JSON object if no input provided
            Ok(json!({}))
        }
    }
}

/// Run a task with the given input
async fn run_task(task_path: &str, input_json: &JsonValue) -> Result<JsonValue> {
    info!("Loading task from: {}", task_path);

    // Load the task from the filesystem
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;

    debug!("Task loaded: {} ({})", task.metadata.label, task.uuid());

    // Execute the task with the provided input
    info!("Executing task with input");
    let result = ratchet_lib::js_executor::execute_task(&mut task, input_json.clone())
        .await
        .context("Failed to execute task")?;

    info!("Task execution completed successfully");
    Ok(result)
}

/// Validate a task's structure and syntax
fn validate_task(task_path: &str) -> Result<()> {
    info!("Validating task at: {}", task_path);

    // Load the task from the filesystem
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;

    debug!("Task loaded: {} ({})", task.metadata.label, task.uuid());

    // Validate the task
    task.validate().context("Task validation failed")?;

    println!("✓ Task validated successfully!");
    println!("  UUID: {}", task.uuid());
    println!("  Label: {}", task.metadata.label);
    println!("  Version: {}", task.metadata.version);
    println!("  Description: {}", task.metadata.description);

    info!("Task validation completed successfully");
    Ok(())
}

/// Run tests for a task
async fn test_task(task_path: &str) -> Result<()> {
    info!("Running tests for task at: {}", task_path);

    // First validate the task
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;

    debug!("Task loaded: {} ({})", task.metadata.label, task.uuid());

    task.validate().context("Task validation failed")?;

    println!("Task validated successfully!");
    println!("  UUID: {}", task.uuid());
    println!("  Label: {}", task.metadata.label);
    println!("  Version: {}", task.metadata.version);

    // Run tests
    info!("Starting test execution");
    match ratchet_lib::test::run_tests(task_path).await {
        Ok(summary) => {
            info!(
                "Tests completed - Total: {}, Passed: {}, Failed: {}",
                summary.total, summary.passed, summary.failed
            );

            println!("\nTest Results:");
            println!("-------------");
            println!("Total tests: {}", summary.total);
            println!("Passed: {}", summary.passed);
            println!("Failed: {}", summary.failed);
            println!("-------------");

            // Print details of failed tests
            if summary.failed > 0 {
                warn!("Found {} failed tests", summary.failed);
                println!("\nFailed Tests:");
                for (i, result) in summary.results.iter().enumerate() {
                    if !result.passed {
                        let file_name = result.file_path.file_name().unwrap().to_string_lossy();
                        warn!("Test failed: {}", file_name);
                        println!("\n{}. Test: {}", i + 1, file_name);

                        if let Some(actual) = &result.actual_output {
                            // Get the expected output from the test file
                            let test_file_content = std::fs::read_to_string(&result.file_path)
                                .context(format!(
                                    "Failed to read test file: {:?}",
                                    result.file_path
                                ))?;
                            let test_json: JsonValue = serde_json::from_str(&test_file_content)
                                .context(format!(
                                    "Failed to parse test file: {:?}",
                                    result.file_path
                                ))?;
                            let expected = test_json.get("expected_output").unwrap();

                            println!("   Expected: {}", serde_json::to_string_pretty(expected)?);
                            println!("   Actual: {}", serde_json::to_string_pretty(actual)?);
                        } else if let Some(error) = &result.error_message {
                            error!("Test error: {}", error);
                            println!("   Error: {}", error);
                        }
                    }
                }

                // Return non-zero exit code for CI/CD pipelines
                error!("Tests failed, exiting with code 1");
                std::process::exit(1);
            } else if summary.total == 0 {
                warn!("No tests found");
                println!("\nNo tests found. Create test files in the 'tests' directory.");
            } else {
                info!("All tests passed successfully");
                println!("\nAll tests passed! ✓");
            }

            Ok(())
        }
        Err(err) => match err {
            ratchet_lib::test::TestError::NoTestsDirectory => {
                info!("No tests directory found");
                println!("\nNo tests directory found.");
                println!("Create a 'tests' directory with JSON test files to run tests.");
                println!("Each test file should contain 'input' and 'expected_output' fields.");
                println!("Example: {{ \"input\": {{ \"num1\": 5, \"num2\": 10 }}, \"expected_output\": {{ \"sum\": 15 }} }}");
                Ok(())
            }
            _ => {
                error!("Test execution failed: {:?}", err);
                Err(err).context("Test execution failed")
            }
        },
    }
}

/// Replay a task using recorded inputs from a previous session
async fn replay_task(task_path: &str, recording_dir: &PathBuf) -> Result<JsonValue> {
    info!("Replaying task from: {} with recording: {:?}", task_path, recording_dir);

    // Load the recorded input
    let input_file = recording_dir.join("input.json");
    if !input_file.exists() {
        return Err(anyhow::anyhow!("No input.json found in recording directory: {:?}", recording_dir));
    }

    let input_content = fs::read_to_string(&input_file)
        .context(format!("Failed to read input file: {:?}", input_file))?;
    let input_json: JsonValue = from_str(&input_content)
        .context("Failed to parse input JSON from recording")?;

    info!("Loaded recorded input from: {:?}", input_file);
    debug!("Input data: {}", to_string_pretty(&input_json)?);

    // Load the task from the filesystem
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;

    debug!("Task loaded: {} ({})", task.metadata.label, task.uuid());

    // Execute the task with the recorded input
    info!("Executing task with recorded input");
    let result = ratchet_lib::js_executor::execute_task(&mut task, input_json.clone())
        .await
        .context("Failed to execute task")?;

    info!("Task replay completed successfully");
    
    // Compare with recorded output if available
    let output_file = recording_dir.join("output.json");
    if output_file.exists() {
        let recorded_output_content = fs::read_to_string(&output_file)
            .context(format!("Failed to read output file: {:?}", output_file))?;
        let recorded_output: JsonValue = from_str(&recorded_output_content)
            .context("Failed to parse recorded output JSON")?;

        if result == recorded_output {
            println!("✓ Output matches recorded output");
            info!("Output matches recorded output");
        } else {
            println!("⚠ Output differs from recorded output");
            warn!("Output differs from recorded output");
            println!("\nRecorded output:");
            println!("{}", to_string_pretty(&recorded_output)?);
            println!("\nActual output:");
            println!("{}", to_string_pretty(&result)?);
        }
    } else {
        warn!("No recorded output found for comparison at: {:?}", output_file);
    }

    Ok(result)
}

/// Generate a new task template with stub files
fn generate_task(
    path: &PathBuf,
    label: Option<&String>,
    description: Option<&String>,
    version: Option<&String>,
) -> Result<()> {
    info!("Generating task template at: {:?}", path);

    // Check if directory already exists
    if path.exists() {
        return Err(anyhow::anyhow!("Directory already exists: {:?}", path));
    }

    // Create the task directory
    fs::create_dir_all(path)
        .context(format!("Failed to create task directory: {:?}", path))?;

    // Generate UUID for the task
    let task_uuid = Uuid::new_v4();

    // Use provided values or defaults
    let task_label = label.map(|s| s.as_str()).unwrap_or("My Task");
    let task_description = description.map(|s| s.as_str()).unwrap_or("A task that performs a specific operation");
    let task_version = version.map(|s| s.as_str()).unwrap_or("1.0.0");

    info!("Creating task files with UUID: {}", task_uuid);

    // Create metadata.json
    let metadata = json!({
        "uuid": task_uuid,
        "version": task_version,
        "label": task_label,
        "description": task_description
    });
    let metadata_path = path.join("metadata.json");
    let metadata_content = serde_json::to_string_pretty(&metadata)?;
    fs::write(&metadata_path, metadata_content)
        .context(format!("Failed to write metadata.json: {:?}", metadata_path))?;

    // Create input.schema.json
    let input_schema = json!({
        "type": "object",
        "properties": {
            "value": {
                "type": "string",
                "description": "Input value for the task"
            }
        },
        "required": ["value"]
    });
    let input_schema_path = path.join("input.schema.json");
    let input_schema_content = serde_json::to_string_pretty(&input_schema)?;
    fs::write(&input_schema_path, input_schema_content)
        .context(format!("Failed to write input.schema.json: {:?}", input_schema_path))?;

    // Create output.schema.json
    let output_schema = json!({
        "type": "object",
        "properties": {
            "result": {
                "type": "string",
                "description": "Result of the task operation"
            }
        },
        "required": ["result"]
    });
    let output_schema_path = path.join("output.schema.json");
    let output_schema_content = serde_json::to_string_pretty(&output_schema)?;
    fs::write(&output_schema_path, output_schema_content)
        .context(format!("Failed to write output.schema.json: {:?}", output_schema_path))?;

    // Create main.js
    let main_js_content = r#"// Task implementation
// This function receives input matching input.schema.json
// and must return output matching output.schema.json

function main(input) {
    // Validate input
    if (!input || typeof input.value !== 'string') {
        throw new Error('Invalid input: value must be a string');
    }
    
    // Process the input
    const processedValue = `Processed: ${input.value}`;
    
    // Return the result matching the output schema
    return {
        result: processedValue
    };
}

// Export the main function (required)
main;
"#;
    let main_js_path = path.join("main.js");
    fs::write(&main_js_path, main_js_content)
        .context(format!("Failed to write main.js: {:?}", main_js_path))?;

    // Create tests directory with a sample test
    let tests_dir = path.join("tests");
    fs::create_dir_all(&tests_dir)
        .context(format!("Failed to create tests directory: {:?}", tests_dir))?;

    // Create a sample test file
    let test_data = json!({
        "input": {
            "value": "test input"
        },
        "expected_output": {
            "result": "Processed: test input"
        }
    });
    let test_path = tests_dir.join("test-001.json");
    let test_content = serde_json::to_string_pretty(&test_data)?;
    fs::write(&test_path, test_content)
        .context(format!("Failed to write test file: {:?}", test_path))?;

    println!("✓ Task template created successfully!");
    println!("  Path: {:?}", path);
    println!("  UUID: {}", task_uuid);
    println!("  Label: {}", task_label);
    println!("  Version: {}", task_version);
    println!("  Description: {}", task_description);
    println!("\nFiles created:");
    println!("  - metadata.json        (task metadata)");
    println!("  - input.schema.json    (input validation schema)");
    println!("  - output.schema.json   (output validation schema)");
    println!("  - main.js              (task implementation)");
    println!("  - tests/test-001.json  (sample test case)");
    println!("\nNext steps:");
    println!("  1. Edit main.js to implement your task logic");
    println!("  2. Update input.schema.json and output.schema.json as needed");
    println!("  3. Add more test cases in the tests/ directory");
    println!("  4. Validate: ratchet validate --from-fs={}", path.display());
    println!("  5. Test: ratchet test --from-fs={}", path.display());

    info!("Task template generation completed successfully");
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing before doing anything else
    init_tracing(cli.log_level.as_ref(), cli.command.as_ref().and_then(|cmd| {
        match cmd {
            Commands::RunOnce { record, .. } => record.as_ref(),
            _ => None,
        }
    }))?;

    info!("Ratchet CLI starting");

    // Create a tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    match &cli.command {
        Some(Commands::RunOnce {
            from_fs,
            input_json,
            record,
        }) => {
            info!("Running task from file system path: {}", from_fs);

            // Parse input JSON
            let input = parse_input_json(input_json.as_ref())?;

            if input_json.is_some() {
                info!("Using provided input: {}", input_json.as_ref().unwrap());
            }

            // Run the task
            let result = runtime.block_on(run_task(from_fs, &input))?;

            // Pretty-print the result
            let formatted = to_string_pretty(&result).context("Failed to format result as JSON")?;

            println!("Result: {}", formatted);
            info!("Task execution completed");
            
            // Finalize recording if it was enabled
            if record.is_some() {
                if let Err(e) = ratchet_lib::recording::finalize_recording() {
                    warn!("Failed to finalize recording: {}", e);
                } else {
                    if let Some(dir) = ratchet_lib::recording::get_recording_dir() {
                        println!("Recording saved to: {:?}", dir);
                    }
                }
            }
            
            Ok(())
        }
        Some(Commands::Validate { from_fs }) => validate_task(from_fs),
        Some(Commands::Test { from_fs }) => runtime.block_on(test_task(from_fs)),
        Some(Commands::Replay { from_fs, recording }) => {
            info!("Replaying task from file system path: {} with recording: {:?}", from_fs, recording);

            // Run the replay
            let result = runtime.block_on(replay_task(from_fs, recording))?;

            // Pretty-print the result
            let formatted = to_string_pretty(&result).context("Failed to format result as JSON")?;

            println!("Replay Result: {}", formatted);
            info!("Task replay completed");
            
            Ok(())
        }
        Some(Commands::Generate { generate_cmd }) => {
            match generate_cmd {
                GenerateCommands::Task { path, label, description, version } => {
                    info!("Generating task template at: {:?}", path);
                    generate_task(path, label.as_ref(), description.as_ref(), version.as_ref())
                }
            }
        }
        None => {
            warn!("No command specified");
            println!("No command specified. Use --help to see available commands.");
            Ok(())
        }
    }
}

