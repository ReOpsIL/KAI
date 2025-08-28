//! KAI - OpenRouter API client with comprehensive file system tools
//!
//! This library provides a complete set of file system tools designed for use with
//! OpenRouter's LLM chat completion API, including tool calling functionality.
//!
//! # Features
//!
//! - **File Operations**: Read, write, create, and delete files with wildcard support
//! - **Directory Operations**: List, create, and manage directories
//! - **Search Operations**: Grep-like text search with regex support
//! - **Text Processing**: Search and replace across multiple files
//! - **File Discovery**: Find files by patterns and types
//! - **OpenRouter Integration**: Compatible with OpenRouter's tool-calling API
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use kai::tools::{get_file_system_tools, FileSystemOperations};
//! use kai::openrouter::OpenRouterClient;
//!
//! // Get tool definitions for OpenRouter
//! let tools = get_file_system_tools();
//!
//! // Use tools directly
//! let result = FileSystemOperations::read_file("example.txt");
//! if result.success {
//!     println!("File read successfully!");
//! }
//!
//! // Initialize OpenRouter client
//! let client = OpenRouterClient::new("your-api-key".to_string());
//! ```
//!
//! # Available Tools
//!
//! 1. **read_file** - Read complete file contents
//! 2. **write_file** - Write or append content to files
//! 3. **list_directory** - List files/directories with optional patterns
//! 4. **create_path** - Create files or directories
//! 5. **delete_path** - Delete files/directories with wildcard support
//! 6. **grep_files** - Search text in files using regular expressions
//! 7. **search_replace** - Find and replace text across multiple files
//! 8. **find_files** - Find files by name patterns and types
//!
//! All tools support wildcard patterns (*, **, ?) and provide comprehensive error handling.

pub mod openrouter;
pub mod tools;
pub mod session;
pub mod context;

// Re-export commonly used types for convenience
pub use tools::{FileSystemTool, FileSystemOperations, ToolResult, get_file_system_tools};
pub use openrouter::{OpenRouterClient, ChatRequest, ChatResponse, Message};