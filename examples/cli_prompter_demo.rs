//! CLI Prompter Demo
//!
//! This example demonstrates how to use the enhanced CLI prompter
//! with interactive features including command menus, file browsing,
//! and advanced text editing capabilities.

use kai::cli::CliPrompter;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("Starting KAI Enhanced CLI Prompter Demo");
    println!("========================================");
    println!();
    
    // Initialize the CLI prompter
    let mut prompter = match CliPrompter::new() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to initialize CLI prompter: {}", e);
            return Err(e);
        }
    };
    
    // Run the interactive CLI
    match prompter.run().await {
        Ok(_) => println!("CLI prompter exited successfully."),
        Err(e) => {
            eprintln!("CLI prompter error: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}