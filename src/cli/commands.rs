//! CLI Commands Module
//!
//! Command definitions, parsing, and execution logic for the CLI prompter.
//! Provides a comprehensive set of built-in commands and extensible command system.

use std::fmt;

/// Available CLI commands
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Help,
    History,
    Clear,
    Config,
    Templates,
    Export,
    Quit,
    Save,
    Load,
    Theme,
    KeyBinds,
}

impl CliCommand {
    /// Parse a command string into a CliCommand
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "help" | "h" | "?" => Some(Self::Help),
            "history" | "hist" => Some(Self::History),
            "clear" | "cls" => Some(Self::Clear),
            "config" | "settings" => Some(Self::Config),
            "templates" | "template" | "tmpl" => Some(Self::Templates),
            "export" | "exp" => Some(Self::Export),
            "quit" | "exit" | "q" => Some(Self::Quit),
            "save" => Some(Self::Save),
            "load" => Some(Self::Load),
            "theme" | "themes" => Some(Self::Theme),
            "keybinds" | "keys" | "bindings" => Some(Self::KeyBinds),
            _ => None,
        }
    }
    
    /// Get command description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Help => "Show available commands and help information",
            Self::History => "View and search command history",
            Self::Clear => "Clear the terminal screen",
            Self::Config => "Open configuration settings",
            Self::Templates => "Load and manage prompt templates",
            Self::Export => "Export current session to file",
            Self::Quit => "Exit the application",
            Self::Save => "Save current session",
            Self::Load => "Load saved session",
            Self::Theme => "Change color theme",
            Self::KeyBinds => "View and edit key bindings",
        }
    }
    
    /// Get command usage/syntax
    pub fn usage(&self) -> &'static str {
        match self {
            Self::Help => "/help [command]",
            Self::History => "/history [search_term]",
            Self::Clear => "/clear",
            Self::Config => "/config [show|edit]",
            Self::Templates => "/templates [list|load|save]",
            Self::Export => "/export [format] [filename]",
            Self::Quit => "/quit",
            Self::Save => "/save [session_name]",
            Self::Load => "/load [session_name]",
            Self::Theme => "/theme [theme_name]",
            Self::KeyBinds => "/keybinds [show|edit]",
        }
    }
    
    /// Get command category for grouping
    pub fn category(&self) -> CommandCategory {
        match self {
            Self::Help => CommandCategory::Help,
            Self::History => CommandCategory::Navigation,
            Self::Clear => CommandCategory::Display,
            Self::Config | Self::KeyBinds => CommandCategory::Settings,
            Self::Templates | Self::Export | Self::Save | Self::Load => CommandCategory::Session,
            Self::Quit => CommandCategory::Control,
            Self::Theme => CommandCategory::Display,
        }
    }
    
    /// Check if command requires confirmation
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, Self::Quit | Self::Clear)
    }
    
    /// Get all available commands
    pub fn all_commands() -> Vec<Self> {
        vec![
            Self::Help,
            Self::History,
            Self::Clear,
            Self::Config,
            Self::Templates,
            Self::Export,
            Self::Save,
            Self::Load,
            Self::Theme,
            Self::KeyBinds,
            Self::Quit,
        ]
    }
    
    /// Get commands by category
    pub fn by_category(category: CommandCategory) -> Vec<Self> {
        Self::all_commands()
            .into_iter()
            .filter(|cmd| cmd.category() == category)
            .collect()
    }
    
    /// Get formatted command list for display
    pub fn get_command_menu() -> Vec<String> {
        let mut menu = Vec::new();
        
        // Group commands by category
        let categories = [
            CommandCategory::Help,
            CommandCategory::Navigation,
            CommandCategory::Display,
            CommandCategory::Settings,
            CommandCategory::Session,
            CommandCategory::Control,
        ];
        
        for category in &categories {
            let commands = Self::by_category(*category);
            if !commands.is_empty() {
                menu.push(format!("── {} ──", category));
                for cmd in commands {
                    menu.push(format!("{} - {}", cmd.usage(), cmd.description()));
                }
                menu.push(String::new()); // Empty line between categories
            }
        }
        
        // Remove last empty line
        if menu.last() == Some(&String::new()) {
            menu.pop();
        }
        
        menu
    }
    
    /// Get help text for a specific command
    pub fn get_help_text(&self) -> Vec<String> {
        let mut help = vec![
            format!("🎯 KAI CLI - {} Command", self),
            "".to_string(),
            format!("Description: {}", self.description()),
            format!("Usage: {}", self.usage()),
            format!("Category: {}", self.category()),
        ];
        
        // Add specific help content based on command
        match self {
            Self::Help => {
                help.extend(vec![
                    "".to_string(),
                    "Examples:".to_string(),
                    "  /help          - Show all commands".to_string(),
                    "  /help theme    - Show help for theme command".to_string(),
                ]);
            }
            Self::History => {
                help.extend(vec![
                    "".to_string(),
                    "Navigate through your command history:".to_string(),
                    "  • Use ↑/↓ arrows in normal mode".to_string(),
                    "  • Use Ctrl+R for interactive search".to_string(),
                    "  • Commands are automatically saved".to_string(),
                ]);
            }
            Self::Theme => {
                help.extend(vec![
                    "".to_string(),
                    "Available themes:".to_string(),
                    "  • default - Blue frame, white text".to_string(),
                    "  • dark - Black frame, green text".to_string(),
                    "  • light - Gray frame, black text".to_string(),
                    "  • ocean - Cyan frame, white text".to_string(),
                    "  • sunset - Magenta frame, yellow text".to_string(),
                ]);
            }
            Self::Config => {
                help.extend(vec![
                    "".to_string(),
                    "Configuration options:".to_string(),
                    "  • Frame and text colors".to_string(),
                    "  • Command and file browser prefixes".to_string(),
                    "  • History settings".to_string(),
                    "  • Custom key bindings".to_string(),
                ]);
            }
            _ => {}
        }
        
        help
    }
}

impl fmt::Display for CliCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Help => "Help",
            Self::History => "History",
            Self::Clear => "Clear",
            Self::Config => "Config",
            Self::Templates => "Templates",
            Self::Export => "Export",
            Self::Quit => "Quit",
            Self::Save => "Save",
            Self::Load => "Load",
            Self::Theme => "Theme",
            Self::KeyBinds => "KeyBinds",
        };
        write!(f, "{}", name)
    }
}

/// Command categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    Help,
    Navigation,
    Display,
    Settings,
    Session,
    Control,
}

impl fmt::Display for CommandCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Help => "Help & Information",
            Self::Navigation => "Navigation & Search", 
            Self::Display => "Display & UI",
            Self::Settings => "Settings & Configuration",
            Self::Session => "Session Management",
            Self::Control => "Application Control",
        };
        write!(f, "{}", name)
    }
}

/// Command execution result
#[derive(Debug, Clone)]
pub enum CommandResult {
    Success(String),
    Info(String),
    Warning(String),
    Error(String),
    Exit,
    NoOp,
}

impl CommandResult {
    /// Check if the result indicates success
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_) | Self::Info(_) | Self::NoOp)
    }
    
    /// Check if the result indicates an error
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_) | Self::Warning(_))
    }
    
    /// Get the message content if any
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Success(msg) | Self::Info(msg) | Self::Warning(msg) | Self::Error(msg) => Some(msg),
            Self::Exit | Self::NoOp => None,
        }
    }
}

/// Command parser and utilities
pub struct CommandParser;

impl CommandParser {
    /// Parse a full command line into command and arguments
    pub fn parse_command_line(input: &str) -> Option<(CliCommand, Vec<String>)> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        
        let without_prefix = &trimmed[1..];
        let parts: Vec<&str> = without_prefix.split_whitespace().collect();
        
        if parts.is_empty() {
            return None;
        }
        
        let command = CliCommand::from_str(parts[0])?;
        let args = parts[1..].iter().map(|s| s.to_string()).collect();
        
        Some((command, args))
    }
    
    /// Get command suggestions for partial input
    pub fn get_suggestions(partial: &str) -> Vec<String> {
        let partial = partial.to_lowercase();
        let mut suggestions = Vec::new();
        
        for cmd in CliCommand::all_commands() {
            let cmd_str = format!("{:?}", cmd).to_lowercase();
            if cmd_str.starts_with(&partial) {
                suggestions.push(format!("/{} - {}", cmd_str, cmd.description()));
            }
        }
        
        suggestions.sort();
        suggestions
    }
    
    /// Validate command arguments
    pub fn validate_args(command: &CliCommand, args: &[String]) -> Result<(), String> {
        match command {
            CliCommand::Theme => {
                if args.len() > 1 {
                    return Err("Theme command accepts at most one argument".to_string());
                }
            }
            CliCommand::Help => {
                if args.len() > 1 {
                    return Err("Help command accepts at most one argument".to_string());
                }
            }
            _ => {} // Most commands are flexible with arguments
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_parsing() {
        assert_eq!(CliCommand::from_str("help"), Some(CliCommand::Help));
        assert_eq!(CliCommand::from_str("QUIT"), Some(CliCommand::Quit));
        assert_eq!(CliCommand::from_str("h"), Some(CliCommand::Help));
        assert_eq!(CliCommand::from_str("invalid"), None);
    }
    
    #[test]
    fn test_command_line_parsing() {
        let result = CommandParser::parse_command_line("/help theme");
        assert!(result.is_some());
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, CliCommand::Help);
        assert_eq!(args, vec!["theme".to_string()]);
        
        assert!(CommandParser::parse_command_line("not a command").is_none());
        assert!(CommandParser::parse_command_line("/invalid").is_none());
    }
    
    #[test]
    fn test_command_categories() {
        assert_eq!(CliCommand::Help.category(), CommandCategory::Help);
        assert_eq!(CliCommand::Theme.category(), CommandCategory::Display);
        assert_eq!(CliCommand::Quit.category(), CommandCategory::Control);
    }
    
    #[test]
    fn test_command_validation() {
        let result = CommandParser::validate_args(&CliCommand::Theme, &[]);
        assert!(result.is_ok());
        
        let result = CommandParser::validate_args(&CliCommand::Theme, &["dark".to_string()]);
        assert!(result.is_ok());
        
        let result = CommandParser::validate_args(&CliCommand::Theme, &["dark".to_string(), "extra".to_string()]);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_command_suggestions() {
        let suggestions = CommandParser::get_suggestions("th");
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("theme")));
    }
    
    #[test]
    fn test_command_result() {
        let success = CommandResult::Success("Done".to_string());
        assert!(success.is_success());
        assert!(!success.is_error());
        assert_eq!(success.message(), Some("Done"));
        
        let error = CommandResult::Error("Failed".to_string());
        assert!(!error.is_success());
        assert!(error.is_error());
        assert_eq!(error.message(), Some("Failed"));
    }
}