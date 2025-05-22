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
        /// First number to add
        #[arg(long, default_value_t = 5)]
        num1: i32,

        /// Second number to add
        #[arg(long, default_value_t = 7)]
        num2: i32,
    },
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::RunAddition { num1, num2 }) => {
            println!(
                "Running addition task with inputs: {} and {}",
                num1, num2
            );
            match js_task::run_addition_task(*num1, *num2) {
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

