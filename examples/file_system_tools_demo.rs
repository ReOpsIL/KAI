/// Demonstration of file system tools for OpenRouter LLM integration
/// This example shows how to use the comprehensive file system tools
/// with OpenRouter's chat completion API including tool calling.

use kai::tools::{get_file_system_tools, FileSystemOperations, ToolResult};
use kai::openrouter::{OpenRouterClient, Message};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OpenRouter client (API key would come from environment)
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .unwrap_or_else(|_| "your-api-key-here".to_string());
    let client = OpenRouterClient::new(api_key);

    // Get file system tools definitions for OpenRouter
    let file_system_tools = get_file_system_tools();
    
    // Convert tools to JSON format expected by OpenRouter API
    let tools_json: Vec<serde_json::Value> = file_system_tools
        .iter()
        .map(|tool| serde_json::to_value(tool).unwrap())
        .collect();

    println!("🔧 Available File System Tools:");
    for tool in &file_system_tools {
        println!("  - {}: {}", tool.function.name, tool.function.description);
    }
    println!();

    // Example: Demonstrate direct tool usage
    println!("📁 Direct Tool Usage Examples:");
    
    // Example 1: Create a test file
    println!("\n1. Creating a test file...");
    let write_result = FileSystemOperations::write_file(
        "example_test.txt",
        "Hello from KAI file system tools!\nThis is a test file.\nIt contains multiple lines for demonstration.",
        None
    );
    print_result("Write File", &write_result);

    // Example 2: Read the file back
    println!("\n2. Reading the file back...");
    let read_result = FileSystemOperations::read_file("example_test.txt");
    print_result("Read File", &read_result);

    // Example 3: List current directory
    println!("\n3. Listing current directory (*.txt files)...");
    let list_result = FileSystemOperations::list_directory(".", Some("*.txt"), None);
    print_result("List Directory", &list_result);

    // Example 4: Search for text in files
    println!("\n4. Searching for 'test' in .txt files...");
    let grep_result = FileSystemOperations::grep_files(
        "test",
        "*.txt", 
        None,
        Some(true),
        Some(1)
    );
    print_result("Grep Files", &grep_result);

    // Example 5: Find files by pattern
    println!("\n5. Finding all .txt files...");
    let find_result = FileSystemOperations::find_files(
        "*.txt",
        Some("."),
        Some("file")
    );
    print_result("Find Files", &find_result);

    // Example OpenRouter integration structure
    println!("\n🤖 OpenRouter Integration Example:");
    println!("To use these tools with OpenRouter, send a request like:");
    
    let example_request = json!({
        "model": "anthropic/claude-3.5-sonnet",
        "messages": [
            {
                "role": "user",
                "content": "Please read the contents of example_test.txt and search for the word 'demonstration' in all .txt files"
            }
        ],
        "tools": tools_json,
        "tool_choice": "auto"
    });
    
    println!("{}", serde_json::to_string_pretty(&example_request)?);

    // Clean up
    println!("\n🧹 Cleaning up test file...");
    let delete_result = FileSystemOperations::delete_path("example_test.txt", None);
    print_result("Delete File", &delete_result);

    println!("\n✅ File system tools demonstration completed!");
    println!("\n📋 Summary of Available Tools:");
    println!("1. read_file - Read complete file contents");
    println!("2. write_file - Write/append content to files");
    println!("3. list_directory - List files/directories with patterns");
    println!("4. create_path - Create files or directories");
    println!("5. delete_path - Delete files/directories with wildcards");
    println!("6. grep_files - Search text in files with regex");
    println!("7. search_replace - Find and replace text in files");
    println!("8. find_files - Find files by name patterns");
    println!("\nAll tools support wildcard patterns and provide detailed error handling!");

    Ok(())
}

fn print_result(operation: &str, result: &ToolResult) {
    if result.success {
        println!("  ✅ {}: Success", operation);
        if let Some(data) = &result.data {
            // Print a condensed version of the data for readability
            match operation {
                "Read File" => {
                    if let Some(content) = data.get("content") {
                        let content_str = content.as_str().unwrap_or("");
                        let preview = if content_str.len() > 100 {
                            format!("{}...", &content_str[..100])
                        } else {
                            content_str.to_string()
                        };
                        println!("     Content preview: {}", preview);
                        println!("     Size: {} bytes", data.get("size").unwrap_or(&json!(0)));
                    }
                }
                "List Directory" => {
                    if let Some(files) = data.get("files") {
                        let file_count = files.as_array().map(|arr| arr.len()).unwrap_or(0);
                        println!("     Found {} files", file_count);
                    }
                    if let Some(dirs) = data.get("directories") {
                        let dir_count = dirs.as_array().map(|arr| arr.len()).unwrap_or(0);
                        println!("     Found {} directories", dir_count);
                    }
                }
                "Grep Files" => {
                    if let Some(matches) = data.get("total_matches") {
                        println!("     Total matches: {}", matches);
                    }
                    if let Some(files) = data.get("files_with_matches") {
                        println!("     Files with matches: {}", files);
                    }
                }
                "Find Files" => {
                    if let Some(count) = data.get("count") {
                        println!("     Found {} items", count);
                    }
                }
                _ => {
                    println!("     Data: {}", serde_json::to_string(data).unwrap_or_default());
                }
            }
        }
    } else {
        println!("  ❌ {}: Failed", operation);
        if let Some(error) = &result.error {
            println!("     Error: {}", error);
        }
    }
}