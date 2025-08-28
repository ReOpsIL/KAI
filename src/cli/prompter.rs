//! Main CLI Prompter
//!
//! The core orchestration module that brings together all CLI components
//! into a cohesive interactive terminal application.

use std::io;
use std::path::PathBuf;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal,
};
use inquire::{InquireError, Select};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
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
            cursor::Show
        )?;

        let config = CliConfig::default();
        let history = CommandHistory::new(config.max_history_size);
        let (width, _) = terminal::size()?;
        let editor = TextEditor::new(width as usize);
        let file_browser = FileBrowser::new(
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
        );

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
        })
    }

    /// Create CLI prompter with custom configuration
    pub fn with_config(config: CliConfig) -> io::Result<Self> {
        let mut prompter = Self::new()?;
        prompter.config = config;
        Ok(prompter)
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
            cursor::Show
        )?;
        Ok(())
    }

    /// Show welcome screen
    async fn show_welcome(&mut self) -> io::Result<()> {
        let welcome_text = vec![
            "🦀 KAI Enhanced CLI Prompter".to_string(),
            "".to_string(),
            "Enhanced terminal interface with advanced features:".to_string(),
            "• Type '/' for commands menu".to_string(),
            "• Type '@' for file browser".to_string(),
            "• Ctrl+C to exit".to_string(),
            "• Ctrl+R for history search".to_string(),
            "".to_string(),
            "Start typing your prompt...".to_string(),
        ];

        self.terminal.draw(|f| {
            let area = f.size();
            let block = Block::default()
                .title("Welcome")
                .borders(Borders::ALL)
                .style(Style::default().fg(RatatuiColor::Blue));

            let text: Vec<Line> = welcome_text
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

        tokio::time::sleep(Duration::from_millis(2000)).await;
        Ok(())
    }

    /// Handle user input
    async fn handle_input(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    return self.process_key_event(key).await;
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
            KeyCode::Char(ch) => {
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
            KeyCode::Backspace => {
                self.history.reset_navigation();
                self.editor.delete_char();
                self.escape_count = 0;
                self.ctrl_c_count = 0;
            },
            KeyCode::Delete => {
                self.history.reset_navigation();
                self.editor.delete_char_forward();
                self.escape_count = 0;
                self.ctrl_c_count = 0;
            },
            KeyCode::Enter => {
                let input = self.editor.get_text();
                if !input.trim().is_empty() {
                    self.process_input(&input).await?;
                    self.history.add_command(input);
                    let (width, _) = terminal::size()?;
                    self.editor = TextEditor::new(width as usize);
                } else {
                    self.editor.handle_enter();
                }
            }
            KeyCode::Left => self.editor.move_cursor(CursorDirection::Left),
            KeyCode::Right => self.editor.move_cursor(CursorDirection::Right),
            KeyCode::Up => {
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
            },
            KeyCode::Down => {
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
                        if self.history.current_index().is_none() && self.editor.line_count() > 1 {
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
            },
            KeyCode::Home => self.editor.move_cursor(CursorDirection::Home),
            KeyCode::End => self.editor.move_cursor(CursorDirection::End),
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
                    self.show_info(&format!("Press Escape {} more time(s) to clear prompt", remaining))?;
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
                if let Some((command, args)) = CommandParser::parse_command_line(&format!("/{}", cmd_str)) {
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
                self.show_message_frame("Help", &help_text)?;
                CommandResult::Success("Help displayed".to_string())
            }
            CliCommand::History => {
                let history_summary = self.history.get_summary(Some(20));
                self.show_message_frame("History", &history_summary)?;
                CommandResult::Success("History displayed".to_string())
            }
            CliCommand::Clear => {
                self.terminal.clear()?;
                let (width, _) = terminal::size()?;
                self.editor = TextEditor::new(width as usize);
                CommandResult::Success("Screen cleared".to_string())
            }
            CliCommand::Config => {
                let config_summary = self.config.get_summary();
                self.show_message_frame("Configuration", &config_summary)?;
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

    /// Process user input
    async fn process_input(&mut self, input: &str) -> io::Result<()> {
        if input.trim().is_empty() {
            return Ok(());
        }

        self.show_info(&format!("Processed input: {}", input.trim()))?;
        Ok(())
    }

    /// Render the main frame (positioned at bottom of screen)
    fn render_frame(&mut self) -> io::Result<()> {
        let editor_text = self.editor.get_text();
        let frame_color = self.config.get_frame_color();

        self.terminal.draw(|f| {
            let area = f.size();

            let content_width = (area.width as usize).min(120).max(60);
            let wrapped_lines = wrap_text(&editor_text, content_width - 4);
            let content_height = (wrapped_lines.len().max(3) + 2).min(area.height as usize / 2);

            // Position frame at bottom of screen
            let frame_rect = Rect {
                x: (area.width.saturating_sub(content_width as u16)) / 2,
                y: area.height.saturating_sub(content_height as u16),
                width: content_width as u16,
                height: content_height as u16,
            };

            let block = Block::default()
                .title("KAI 🦀")
                .borders(Borders::ALL)
                .style(Style::default().fg(frame_color));

            let inner_area = block.inner(frame_rect);

            let text: Vec<Line> = wrapped_lines
                .iter()
                .map(|line| {
                    Line::from(Span::styled(
                        line.clone(),
                        Style::default().fg(RatatuiColor::White),
                    ))
                })
                .collect();

            let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });

            f.render_widget(paragraph, frame_rect);

            // Calculate and set cursor position
            let (cursor_x, cursor_y) = self.editor.get_cursor_position();
            let cursor_screen_x = inner_area.x + (cursor_x as u16).min(inner_area.width.saturating_sub(1));
            let cursor_screen_y = inner_area.y + (cursor_y as u16).min(inner_area.height.saturating_sub(1));
            
            f.set_cursor(cursor_screen_x, cursor_screen_y);
        })?;

        Ok(())
    }

    /// Show a message frame to the user
    fn show_message_frame(&mut self, title: &str, lines: &[String]) -> io::Result<()> {
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

        let _ = wait_for_key()?;
        Ok(())
    }

    /// Show error message
    fn show_error(&mut self, message: &str) -> io::Result<()> {
        self.show_message_frame(
            "Error",
            &[
                format!("❌ {}", message),
                "".to_string(),
                "Press any key to continue...".to_string(),
            ],
        )?;
        Ok(())
    }

    /// Show info message
    fn show_info(&mut self, message: &str) -> io::Result<()> {
        self.show_message_frame(
            "Info",
            &[
                format!("ℹ️ {}", message),
                "".to_string(),
                "Press any key to continue...".to_string(),
            ],
        )?;
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