use kai::session::session_manager::{SessionManager, Session};
use kai::context::{Context, ContextDataStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Context object functionality...");
    
    // Create a test session with context
    let root_path = std::env::current_dir()?;
    let mut session = Session::new_with_root("Test Session".to_string(), root_path.clone());
    
    println!("Created session '{}' with ID: {}", session.name, session.id);
    println!("Context initialized: {}", session.context.initialized);
    println!("Tracked files count: {}", session.context.tracked_files_count());
    
    // Create a context data store
    let data_store = ContextDataStore::new(root_path.clone());
    
    // Test force refresh (full update) without OpenRouter client
    println!("\n--- Testing force refresh (without LLM) ---");
    match session.context.update(&data_store, None, true).await {
        Ok(()) => {
            println!("✓ Context update completed successfully");
            println!("Context initialized: {}", session.context.initialized);
            println!("Tracked files count: {}", session.context.tracked_files_count());
            println!("Last updated: {}", session.context.last_updated);
        }
        Err(e) => {
            println!("✗ Context update failed: {}", e);
        }
    }
    
    // Test incremental update (should detect no changes)
    println!("\n--- Testing incremental update (no changes expected) ---");
    match session.context.update(&data_store, None, false).await {
        Ok(()) => {
            println!("✓ Incremental update completed");
        }
        Err(e) => {
            println!("✗ Incremental update failed: {}", e);
        }
    }
    
    // Test context needs refresh detection
    println!("\n--- Testing context refresh detection ---");
    println!("Needs refresh (force=false): {}", session.context.needs_refresh(false));
    println!("Needs refresh (force=true): {}", session.context.needs_refresh(true));
    
    // Test with SessionManager
    println!("\n--- Testing with SessionManager ---");
    let storage_path = root_path.join(".sessions.json");
    let mut session_manager = SessionManager::new(&storage_path);
    
    let result = session_manager.create_session("Context Test Session");
    if result.success {
        println!("✓ Created session through SessionManager");
        
        if let Some(active_session) = session_manager.get_active_session() {
            println!("Active session has context initialized: {}", active_session.context.initialized);
        }
    } else {
        println!("✗ Failed to create session: {}", result.message);
    }
    
    // Check if context directory was created
    if data_store.context_dir_exists() {
        println!("✓ Context directory was created at: {}", data_store.context_dir_path().display());
    } else {
        println!("✗ Context directory was not created");
    }
    
    println!("\nContext functionality test completed!");
    Ok(())
}