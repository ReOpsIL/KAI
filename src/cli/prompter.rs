//! Simple Terminal CLI Prompter
//!
//! A streamlined terminal interface that provides AI-powered planning
//! with simple text-based input/output, similar to regular terminal applications.

use std::io::{self, Write};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use inquire::{InquireError, Select};

use super::{
    commands::{CliCommand, CommandParser, CommandResult},
    config::CliConfig,
    file_browser::{FileBrowser, SelectionResult},
    history::CommandHistory,
};
use crate::context::context_data_store::ContextDataStore;
use crate::context::Context;
use crate::planer::Planner;

/// Types of output messages
#[derive(Debug, Clone)]
pub enum MessageType {
    Info,
    Error,
    Success,
    Warning,
    Planning,
    System,
    UserInput,
}

impl MessageType {
    pub fn prefix(&self) -> &'static str {
        match self {
            MessageType::Info => "🍜",
            MessageType::Error => "🌶️",
            MessageType::Success => "🍣",
            MessageType::Warning => "🥡",
            MessageType::Planning => "🥠",
            MessageType::System => "🍚",
            MessageType::UserInput => "🥢",
        }
    }
}

/// Simple terminal CLI prompter
pub struct CliPrompter {
    config: CliConfig,
    history: CommandHistory,
    file_browser: FileBrowser,
    should_exit: bool,
    planner: Option<Planner>,
    in_file_browser: bool,
    context: Context,
    context_data_store: ContextDataStore,
}

impl CliPrompter {
    /// Create a new simple CLI prompter instance
    pub fn new() -> io::Result<Self> {
        let config = CliConfig::default();
        let history = CommandHistory::new(config.max_history_size);
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
        let file_browser = FileBrowser::new(current_dir.clone());

        let context = Context::new();
        let context_data_store = ContextDataStore::new(current_dir);

        Ok(Self {
            config,
            history,
            file_browser,
            should_exit: false,
            planner: None,
            in_file_browser: false,
            context,
            context_data_store,
        })
    }

    /// Create CLI prompter with planner for AI-powered input processing
    pub fn with_planner(planner: Planner) -> io::Result<Self> {
        let mut prompter = Self::new()?;
        prompter.planner = Some(planner);
        Ok(prompter)
    }

    /// Initialize and update context
    pub async fn initialize_context(&mut self) -> io::Result<()> {
        // Get OpenRouter client from planner if available
        let openrouter_client = self
            .planner
            .as_ref()
            .and_then(|p| p.task_planner.get_llm_client())
            .map(|arc_client| (*arc_client).clone());

        // Update context with current file information
        match self
            .context
            .update(&self.context_data_store, openrouter_client, false)
            .await
        {
            Ok(()) => {
                self.print_info(&format!(
                    "Context initialized - tracking {} files",
                    self.context.tracked_files_count()
                ));
            }
            Err(e) => {
                self.print_warning(&format!("Context initialization failed: {}", e));
            }
        }

        Ok(())
    }

    /// Set planner after creation
    pub fn set_planner(&mut self, planner: Planner) {
        self.planner = Some(planner);
    }

    /// Print a message with type prefix
    fn print_message(&self, message_type: MessageType, content: &str) {
        let prefix = message_type.prefix();

        // Use plain text formatting without timestamp to avoid any display width issues
        println!("{} {}", prefix, content);
    }

    /// Print error message
    fn print_error(&self, message: &str) {
        self.print_message(MessageType::Error, message);
    }

    /// Print info message
    fn print_info(&self, message: &str) {
        self.print_message(MessageType::Info, message);
    }

    /// Print success message
    fn print_success(&self, message: &str) {
        self.print_message(MessageType::Success, message);
    }

    /// Print warning message
    fn print_warning(&self, message: &str) {
        self.print_message(MessageType::Warning, message);
    }

    /// Print system message
    fn print_system(&self, message: &str) {
        self.print_message(MessageType::System, message);
    }

    /// Print user input
    fn print_user_input(&self, input: &str) {
        self.print_message(MessageType::UserInput, input);
    }

    /// Print planning output
    fn print_planning(&self, content: &str) {
        self.print_message(MessageType::Planning, content);
    }

    /// Show welcome screen
    async fn show_welcome(&self) -> io::Result<()> {
        println!();
        self.print_system("🦀 KAI Enhanced CLI Prompter");
        self.print_info("AI-powered terminal interface for intelligent task planning");
        println!();
        self.print_info("Commands:");
        self.print_info("  - Type '/' to open command menu with auto-complete");
        self.print_info("  - Type '@' to open interactive file browser");
        self.print_info("  - Type '/help' for command help");
        self.print_info("  - Ctrl+C to exit");
        println!();
        self.print_success("Ready! Type your prompts and press Enter...");
        println!();
        Ok(())
    }

    /// Run the main CLI loop
    pub async fn run(&mut self) -> io::Result<()> {
        // Show welcome message
        self.show_welcome().await?;

        // Enable raw mode for character-by-character input
        enable_raw_mode().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let result = self.run_input_loop().await;

        // Always disable raw mode on exit
        let _ = disable_raw_mode();

        result
    }

    /// Main input loop with character-by-character detection
    async fn run_input_loop(&mut self) -> io::Result<()> {
        while !self.should_exit {
            print!("🦀 KAI: ");
            io::stdout().flush()?;

            let mut input_buffer = String::new();
            let mut cursor_pos = 0usize;

            loop {
                // Read events
                if event::poll(std::time::Duration::from_millis(100))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                {
                    let event =
                        event::read().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                    match event {
                        Event::Key(key_event) => {
                            if self
                                .handle_key_event(key_event, &mut input_buffer, &mut cursor_pos)
                                .await?
                            {
                                break; // Line completed or menu activated
                            }
                        }
                        _ => {} // Ignore other events
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle individual key events and return true if line is complete
    async fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        input_buffer: &mut String,
        cursor_pos: &mut usize,
    ) -> io::Result<bool> {
        match key_event {
            // Handle Ctrl+C for exit
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                self.should_exit = true;
                println!();
                return Ok(true);
            }

            // Handle Enter - process the complete input
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                println!(); // Move to new line
                if !input_buffer.trim().is_empty() {
                    self.process_line(input_buffer.trim()).await?;
                }
                input_buffer.clear();
                *cursor_pos = 0;
                return Ok(true);
            }

            // Handle Backspace
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                if *cursor_pos > 0 {
                    input_buffer.remove(*cursor_pos - 1);
                    *cursor_pos -= 1;
                    // Redraw the line
                    self.redraw_input_line(input_buffer, *cursor_pos)?;
                }
            }

            // Handle Left Arrow
            KeyEvent {
                code: KeyCode::Left,
                ..
            } => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                    print!("\x1B[D"); // Move cursor left
                    io::stdout().flush()?;
                }
            }

            // Handle Right Arrow
            KeyEvent {
                code: KeyCode::Right,
                ..
            } => {
                if *cursor_pos < input_buffer.len() {
                    *cursor_pos += 1;
                    print!("\x1B[C"); // Move cursor right
                    io::stdout().flush()?;
                }
            }

            // Handle regular characters
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
                ..
            }
            | KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => {
                // Check for immediate menu triggers
                if input_buffer.is_empty() && c == '/' {
                    // Commands menu - execute immediately
                    println!("{}", c); // Show the character

                    // Temporarily disable raw mode for inquire menus
                    let _ = disable_raw_mode();
                    let menu_result = self.show_command_menu().await;
                    let _ = enable_raw_mode();

                    menu_result?;
                    input_buffer.clear();
                    *cursor_pos = 0;
                    return Ok(true);
                } else if c == '@' && !self.in_file_browser {
                    // File browser - insert selected path at cursor position
                    // Only activate if not already in file browser
                    self.in_file_browser = true;

                    // Temporarily disable raw mode for inquire menus
                    let _ = disable_raw_mode();
                    let selected_path = self.show_file_browser().await?;
                    let _ = enable_raw_mode();

                    self.in_file_browser = false;

                    if let Some(path) = selected_path {
                        // Insert the path at cursor position
                        input_buffer.insert_str(*cursor_pos, &path);
                        *cursor_pos += path.len();

                        // Redraw the line
                        self.redraw_input_line(input_buffer, *cursor_pos)?;
                    } else {
                        // File browser was cancelled, restore current input line
                        if !input_buffer.is_empty() {
                            self.redraw_input_line(input_buffer, *cursor_pos)?;
                        }
                    }
                } else {
                    // Regular character input at cursor position
                    input_buffer.insert(*cursor_pos, c);
                    *cursor_pos += 1;

                    // If cursor is at end, just print the character
                    if *cursor_pos == input_buffer.len() {
                        print!("{}", c);
                        io::stdout().flush()?;
                    } else {
                        // Cursor is in middle, need to redraw the line
                        self.redraw_input_line(input_buffer, *cursor_pos)?;
                    }
                }
            }

            _ => {} // Ignore other key events
        }

        Ok(false)
    }

    /// Redraw the input line with cursor at the specified position
    fn redraw_input_line(&self, input_buffer: &str, cursor_pos: usize) -> io::Result<()> {
        // Move to beginning of line after prompt
        print!("\r🦀 KAI: ");

        // Clear to end of line and print the buffer
        print!("\x1B[K{}", input_buffer);

        // Move cursor to correct position
        let chars_after_cursor = input_buffer.len() - cursor_pos;
        if chars_after_cursor > 0 {
            print!("\x1B[{}D", chars_after_cursor); // Move left by chars_after_cursor
        }

        io::stdout().flush()?;
        Ok(())
    }

    /// Process a complete line of input
    async fn process_line(&mut self, input: &str) -> io::Result<()> {
        // Add to history
        self.history.add_command(input.to_string());

        // Show user input in log
        self.print_user_input(input);

        // Handle slash commands (but not single '/' which is handled immediately)
        if input.starts_with('/') && input.len() > 1 {
            if let Some((command, args)) = CommandParser::parse_command_line(input) {
                self.execute_command(command, args).await?;
                return Ok(());
            }
        }

        // Process with AI planner
        self.process_ai_input(input).await?;

        println!(); // Add spacing after processing
        Ok(())
    }

    /// Process input through AI planner
    async fn process_ai_input(&mut self, input: &str) -> io::Result<()> {
        // Add user input to context story
        self.context.add_user_prompt(input.to_string());

        if let Some(mut planner) = self.planner.take() {
            self.print_info("Processing with AI planner...");

            // Pass context to planner for enhanced prompt generation
            let planning_result = planner
                .create_and_execute_advanced_plan_with_context(input, &self.context)
                .await;

            match planning_result {
                Ok(result) => {
                    // Add response to context story
                    self.context.add_response(result.clone(), None);

                    self.print_system("=== AI Planning Result ===");
                    for line in result.lines() {
                        if line.trim().is_empty() {
                            println!();
                        } else {
                            self.print_planning(line);
                        }
                    }
                }
                Err(error) => {
                    // Still add error response to context for learning
                    self.context.add_response(format!("Error: {}", error), None);

                    self.print_error(&format!("AI planning failed: {}", error));
                    self.print_error("🦀 KAI requires working AI integration. Exiting...");
                    std::process::exit(1);
                }
            }

            self.planner = Some(planner);
        } else {
            self.print_error(
                "No AI planner available. 🦀 KAI requires AI integration to function.",
            );
            self.print_error("Ensure OpenRouter API key is properly configured and restart.");
            std::process::exit(1);
        }

        Ok(())
    }

    /// Execute a CLI command
    async fn execute_command(&mut self, command: CliCommand, _args: Vec<String>) -> io::Result<()> {
        let result = match command {
            CliCommand::Help => {
                let help_text = command.get_help_text();
                self.print_system("=== Help ===");
                for line in &help_text {
                    if !line.trim().is_empty() {
                        self.print_info(line);
                    }
                }
                CommandResult::Success("Help displayed".to_string())
            }
            CliCommand::History => {
                let history_summary = self.history.get_summary(Some(20));
                self.print_system("=== Command History ===");
                for line in &history_summary {
                    if !line.trim().is_empty() {
                        self.print_info(line);
                    }
                }
                CommandResult::Success("History displayed".to_string())
            }
            CliCommand::Clear => {
                // Clear terminal screen
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush()?;
                self.print_system("Terminal cleared");
                CommandResult::Success("Screen cleared".to_string())
            }
            CliCommand::Config => {
                let config_summary = self.config.get_summary();
                self.print_system("=== Configuration ===");
                for line in &config_summary {
                    if !line.trim().is_empty() {
                        self.print_info(line);
                    }
                }
                CommandResult::Success("Configuration displayed".to_string())
            }
            CliCommand::Quit => {
                self.should_exit = true;
                CommandResult::Exit
            }
            _ => {
                self.print_warning(&format!(
                    "Command '{:?}' not available in simple mode",
                    command
                ));
                CommandResult::Info("Use full CLI mode for advanced features".to_string())
            }
        };

        // Handle command result
        match result {
            CommandResult::Error(msg) => self.print_error(&msg),
            CommandResult::Warning(msg) => self.print_warning(&msg),
            CommandResult::Info(msg) => self.print_info(&msg),
            CommandResult::Success(_) | CommandResult::NoOp | CommandResult::Exit => {}
        }

        Ok(())
    }

    /// Show interactive command menu with inquire auto-complete
    async fn show_command_menu(&mut self) -> io::Result<()> {
        let commands = CliCommand::get_command_menu();

        let selection = Select::new("Select command:", commands)
            .with_page_size(10)
            .with_help_message("Use arrow keys to navigate, Enter to select, Esc to cancel")
            .prompt();

        match selection {
            Ok(cmd_str) => {
                // Parse the selected command (format: "/command - description")
                let cmd_part = cmd_str.split(" - ").next().unwrap_or(&cmd_str);
                if !cmd_part.trim().is_empty() && !cmd_part.starts_with("──") {
                    if let Some((command, args)) = CommandParser::parse_command_line(cmd_part) {
                        self.execute_command(command, args).await?;
                    }
                }
            }
            Err(InquireError::OperationCanceled) => {
                // Menu was cancelled - no action needed
            }
            Err(e) => {
                self.print_error(&format!("Command menu error: {}", e));
            }
        }

        Ok(())
    }

    /// Show interactive file browser with inquire auto-complete
    /// Returns the selected file/directory path, or None if cancelled
    async fn show_file_browser(&mut self) -> io::Result<Option<String>> {
        loop {
            let entries = match self.file_browser.read_current_directory() {
                Ok(entries) => entries,
                Err(e) => {
                    self.print_error(&format!("Cannot read directory: {}", e));
                    return Ok(None);
                }
            };

            let display_entries = match self.file_browser.get_display_entries() {
                Ok(entries) => entries,
                Err(e) => {
                    self.print_error(&format!("Cannot get directory entries: {}", e));
                    return Ok(None);
                }
            };

            let selection = Select::new("", display_entries)
                .with_page_size(15)
                .with_help_message("Navigate: ↑↓  Select: Enter  Cancel: Esc")
                .prompt();

            match selection {
                Ok(selected) => {
                    // Clear the inquire output more aggressively
                    print!("\x1B[3A"); // Move up 3 lines
                    for _ in 0..5 {
                        print!("\x1B[K\x1B[1B"); // Clear current line and move down
                    }
                    print!("\x1B[3A"); // Move back up 3 lines
                    io::stdout().flush()?;

                    let result = self.file_browser.process_selection(&selected, &entries);
                    match result {
                        SelectionResult::FileSelected(path) => {
                            // Return the selected path for insertion into input
                            return Ok(Some(path.display().to_string()));
                        }
                        SelectionResult::NavigatedTo(_) | SelectionResult::NavigatedUp => {
                            // Continue browsing
                        }
                        SelectionResult::Error(err) => {
                            self.print_error(&err);
                        }
                    }
                }
                Err(InquireError::OperationCanceled) => {
                    // Clear inquire output more aggressively for cancellation
                    print!("\x1B[1A\x1B[K"); // Move up 1 lines

                    // print!("\x1B[3A"); // Move up 3 lines
                    // for _ in 0..5 {
                    //     print!("\x1B[K\x1B[1B"); // Clear current line and move down
                    // }
                    print!("\r🦀 KAI: ");
                    io::stdout().flush()?;
                    return Ok(None);
                }
                Err(e) => {
                    self.print_error(&format!("File browser error: {}", e));
                    return Ok(None);
                }
            }
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &CliConfig {
        &self.config
    }

    /// Get command history
    pub fn history(&self) -> &CommandHistory {
        &self.history
    }
}
