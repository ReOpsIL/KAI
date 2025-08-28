//! KAI - Main Entry Point
//!
//! This is the main entry point for the KAI application, which starts the
//! enhanced CLI prompter for interactive usage.

use kai::cli::CliPrompter;
use std::io::{self, Write};
use std::process;

#[tokio::main]
async fn main() {
    // Print welcome banner
    print_banner();
    
    // Initialize and run the CLI prompter
    match run_cli_prompter().await {
        Ok(_) => {
            println!("\n👋 Thanks for using KAI! Goodbye!");
        }
        Err(e) => {
            eprintln!("\n❌ KAI encountered an error: {}", e);
            eprintln!("Please check your terminal compatibility and try again.");
            process::exit(1);
        }
    }
}

fn print_banner() {
    println!("╭─────────────────────────────────────────────────╮");
    println!("│  🤖 KAI - Enhanced CLI Prompter               │");
    println!("│  Advanced terminal interface for AI prompting  │");
    println!("╰─────────────────────────────────────────────────╯");
    println!();
    println!("🚀 Starting enhanced CLI interface...");
    println!("💡 Tip: Type '/' for commands or '@' for file browser");
    println!();
    
    // Small delay to let user read the banner
    std::thread::sleep(std::time::Duration::from_millis(1000));
}

async fn run_cli_prompter() -> io::Result<()> {
    // Initialize the CLI prompter
    let mut prompter = match CliPrompter::new() {
        Ok(p) => {
            println!("✅ CLI prompter initialized successfully");
            p
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize CLI prompter: {}", e);
            eprintln!("\nPossible issues:");
            eprintln!("  • Terminal not supported (try a different terminal)");
            eprintln!("  • Terminal size too small (resize your terminal)");
            eprintln!("  • Permission issues (check terminal permissions)");
            return Err(e);
        }
    };
    
    // Clear screen before starting
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap_or(());
    
    // Run the interactive CLI
    prompter.run().await
}
