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
    /// Run the sample addition JS task
    RunAddition {
        /// Minimum value for random numbers
        #[arg(long, default_value_t = 1)]
        min: i32,

        /// Maximum value for random numbers
        #[arg(long, default_value_t = 100)]
        max: i32,
    },
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::RunAddition { min, max }) => {
            println!(
                "Running addition task with random numbers between {} and {}",
                min, max
            );
            match js_task::run_addition_task(*min, *max) {
                Ok(result) => {
                    let formatted = to_string_pretty(&result).unwrap();
                    println!("Result: {}", formatted);
                }
                Err(err) => {
                    eprintln!("Error running addition task: {}", err);
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("No command specified. Use --help to see available commands.");
        }
    }
}

