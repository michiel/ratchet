use clap::{Parser, Subcommand};
use ratchet_lib::js_task;
use serde_json::to_string_pretty;

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
    },
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::RunOnce { from_fs }) => {
            println!("Running task from file system path: {}", from_fs);
            match js_task::run_task_from_fs(from_fs) {
                Ok(result) => {
                    let formatted = to_string_pretty(&result).unwrap();
                    println!("Result: {}", formatted);
                }
                Err(err) => {
                    eprintln!("Error running task: {}", err);
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("No command specified. Use --help to see available commands.");
        }
    }
}

