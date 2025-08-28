use kai::session::{SessionManager, Session};
use std::path::PathBuf;

fn main() {
    println!("=== Session Manager Demo ===\n");
    
    // Create a session manager with a temporary storage path
    let storage_path = PathBuf::from("demo_sessions.json");
    let mut manager = SessionManager::new(&storage_path);
    
    // Demo 1: Create sessions
    println!("1. Creating sessions...");
    let result1 = manager.create_session("Development Session");
    println!("   {}", result1.message);
    if let Some(id) = &result1.data {
        println!("   Generated ID: {}", id);
    }
    
    let result2 = manager.create_session("Testing Session");
    println!("   {}", result2.message);
    
    let result3 = manager.create_session("Production Session");
    println!("   {}", result3.message);
    
    println!();
    
    // Demo 2: List all sessions
    println!("2. Listing all sessions (ordered by date):");
    let result = manager.list_sessions(None);
    println!("   {}", result.message);
    if let Some(data) = &result.data {
        for line in data.lines() {
            println!("   {}", line);
        }
    }
    
    println!();
    
    // Demo 3: Filter sessions by name
    println!("3. Filtering sessions by name 'Dev':");
    let result = manager.list_sessions(Some("Dev"));
    println!("   {}", result.message);
    if let Some(data) = &result.data {
        for line in data.lines() {
            println!("   {}", line);
        }
    }
    
    println!();
    
    // Demo 4: Select a session
    println!("4. Selecting 'Development Session':");
    let result = manager.select_session("Development Session");
    println!("   {}", result.message);
    
    // Show active session
    if let Some(active) = manager.get_active_session() {
        println!("   Active session: {} (ID: {})", active.name, active.id);
    }
    
    println!();
    
    // Demo 5: Add some mock data and clean it
    println!("5. Adding mock data to active session and then cleaning...");
    if let Some(active_id) = &manager.get_active_session().map(|s| s.id.clone()) {
        // We'll simulate adding data by creating a new session manager 
        // and directly manipulating the session (in real usage, you'd have methods to add data)
        let result = manager.clean_session_data(active_id);
        println!("   {}", result.message);
    }
    
    println!();
    
    // Demo 6: Delete a session
    println!("6. Deleting 'Testing Session':");
    let result = manager.delete_session("Testing Session");
    println!("   {}", result.message);
    
    println!();
    
    // Demo 7: List remaining sessions
    println!("7. Listing remaining sessions:");
    let result = manager.list_sessions(None);
    println!("   {}", result.message);
    if let Some(data) = &result.data {
        for line in data.lines() {
            println!("   {}", line);
        }
    }
    
    println!();
    
    // Demo 8: Error handling - try to select non-existent session
    println!("8. Testing error handling - selecting non-existent session:");
    let result = manager.select_session("Non-existent Session");
    println!("   {}", result.message);
    println!("   Success: {}", result.success);
    
    println!();
    
    // Demo 9: Show 4-digit ID generation
    println!("9. Demonstrating 4-digit ID generation:");
    for i in 1..=5 {
        let result = manager.create_session(&format!("Test Session {}", i));
        if let Some(id) = &result.data {
            println!("   Session {}: ID = {} (length: {})", i, id, id.len());
        }
    }
    
    println!("\n=== Demo Complete ===");
    
    // Clean up demo file
    if storage_path.exists() {
        std::fs::remove_file(&storage_path).ok();
        println!("Demo storage file cleaned up.");
    }
}