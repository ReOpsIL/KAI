//! KAI - Main Entry Point
//!
//! This is the main entry point for the KAI application, which starts the
//! enhanced CLI prompter for interactive usage with AI planning capabilities.

use std::env;
use std::io::{self, Write};
use std::process;
use std::sync::Arc;
use KAI::cli::CliPrompter;
use KAI::llm::OpenRouterClient;
use KAI::planer::Planner;

#[tokio::main]
async fn main() {
    // Initialize OpenRouter client from environment variable
    let openrouter_client = match initialize_openrouter_client() {
        Ok(client) => {
            println!("OpenRouter client initialized successfully");
            Some(client)
        }
        Err(e) => {
            eprintln!("ERROR: {}", e);
            eprintln!("\nTo enable AI planning features:");
            eprintln!("   1. Get an API key from https://openrouter.ai");
            eprintln!("   2. Set environment variable: export OPENROUTER_API_KEY=your_key");
            eprintln!("   3. Restart the application");
            eprintln!(
                "\nExiting application - OpenRouter API key required for 🦀 KAI functionality"
            );
            process::exit(1);
        }
    };

    // Initialize and run the application
    match run_kai_application(openrouter_client).await {
        Ok(_) => {
            println!("\nThanks for using 🦀 KAI! Goodbye!");
        }
        Err(e) => {
            eprintln!("\nERROR: 🦀 KAI encountered an error: {}", e);
            eprintln!("Please check your terminal compatibility and try again.");
            process::exit(1);
        }
    }
}

/// Initialize OpenRouter client from environment variable
fn initialize_openrouter_client() -> Result<Arc<OpenRouterClient>, String> {
    let api_key = env::var("OPENROUTER_API_KEY").map_err(|_| {
        "OpenRouter API key not found in environment variable OPENROUTER_API_KEY".to_string()
    })?;

    if api_key.is_empty() {
        return Err("OpenRouter API key is empty".to_string());
    }

    if api_key.len() < 10 {
        return Err("OpenRouter API key appears to be invalid (too short)".to_string());
    }

    let client = OpenRouterClient::new(api_key);
    Ok(Arc::new(client))
}

fn print_banner() {
    println!("╭─────────────────────────────────────────────────╮");
    println!("│  KAI - Enhanced AI-Powered CLI Assistant        │");
    println!("│  Advanced terminal interface with AI planning   │");
    println!("╰─────────────────────────────────────────────────╯");
    println!();
    println!("AI Planning: Enabled");
    println!("Starting enhanced CLI interface...");
    println!("Tip: Type '/' for commands or '@' for file browser");
    println!();

    // Small delay to let user read the banner
    std::thread::sleep(std::time::Duration::from_millis(1000));
}

async fn run_kai_application(openrouter_client: Option<Arc<OpenRouterClient>>) -> io::Result<()> {
    // Initialize the planner with LLM client
    let mut prompter = if let Some(client) = openrouter_client {
        println!("AI Planning system initialized with OpenRouter");
        let planner = Planner::with_llm_client(client);

        // Create prompter with planner
        match CliPrompter::with_planner(planner) {
            Ok(mut p) => {
                println!("CLI prompter initialized successfully with AI planning");

                // Initialize context
                println!("Initializing project context...");
                if let Err(e) = p.initialize_context().await {
                    eprintln!("WARNING: Context initialization failed: {}", e);
                    eprintln!("Continuing without context integration...");
                }

                p
            }
            Err(e) => {
                eprintln!(
                    "ERROR: Failed to initialize CLI prompter with planner: {}",
                    e
                );
                eprintln!("\nPossible issues:");
                eprintln!("  • Terminal not supported (try a different terminal)");
                eprintln!("  • Terminal size too small (resize your terminal)");
                eprintln!("  • Permission issues (check terminal permissions)");
                return Err(e);
            }
        }
    } else {
        eprintln!("🦀 KAI requires AI planning to function - no basic mode available");
        eprintln!("OpenRouter client initialization failed - exiting");
        process::exit(1);
    };

    // Show banner after successful initialization
    //print_banner();

    // Clear screen before starting
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap_or(());

    // Run the interactive CLI
    prompter.run().await
}
