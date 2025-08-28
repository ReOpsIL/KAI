use kai::context::{Harvester, HarvesterConfig, ContextDataStore};
use kai::openrouter::OpenRouterClient;
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌾 Harvester Demo");
    println!("================");
    
    // Get API key from environment (optional for this demo)
    let api_key = env::var("OPENROUTER_API_KEY").ok();
    
    // Create harvester with default configuration
    let config = HarvesterConfig {
        root_path: std::env::current_dir()?,
        max_file_size_mb: 1, // Smaller limit for demo
        ..Default::default()
    };
    
    let mut harvester = Harvester::new(config);
    
    // Add OpenRouter client if API key is available
    if let Some(key) = api_key {
        println!("🔑 OpenRouter API key found - will generate descriptions");
        let client = OpenRouterClient::new(key);
        harvester = harvester.with_openrouter(client);
    } else {
        println!("⚠️  No OpenRouter API key found (OPENROUTER_API_KEY) - will skip description generation");
    }
    
    // Add some custom exclusions for demo
    harvester = harvester.add_exclude_patterns(vec![
        "examples".to_string(), // Exclude examples directory to avoid recursion
    ]);
    
    println!("\n🔍 Starting file discovery and harvesting...");
    
    // Run the harvesting process
    let modules = harvester.harvest().await?;
    
    println!("\n📊 Harvesting Results:");
    println!("- Found {} modules", modules.len());
    
    let total_files: usize = modules.iter().map(|m| m.files.len()).sum();
    println!("- Found {} total files", total_files);
    
    // Display module summary
    for module in &modules {
        println!("\n📁 Module: {}", module.name);
        println!("   Path: {}", module.path.display());
        println!("   Files: {}", module.files.len());
        
        if let Some(description) = &module.description {
            let first_line = description.lines().next().unwrap_or("");
            println!("   Description: {}", first_line);
        }
        
        // Show first few files
        for (i, file) in module.files.iter().take(3).enumerate() {
            println!("   📄 {}: {}", i + 1, file.relative_path.display());
            if let Some(description) = &file.description {
                let first_line = description.lines().next().unwrap_or("");
                println!("      → {}", first_line);
            }
        }
        
        if module.files.len() > 3 {
            println!("   ... and {} more files", module.files.len() - 3);
        }
    }
    
    // Create context data store and save results
    println!("\n💾 Saving results to .context directory...");
    let context_store = ContextDataStore::with_current_dir()?;
    
    // Clear existing context first
    if context_store.context_dir_exists() {
        println!("🧹 Clearing existing context directory");
        context_store.clear_context()?;
    }
    
    // Save all harvester results
    context_store.save_harvester_results(&modules)?;
    
    println!("✅ Results saved to: {}", context_store.context_dir_path().display());
    
    // List generated files
    if let Ok(entries) = std::fs::read_dir(context_store.context_dir_path()) {
        println!("\n📝 Generated markdown files:");
        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
            .collect();
        files.sort();
        
        for file in files {
            println!("   - {}", file);
        }
    }
    
    println!("\n🎉 Demo completed successfully!");
    println!("💡 Check the .context directory for generated markdown files");
    
    Ok(())
}