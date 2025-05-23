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
        }
        None => {
            println!("No command specified. Use --help to see available commands.");
            Ok(())
        }
    }
}

