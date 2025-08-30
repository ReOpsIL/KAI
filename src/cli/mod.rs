//! CLI Terminal Interface Module
//!
//! This module provides a sophisticated terminal-based prompt CLI application
//! with interactive text input, special command handling, file system navigation,
//! and advanced text editing capabilities.
//!
//! ## Module Structure
//!
//! - `config` - Configuration and theme management
//! - `editor` - Multi-line text editor with cursor management
//! - `history` - Command history with search capabilities
//! - `commands` - Command definitions and parsing
//! - `file_browser` - Interactive file system navigation
//! - `utils` - Common utilities and helper functions
//! - `prompter` - Main CLI prompter orchestration

pub mod commands;
pub mod config;
pub mod editor;
pub mod file_browser;
pub mod history;
pub mod prompter;
pub mod utils;

// Re-export main types for convenience
// pub use prompter::CliPrompter;
pub use commands::{CliCommand, CommandParser, CommandResult};
pub use config::CliConfig;
pub use editor::{CursorDirection, TextEditor};
pub use file_browser::{FileBrowser, FileEntry, SelectionResult};
pub use history::CommandHistory;
pub use prompter::SimpleCliPrompter;
