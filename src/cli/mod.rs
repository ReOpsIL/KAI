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
//! - `cli_term` - Legacy module (deprecated, use `prompter`)

pub mod config;
pub mod editor;
pub mod history;
pub mod commands;
pub mod file_browser;
pub mod utils;
pub mod prompter;

// Legacy module for backwards compatibility
#[deprecated(since = "0.1.1", note = "Use `prompter` module instead")]
pub mod cli_term;

// Re-export main types for convenience
pub use prompter::CliPrompter;
pub use config::CliConfig;
pub use history::CommandHistory;
pub use editor::{TextEditor, CursorDirection};
pub use commands::{CliCommand, CommandParser, CommandResult};
pub use file_browser::{FileBrowser, FileEntry, SelectionResult};

// Re-export legacy types for backwards compatibility
#[deprecated(since = "0.1.1", note = "Use types from specific modules instead")]
pub use cli_term::{CliPrompter as LegacyCliPrompter, CliConfig as LegacyCliConfig, CommandHistory as LegacyCommandHistory};