//! Main CLI Prompter
//!
//! The core orchestration module that brings together all CLI components
//! into a cohesive interactive terminal application.

use chrono;
use std::io;
use std::path::PathBuf;

use crossterm::{
    cursor,
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
    },
    execute, terminal,
};
use inquire::{InquireError, Select};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color as RatatuiColor, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use tokio::time::Duration;

use super::{
    commands::{CliCommand, CommandParser, CommandResult},
    config::CliConfig,
    editor::{CursorDirection, TextEditor},
    file_browser::{FileBrowser, SelectionResult},
    history::CommandHistory,
    utils::{wait_for_key, wrap_text},
};
use crate::planer::Planner;

/// Focus state for the terminal frames
#[derive(Debug, Clone, PartialEq)]
pub enum FocusState {
    Output,
    Input,
}

/// Represents a single output message with timestamp and type
#[derive(Debug, Clone)]
pub struct OutputMessage {
    pub timestamp: String,
    pub message_type: MessageType,
    pub content: String,
}

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
            MessageType::Info => "ℹ️",
            MessageType::Error => "❌",
            MessageType::Success => "✅",
            MessageType::Warning => "⚠️",
            MessageType::Planning => "🧠",
            MessageType::System => "🔧",
            MessageType::UserInput => "👤",
        }
    }

    pub fn color(&self) -> RatatuiColor {
        match self {
            MessageType::Info => RatatuiColor::Cyan,
            MessageType::Error => RatatuiColor::Red,
            MessageType::Success => RatatuiColor::Green,
            MessageType::Warning => RatatuiColor::Yellow,
            MessageType::Planning => RatatuiColor::Magenta,
            MessageType::System => RatatuiColor::Gray,
            MessageType::UserInput => RatatuiColor::Blue,
        }
    }
}

/// Main CLI prompter application
pub struct CliPrompter {
    config: CliConfig,
    history: CommandHistory,
    editor: TextEditor,
    file_browser: FileBrowser,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    should_exit: bool,
    escape_count: u8,
    ctrl_c_count: u8,
    clipboard: String,
    planner: Option<Planner>,
    output_buffer: Vec<OutputMessage>,
    scroll_position: usize,
    max_output_lines: usize,
    focus_state: FocusState,
}

impl CliPrompter {
    /// Create a new CLI prompter instance
    pub fn new() -> io::Result<Self> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        terminal::enable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            terminal::EnterAlternateScreen,
            cursor::Show,
            event::EnableMouseCapture
        )?;

        let config = CliConfig::default();
        let history = CommandHistory::new(config.max_history_size);
        let (width, _) = terminal::size()?;
        let editor = TextEditor::new(width as usize);
        let file_browser =
            FileBrowser::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));

        Ok(Self {
            config,
            history,
            editor,
            file_browser,
            terminal,
            should_exit: false,
            escape_count: 0,
            ctrl_c_count: 0,
            clipboard: String::new(),
            planner: None,
            output_buffer: Vec::new(),
            scroll_position: 0,
            max_output_lines: 1000,
            focus_state: FocusState::Input, // Start with input focused
        })
    }

    /// Create CLI prompter with custom configuration
    pub fn with_config(config: CliConfig) -> io::Result<Self> {
        let mut prompter = Self::new()?;
        prompter.config = config;
        prompter.output_buffer = Vec::new();
        prompter.scroll_position = 0;
        prompter.max_output_lines = 1000;
        prompter.focus_state = FocusState::Input;
        Ok(prompter)
    }

    /// Create CLI prompter with planner for AI-powered input processing
    pub fn with_planner(planner: Planner) -> io::Result<Self> {
        let mut prompter = Self::new()?;
        prompter.planner = Some(planner);
        Ok(prompter)
    }

    /// Set planner after creation
    pub fn set_planner(&mut self, planner: Planner) {
        self.planner = Some(planner);
    }

    /// Add a message to the output buffer
    fn add_output_message(&mut self, message_type: MessageType, content: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        let message = OutputMessage {
            timestamp,
            message_type,
            content,
        };

        self.output_buffer.push(message);

        // Keep buffer size manageable
        if self.output_buffer.len() > self.max_output_lines {
            self.output_buffer
                .drain(0..self.output_buffer.len() - self.max_output_lines);
            // Adjust scroll position if needed
            if self.scroll_position > 0 {
                self.scroll_position = self
                    .scroll_position
                    .saturating_sub(self.output_buffer.len() - self.max_output_lines);
            }
        }

        // Auto-scroll to bottom for new messages
        self.scroll_to_bottom();
    }

    /// Scroll to the bottom of output
    fn scroll_to_bottom(&mut self) {
        self.scroll_position = self.output_buffer.len();
    }

    /// Scroll up in output
    fn scroll_up(&mut self, lines: usize) {
        self.scroll_position = self.scroll_position.saturating_sub(lines);
    }

    /// Scroll down in output
    fn scroll_down(&mut self, lines: usize) {
        self.scroll_position = (self.scroll_position + lines).min(self.output_buffer.len());
    }

    /// Switch focus between output and input frames
    fn switch_focus(&mut self) {
        self.focus_state = match self.focus_state {
            FocusState::Output => FocusState::Input,
            FocusState::Input => FocusState::Output,
        };
    }

    /// Get the current focus state
    fn is_output_focused(&self) -> bool {
        self.focus_state == FocusState::Output
    }

    /// Get the current focus state
    fn is_input_focused(&self) -> bool {
        self.focus_state == FocusState::Input
    }

    /// Run the main CLI loop
    pub async fn run(&mut self) -> io::Result<()> {
        self.show_welcome().await?;

        while !self.should_exit {
            self.render_frame()?;

            if let Err(e) = self.handle_input().await {
                self.show_error(&format!("Input error: {}", e))?;
            }
        }

        self.cleanup()?;
        Ok(())
    }

    /// Clean up terminal state
    fn cleanup(&mut self) -> io::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            terminal::LeaveAlternateScreen,
            cursor::Show,
            event::DisableMouseCapture
        )?;
        Ok(())
    }

    /// Show welcome screen
    async fn show_welcome(&mut self) -> io::Result<()> {
        // Add welcome messages to output buffer
        self.add_output_message(
            MessageType::System,
            "🦀 KAI Enhanced CLI Prompter".to_string(),
        );
        self.add_output_message(
            MessageType::Info,
            "Enhanced terminal interface with AI-powered planning and focus system".to_string(),
        );
        self.add_output_message(
            MessageType::Info,
            "• Tab key to switch focus between Output and Input areas".to_string(),
        );
        self.add_output_message(
            MessageType::Info,
            "• Click on areas to focus them".to_string(),
        );
        self.add_output_message(
            MessageType::Info,
            "• PageUp/PageDown to scroll (when Output focused)".to_string(),
        );
        self.add_output_message(
            MessageType::Info,
            "• Type '/' for commands, '@' for file browser (when Input focused)".to_string(),
        );
        self.add_output_message(MessageType::Info, "• Ctrl+C to exit".to_string());
        self.add_output_message(
            MessageType::Success,
            "Ready! Input area is focused - start typing your prompt...".to_string(),
        );

        Ok(())
    }

    /// Handle user input
    async fn handle_input(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    return self.process_key_event(key).await;
                }
                Event::Mouse(mouse) => {
                    return self.process_mouse_event(mouse).await;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Process mouse events for focus management
    async fn process_mouse_event(&mut self, mouse: MouseEvent) -> io::Result<()> {
        if let MouseEventKind::Down(_) = mouse.kind {
            let (_, terminal_height) = terminal::size()?;
            let input_area_start = terminal_height.saturating_sub(5); // Input area is 5 lines high

            // Determine which area was clicked based on Y coordinate
            if mouse.row < input_area_start {
                // Clicked in output area
                if self.focus_state != FocusState::Output {
                    self.focus_state = FocusState::Output;
                    //self.add_output_message(MessageType::System, "Output area focused - Use PageUp/PageDown to scroll".to_string());
                }
            } else {
                // Clicked in input area
                if self.focus_state != FocusState::Input {
                    self.focus_state = FocusState::Input;
                    //self.add_output_message(MessageType::System, "Input area focused - Type your commands".to_string());
                }
            }
        }
        Ok(())
    }

    /// Process individual key events
    async fn process_key_event(&mut self, key: KeyEvent) -> io::Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.handle_control_keys(key).await;
        }

        match key.code {
            KeyCode::Tab => {
                // Switch focus between frames
                self.switch_focus();
                let focus_msg = match self.focus_state {
                    FocusState::Output => {
                        "Output area focused - Use PageUp/PageDown to scroll, Tab to switch back"
                    }
                    FocusState::Input => {
                        "Input area focused - Type your commands, Tab to switch to output"
                    }
                };
                self.add_output_message(MessageType::System, focus_msg.to_string());
            }
            KeyCode::Char(ch) => {
                // Only handle text input when input area is focused
                if self.is_input_focused() {
                    if ch == self.config.command_prefix && self.is_at_word_boundary() {
                        self.show_command_menu().await?;
                    } else if ch == self.config.file_browser_prefix && self.is_at_word_boundary() {
                        self.show_file_browser().await?;
                    } else {
                        // Reset history navigation and counters when user starts typing
                        self.history.reset_navigation();
                        self.editor.insert_char(ch);
                        self.escape_count = 0;
                        self.ctrl_c_count = 0;
                    }
                }
            }
            KeyCode::Backspace => {
                // Only handle when input area is focused
                if self.is_input_focused() {
                    self.history.reset_navigation();
                    self.editor.delete_char();
                    self.escape_count = 0;
                    self.ctrl_c_count = 0;
                }
            }
            KeyCode::Delete => {
                // Only handle when input area is focused
                if self.is_input_focused() {
                    self.history.reset_navigation();
                    self.editor.delete_char_forward();
                    self.escape_count = 0;
                    self.ctrl_c_count = 0;
                }
            }
            KeyCode::Enter => {
                // Only handle when input area is focused
                if self.is_input_focused() {
                    let input = self.editor.get_text();
                    if !input.trim().is_empty() {
                        // Add user input to the output buffer before processing
                        self.add_output_message(MessageType::UserInput, input.clone());

                        self.process_input(&input).await?;
                        self.history.add_command(input);
                        let (width, _) = terminal::size()?;
                        self.editor = TextEditor::new(width as usize);
                    } else {
                        self.editor.handle_enter();
                    }
                }
            }
            KeyCode::Left => {
                // Only handle when input area is focused
                if self.is_input_focused() {
                    self.editor.move_cursor(CursorDirection::Left);
                }
            }
            KeyCode::Right => {
                // Only handle when input area is focused
                if self.is_input_focused() {
                    self.editor.move_cursor(CursorDirection::Right);
                }
            }
            KeyCode::Up => {
                // Only handle when input area is focused
                if self.is_input_focused() {
                    // Store current line before navigating if not already in navigation mode
                    if self.history.current_index().is_none() {
                        let current_text = self.editor.get_text();
                        if !current_text.trim().is_empty() {
                            self.history.store_current_line(current_text);
                        }
                    }

                    // Navigate to previous command in history
                    if let Some(prev_command) = self.history.previous() {
                        let (width, _) = terminal::size()?;
                        self.editor = TextEditor::from_text(&prev_command, width as usize);
                    } else {
                        // If no history or single line, allow cursor movement within text
                        if self.editor.line_count() > 1 {
                            self.editor.move_cursor(CursorDirection::Up);
                        }
                    }
                    // Reset escape and ctrl-c counters on navigation
                    self.escape_count = 0;
                    self.ctrl_c_count = 0;
                }
            }
            KeyCode::Down => {
                // Only handle when input area is focused
                if self.is_input_focused() {
                    // Check if we're currently showing the stored line
                    let was_at_stored_line = self.history.is_at_stored_line();

                    // Navigate to next command in history or handle special cases
                    match self.history.next() {
                        Some(next_command) => {
                            let (width, _) = terminal::size()?;
                            self.editor = TextEditor::from_text(&next_command, width as usize);
                        }
                        None => {
                            // At end of history
                            if self.history.current_index().is_none()
                                && self.editor.line_count() > 1
                            {
                                // Allow cursor movement within text if multi-line and not in history mode
                                self.editor.move_cursor(CursorDirection::Down);
                            } else if was_at_stored_line {
                                // We were at stored line, now go to empty but keep stored line
                                let (width, _) = terminal::size()?;
                                self.editor = TextEditor::new(width as usize);
                                // Don't clear stored line - it remains recoverable by pressing Up
                            } else {
                                // Clear editor when going past end of history (no stored line case)
                                let (width, _) = terminal::size()?;
                                self.editor = TextEditor::new(width as usize);
                            }
                        }
                    }
                    // Reset escape and ctrl-c counters on navigation
                    self.escape_count = 0;
                    self.ctrl_c_count = 0;
                }
            }
            KeyCode::Home => {
                // Only handle when input area is focused
                if self.is_input_focused() {
                    self.editor.move_cursor(CursorDirection::Home);
                }
            }
            KeyCode::End => {
                // Only handle when input area is focused
                if self.is_input_focused() {
                    self.editor.move_cursor(CursorDirection::End);
                }
            }
            KeyCode::Esc => {
                self.escape_count += 1;
                self.ctrl_c_count = 0; // Reset ctrl-c counter

                if self.escape_count >= 3 {
                    // Clear everything on 3rd escape press
                    let (width, _) = terminal::size()?;
                    self.editor = TextEditor::new(width as usize);
                    self.history.reset_navigation();
                    self.escape_count = 0;
                    self.show_info("Prompt cleared (3x Escape)")?;
                } else {
                    // Show how many more escapes needed
                    let remaining = 3 - self.escape_count;
                    self.show_info(&format!(
                        "Press Escape {} more time(s) to clear prompt",
                        remaining
                    ))?;
                }
            }
            KeyCode::PageUp => {
                // Only scroll when output area is focused
                if self.is_output_focused() {
                    self.scroll_up(5);
                    self.escape_count = 0;
                    self.ctrl_c_count = 0;
                } else {
                    // Provide helpful feedback when trying to scroll without focus
                    //self.add_output_message(MessageType::System, "PageUp/PageDown only works when output area is focused. Press Tab to switch focus.".to_string());
                }
            }
            KeyCode::PageDown => {
                // Only scroll when output area is focused
                if self.is_output_focused() {
                    self.scroll_down(5);
                    self.escape_count = 0;
                    self.ctrl_c_count = 0;
                } else {
                    // Provide helpful feedback when trying to scroll without focus
                    //self.add_output_message(MessageType::System, "PageUp/PageDown only works when output area is focused. Press Tab to switch focus.".to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle control key combinations
    async fn handle_control_keys(&mut self, key: KeyEvent) -> io::Result<()> {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.ctrl_c_count += 1;
                self.escape_count = 0; // Reset escape counter

                if self.ctrl_c_count >= 3 {
                    // Exit application on 3rd Ctrl-C press
                    self.should_exit = true;
                } else if self.ctrl_c_count == 1 {
                    // Copy selected text or current line to clipboard on first Ctrl-C
                    let current_text = self.editor.get_text();
                    if !current_text.trim().is_empty() {
                        self.clipboard = current_text.clone();
                        self.show_info("Text copied to clipboard (Ctrl-C)")?;
                    }
                    // Don't exit yet, wait for more Ctrl-C presses or timeout
                } else {
                    // Second Ctrl-C - show warning
                    self.show_info("Press Ctrl-C one more time to exit")?;
                }
            }
            KeyCode::Char('a') => self.editor.move_cursor(CursorDirection::Home),
            KeyCode::Char('e') => self.editor.move_cursor(CursorDirection::End),
            KeyCode::Char('d') => self.editor.delete_char_forward(),
            KeyCode::Char('h') => self.editor.delete_char(),
            KeyCode::Char('k') => self.editor.delete_to_end_of_line(),
            KeyCode::Char('u') => self.editor.delete_line(),
            KeyCode::Char('w') => self.editor.delete_word_backward(),
            KeyCode::Char('l') => {
                self.terminal.clear()?;
            }
            KeyCode::Char('r') => {
                self.show_history_search().await?;
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                // Paste from clipboard
                if !self.clipboard.is_empty() {
                    for ch in self.clipboard.chars() {
                        if ch == '\n' {
                            self.editor.handle_enter();
                        } else {
                            self.editor.insert_char(ch);
                        }
                    }
                    self.show_info("Text pasted from clipboard (Ctrl-V)")?;
                }
                // Reset counters
                self.escape_count = 0;
                self.ctrl_c_count = 0;
            }
            _ => {}
        }
        Ok(())
    }

    /// Check if cursor is at a word boundary
    fn is_at_word_boundary(&self) -> bool {
        let (cursor_x, cursor_y) = self.editor.get_cursor_position();
        if cursor_x == 0 {
            return true;
        }

        let current_line = self.editor.current_line();
        if let Some(prev_char) = current_line.chars().nth(cursor_x - 1) {
            return prev_char.is_whitespace();
        }

        true
    }

    /// Show command menu
    async fn show_command_menu(&mut self) -> io::Result<()> {
        let commands = CliCommand::get_command_menu();

        // Clear screen before showing menu
        self.terminal.clear()?;
        terminal::disable_raw_mode()?;

        let selection = Select::new("Select command:", commands)
            .with_page_size(10)
            .with_help_message("Use arrow keys to navigate, Enter to select, Esc to cancel")
            .prompt();

        terminal::enable_raw_mode()?;

        // Clear screen after menu interaction
        self.terminal.clear()?;

        match selection {
            Ok(cmd_str) => {
                if let Some((command, args)) =
                    CommandParser::parse_command_line(&format!("/{}", cmd_str))
                {
                    self.execute_command(command, args).await?;
                }
            }
            Err(InquireError::OperationCanceled) => {
                // Menu was cancelled - no action needed, screen already cleared
            }
            Err(e) => {
                self.show_error(&format!("Command menu error: {}", e))?;
            }
        }

        Ok(())
    }

    /// Show file browser
    async fn show_file_browser(&mut self) -> io::Result<()> {
        loop {
            let entries = match self.file_browser.read_current_directory() {
                Ok(entries) => entries,
                Err(e) => {
                    self.show_error(&format!("Cannot read directory: {}", e))?;
                    break;
                }
            };

            let display_entries = match self.file_browser.get_display_entries() {
                Ok(entries) => entries,
                Err(e) => {
                    self.show_error(&format!("Cannot get directory entries: {}", e))?;
                    break;
                }
            };

            // Clear screen before showing file browser
            self.terminal.clear()?;
            terminal::disable_raw_mode()?;

            let selection = Select::new(
                &format!(
                    "Browse: {} - Select file or directory:",
                    self.file_browser.current_path().display()
                ),
                display_entries,
            )
            .with_page_size(15)
            .with_help_message("Use arrow keys to navigate, Enter to select, Esc to cancel")
            .prompt();

            terminal::enable_raw_mode()?;

            match selection {
                Ok(selected) => {
                    let result = self.file_browser.process_selection(&selected, &entries);
                    match result {
                        SelectionResult::FileSelected(path) => {
                            // Clear screen after file selection
                            self.terminal.clear()?;
                            self.editor.insert_char('@');
                            for ch in path.to_string_lossy().chars() {
                                self.editor.insert_char(ch);
                            }
                            self.editor.insert_char(' ');
                            break;
                        }
                        SelectionResult::NavigatedTo(_) | SelectionResult::NavigatedUp => {
                            // Continue browsing - don't clear screen yet
                        }
                        SelectionResult::Error(err) => {
                            self.terminal.clear()?;
                            self.show_error(&err)?;
                        }
                    }
                }
                Err(InquireError::OperationCanceled) => {
                    // Clear screen when cancelled
                    self.terminal.clear()?;
                    break;
                }
                Err(e) => {
                    self.terminal.clear()?;
                    self.show_error(&format!("File browser error: {}", e))?;
                    break;
                }
            }
        }

        Ok(())
    }

    /// Show history search interface
    async fn show_history_search(&mut self) -> io::Result<()> {
        let history_items = self.history.get_all();
        if history_items.is_empty() {
            self.show_info("No command history available")?;
            return Ok(());
        }

        // Clear screen before showing history
        self.terminal.clear()?;
        terminal::disable_raw_mode()?;

        let selection = Select::new("Command History - Select to insert:", history_items)
            .with_page_size(10)
            .with_help_message("Use arrow keys to navigate, Enter to select, Esc to cancel")
            .prompt();

        terminal::enable_raw_mode()?;

        // Clear screen after history interaction
        self.terminal.clear()?;

        match selection {
            Ok(selected_command) => {
                let (width, _) = terminal::size()?;
                self.editor = TextEditor::from_text(&selected_command, width as usize);
            }
            Err(InquireError::OperationCanceled) => {
                // History cancelled - no action needed, screen already cleared
            }
            Err(e) => {
                self.show_error(&format!("History search error: {}", e))?;
            }
        }

        Ok(())
    }

    /// Execute a CLI command
    async fn execute_command(&mut self, command: CliCommand, _args: Vec<String>) -> io::Result<()> {
        let result = match command {
            CliCommand::Help => {
                let help_text = command.get_help_text();
                self.add_output_message(MessageType::System, "=== Help ===".to_string());
                for line in &help_text {
                    if !line.trim().is_empty() {
                        self.add_output_message(MessageType::Info, line.clone());
                    }
                }
                CommandResult::Success("Help displayed".to_string())
            }
            CliCommand::History => {
                let history_summary = self.history.get_summary(Some(20));
                self.add_output_message(MessageType::System, "=== Command History ===".to_string());
                for line in &history_summary {
                    if !line.trim().is_empty() {
                        self.add_output_message(MessageType::Info, line.clone());
                    }
                }
                CommandResult::Success("History displayed".to_string())
            }
            CliCommand::Clear => {
                // Clear both terminal and output buffer
                self.terminal.clear()?;
                self.output_buffer.clear();
                self.scroll_position = 0;
                let (width, _) = terminal::size()?;
                self.editor = TextEditor::new(width as usize);
                self.add_output_message(MessageType::System, "Terminal cleared".to_string());
                CommandResult::Success("Screen cleared".to_string())
            }
            CliCommand::Config => {
                let config_summary = self.config.get_summary();
                self.add_output_message(MessageType::System, "=== Configuration ===".to_string());
                for line in &config_summary {
                    if !line.trim().is_empty() {
                        self.add_output_message(MessageType::Info, line.clone());
                    }
                }
                CommandResult::Success("Configuration displayed".to_string())
            }
            CliCommand::Theme => {
                self.show_theme_selector().await?;
                CommandResult::Success("Theme selector shown".to_string())
            }
            CliCommand::Quit => {
                self.should_exit = true;
                CommandResult::Exit
            }
            _ => CommandResult::Info(format!("Command '{:?}' not fully implemented yet", command)),
        };

        // Handle command result
        match result {
            CommandResult::Error(msg) => self.show_error(&msg)?,
            CommandResult::Warning(msg) => self.show_info(&format!("⚠️ {}", msg))?,
            CommandResult::Info(msg) => self.show_info(&msg)?,
            CommandResult::Success(_) | CommandResult::NoOp | CommandResult::Exit => {}
        }

        Ok(())
    }

    /// Show theme selector
    async fn show_theme_selector(&mut self) -> io::Result<()> {
        let themes = CliConfig::get_available_themes();

        terminal::disable_raw_mode()?;

        let selection = Select::new("Select Theme:", themes)
            .with_page_size(5)
            .with_help_message("Use arrow keys to navigate, Enter to select, Esc to cancel")
            .prompt();

        terminal::enable_raw_mode()?;

        match selection {
            Ok(theme_str) => {
                let theme_name = theme_str.split(' ').next().unwrap_or("default");
                self.config.apply_theme(theme_name);
                self.show_info(&format!("Applied theme: {}", theme_name))?;
            }
            Err(InquireError::OperationCanceled) => {}
            Err(e) => {
                self.show_error(&format!("Theme selection error: {}", e))?;
            }
        }

        Ok(())
    }

    /// Process user input through the AI planner
    async fn process_input(&mut self, input: &str) -> io::Result<()> {
        if input.trim().is_empty() {
            return Ok(());
        }

        // Check if we have a planner and process accordingly
        let has_planner = self.planner.is_some();

        if has_planner {
            // Show processing message
            self.show_info("🧠 Processing input with AI planner...")?;

            // Take the planner temporarily to avoid borrowing conflicts
            if let Some(mut planner) = self.planner.take() {
                // Use the AI-powered planner
                let planning_result = planner.create_and_execute_advanced_plan(input).await;

                match planning_result {
                    Ok(result) => {
                        self.show_planning_result("AI Planning Result", &result)?;
                    }
                    Err(error) => {
                        // Exit on AI planning failure - no fallback
                        self.show_error(&format!("❌ AI planning failed: {}", error))?;
                        self.show_error("🦀 KAI requires working AI integration. Exiting...")?;
                        std::process::exit(1);
                    }
                }

                // Put the planner back
                self.planner = Some(planner);
            }
        } else {
            // No planner available - this should never happen, exit immediately
            self.show_error(
                "❌ No AI planner available. 🦀 KAI requires AI integration to function.",
            )?;
            self.show_error("Ensure OpenRouter API key is properly configured and restart.")?;
            std::process::exit(1);
        }

        Ok(())
    }

    /// Render the main frame (positioned at bottom of screen)
    fn render_frame(&mut self) -> io::Result<()> {
        let editor_text = self.editor.get_text();
        let frame_color = self.config.get_frame_color();
        let text_color = self.config.get_text_color();

        // Prepare output data before drawing to avoid borrow conflicts
        let output_buffer_clone = self.output_buffer.clone();
        let scroll_position = self.scroll_position;
        let (editor_cursor_x, editor_cursor_y) = self.editor.get_cursor_position();
        let is_output_focused = self.is_output_focused();
        let is_input_focused = self.is_input_focused();

        self.terminal.draw(|f| {
            let area = f.size();

            // Create vertical layout: output area (top) and input area (bottom)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),    // Output area takes most space
                    Constraint::Length(5), // Input area gets 5 lines (3 content + 2 borders)
                ])
                .split(area);

            let output_area = chunks[0];
            let input_area = chunks[1];

            // Render output area with focus indicator
            let output_title = if is_output_focused {
                "● Output [FOCUSED]"
            } else {
                "○ Output"
            };
            let output_border_color = if is_output_focused {
                frame_color
            } else {
                RatatuiColor::DarkGray
            };
            let output_block = Block::default()
                .title(output_title)
                .borders(Borders::ALL)
                .style(Style::default().fg(output_border_color));

            let inner_output_area = output_block.inner(output_area);
            let available_height = inner_output_area.height as usize;

            // Get visible messages based on scroll position
            let total_messages = output_buffer_clone.len();

            // Calculate which messages to show
            let end_index = scroll_position.min(total_messages);
            let start_index = if end_index > available_height {
                end_index - available_height
            } else {
                0
            };

            let visible_messages = if start_index < end_index && start_index < total_messages {
                &output_buffer_clone[start_index..end_index]
            } else {
                &[]
            };

            // Convert messages to Lines
            let output_lines: Vec<Line> = visible_messages
                .iter()
                .map(|msg| {
                    Line::from(vec![
                        Span::styled(
                            format!("[{}] ", msg.timestamp),
                            Style::default().fg(RatatuiColor::Gray),
                        ),
                        Span::styled(
                            format!("{} ", msg.message_type.prefix()),
                            Style::default().fg(msg.message_type.color()),
                        ),
                        Span::styled(msg.content.clone(), Style::default().fg(text_color)),
                    ])
                })
                .collect();

            // If no messages, show a placeholder
            let display_lines = if output_lines.is_empty() {
                vec![Line::from(Span::styled(
                    "🦀 KAI Terminal Ready - Type your commands below...",
                    Style::default().fg(RatatuiColor::Gray),
                ))]
            } else {
                output_lines
            };

            let output_paragraph = Paragraph::new(display_lines)
                .block(output_block)
                .wrap(Wrap { trim: true });

            f.render_widget(output_paragraph, output_area);

            // Render input area with focus indicator
            let input_title = if is_input_focused {
                "● Input [FOCUSED]"
            } else {
                "○ Input"
            };
            let input_border_color = if is_input_focused {
                frame_color
            } else {
                RatatuiColor::DarkGray
            };
            let input_block = Block::default()
                .title(input_title)
                .borders(Borders::ALL)
                .style(Style::default().fg(input_border_color));

            let inner_input_area = input_block.inner(input_area);

            // Wrap input text to fit the area
            let content_width = inner_input_area.width as usize;
            let wrapped_lines = wrap_text(&editor_text, content_width);

            let input_text: Vec<Line> = wrapped_lines
                .iter()
                .map(|line| Line::from(Span::styled(line.clone(), Style::default().fg(text_color))))
                .collect();

            let input_paragraph = Paragraph::new(input_text)
                .block(input_block)
                .wrap(Wrap { trim: false });

            f.render_widget(input_paragraph, input_area);

            // Calculate and set cursor position in input area using actual editor position
            let cursor_screen_x = inner_input_area.x
                + (editor_cursor_x as u16).min(inner_input_area.width.saturating_sub(1));
            let cursor_screen_y = inner_input_area.y
                + (editor_cursor_y as u16).min(inner_input_area.height.saturating_sub(1));

            f.set_cursor(cursor_screen_x, cursor_screen_y);
        })?;

        Ok(())
    }

    /// Show a message frame to the user
    fn show_message_frame(
        &mut self,
        title: &str,
        lines: &[String],
        call_wait_for_key: bool,
    ) -> io::Result<()> {
        let frame_color = self.config.get_frame_color();
        self.terminal.draw(|f| {
            let area = f.size();
            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(Style::default().fg(frame_color));

            let text: Vec<Line> = lines
                .iter()
                .map(|line| {
                    Line::from(Span::styled(
                        line.clone(),
                        Style::default().fg(RatatuiColor::White),
                    ))
                })
                .collect();

            let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

            f.render_widget(paragraph, area);
        })?;

        if call_wait_for_key {
            let _ = wait_for_key()?;
        }
        Ok(())
    }

    /// Show error message
    fn show_error(&mut self, message: &str) -> io::Result<()> {
        self.add_output_message(MessageType::Error, message.to_string());
        Ok(())
    }

    /// Show info message
    fn show_info(&mut self, message: &str) -> io::Result<()> {
        self.add_output_message(MessageType::Info, message.to_string());
        Ok(())
    }

    /// Show success message
    fn show_success(&mut self, message: &str) -> io::Result<()> {
        self.add_output_message(MessageType::Success, message.to_string());
        Ok(())
    }

    /// Show warning message
    fn show_warning(&mut self, message: &str) -> io::Result<()> {
        self.add_output_message(MessageType::Warning, message.to_string());
        Ok(())
    }

    /// Show system message
    fn show_system(&mut self, message: &str) -> io::Result<()> {
        self.add_output_message(MessageType::System, message.to_string());
        Ok(())
    }

    /// Show planning result with formatted output
    fn show_planning_result(&mut self, title: &str, result: &str) -> io::Result<()> {
        // Add title as a system message
        self.add_output_message(MessageType::System, format!("=== {} ===", title));

        // Split the result into lines and add each as a separate message to preserve formatting
        for line in result.lines() {
            if line.trim().is_empty() {
                // Add empty line as a system message for spacing
                self.add_output_message(MessageType::System, "".to_string());
            } else {
                self.add_output_message(MessageType::Planning, line.to_string());
            }
        }

        Ok(())
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

impl Drop for CliPrompter {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_prompter_creation() {
        // This test might not run in CI due to terminal requirements
        // but it's useful for local development
        if std::env::var("CI").is_err() {
            let result = CliPrompter::new();
            // Just check that it doesn't panic
            if let Ok(mut prompter) = result {
                let _ = prompter.cleanup();
            }
        }
    }
}
