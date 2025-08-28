//! Enhanced CLI Terminal Prompter
//!
//! A sophisticated terminal-based prompt CLI application that provides an interactive 
//! text input experience with special command handling, file system navigation, 
//! and advanced text editing capabilities.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::io;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal
};
use inquire::{InquireError, Select};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color as RatatuiColor, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal
};
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

/// Configuration for the CLI prompter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    pub frame_color: String,
    pub text_color: String, 
    pub command_prefix: char,
    pub file_browser_prefix: char,
    pub auto_save_history: bool,
    pub max_history_size: usize,
    pub custom_keybindings: HashMap<String, String>,
    pub theme_name: String,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            frame_color: "DarkBlue".to_string(),
            text_color: "White".to_string(),
            command_prefix: '/',
            file_browser_prefix: '@',
            auto_save_history: true,
            max_history_size: 1000,
            custom_keybindings: HashMap::new(),
            theme_name: "default".to_string(),
        }
    }
}

/// Command history manager with efficient storage
#[derive(Debug, Clone)]
pub struct CommandHistory {
    commands: VecDeque<String>,
    max_size: usize,
    current_index: Option<usize>,
}

impl CommandHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            commands: VecDeque::new(),
            max_size,
            current_index: None,
        }
    }
    
    pub fn add_command(&mut self, cmd: String) {
        if !cmd.trim().is_empty() && !self.commands.back().map_or(false, |last| last == &cmd) {
            if self.commands.len() >= self.max_size {
                self.commands.pop_front();
            }
            self.commands.push_back(cmd);
        }
        self.current_index = None;
    }
    
    pub fn previous(&mut self) -> Option<&String> {
        if self.commands.is_empty() {
            return None;
        }
        
        match self.current_index {
            None => {
                self.current_index = Some(self.commands.len() - 1);
                self.commands.back()
            }
            Some(idx) if idx > 0 => {
                self.current_index = Some(idx - 1);
                self.commands.get(idx - 1)
            }
            Some(_) => self.commands.get(0),
        }
    }
    
    pub fn next(&mut self) -> Option<&String> {
        match self.current_index {
            None => None,
            Some(idx) if idx < self.commands.len() - 1 => {
                self.current_index = Some(idx + 1);
                self.commands.get(idx + 1)
            }
            Some(_) => {
                self.current_index = None;
                None
            }
        }
    }
    
    pub fn get_all(&self) -> Vec<String> {
        self.commands.iter().cloned().collect()
    }
}

/// Multi-line text editor with cursor management
#[derive(Debug, Clone)]
pub struct TextEditor {
    lines: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    scroll_offset: usize,
    max_width: usize,
}

impl TextEditor {
    pub fn new(max_width: usize) -> Self {
        Self {
            lines: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
            scroll_offset: 0,
            max_width,
        }
    }
    
    pub fn from_text(text: &str, max_width: usize) -> Self {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(|s| s.to_string()).collect()
        };
        
        Self {
            lines,
            cursor_x: 0,
            cursor_y: 0,
            scroll_offset: 0,
            max_width,
        }
    }
    
    pub fn get_text(&self) -> String {
        self.lines.join("\n")
    }
    
    pub fn insert_char(&mut self, ch: char) {
        let current_line = &mut self.lines[self.cursor_y];
        current_line.insert(self.cursor_x, ch);
        self.cursor_x += 1;
    }
    
    pub fn delete_char(&mut self) {
        let current_line = &mut self.lines[self.cursor_y];
        if self.cursor_x > 0 {
            current_line.remove(self.cursor_x - 1);
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            let line = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.lines[self.cursor_y].len();
            self.lines[self.cursor_y].push_str(&line);
        }
    }
    
    pub fn delete_char_forward(&mut self) {
        let current_line = &mut self.lines[self.cursor_y];
        if self.cursor_x < current_line.len() {
            current_line.remove(self.cursor_x);
        } else if self.cursor_y < self.lines.len() - 1 {
            let next_line = self.lines.remove(self.cursor_y + 1);
            self.lines[self.cursor_y].push_str(&next_line);
        }
    }
    
    pub fn handle_enter(&mut self) {
        let line_content = self.lines[self.cursor_y].clone();
        let (left, right) = line_content.split_at(self.cursor_x);
        
        self.lines[self.cursor_y] = left.to_string();
        self.lines.insert(self.cursor_y + 1, right.to_string());
        
        self.cursor_y += 1;
        self.cursor_x = 0;
    }
    
    pub fn move_cursor(&mut self, direction: CursorDirection) {
        match direction {
            CursorDirection::Left => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = self.lines[self.cursor_y].len();
                }
            }
            CursorDirection::Right => {
                let current_line_len = self.lines[self.cursor_y].len();
                if self.cursor_x < current_line_len {
                    self.cursor_x += 1;
                } else if self.cursor_y < self.lines.len() - 1 {
                    self.cursor_y += 1;
                    self.cursor_x = 0;
                }
            }
            CursorDirection::Up => {
                if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    let line_len = self.lines[self.cursor_y].len();
                    self.cursor_x = self.cursor_x.min(line_len);
                }
            }
            CursorDirection::Down => {
                if self.cursor_y < self.lines.len() - 1 {
                    self.cursor_y += 1;
                    let line_len = self.lines[self.cursor_y].len();
                    self.cursor_x = self.cursor_x.min(line_len);
                }
            }
            CursorDirection::Home => {
                self.cursor_x = 0;
            }
            CursorDirection::End => {
                self.cursor_x = self.lines[self.cursor_y].len();
            }
        }
    }
    
    pub fn delete_line(&mut self) {
        self.lines[self.cursor_y].clear();
        self.cursor_x = 0;
    }
    
    pub fn delete_to_end_of_line(&mut self) {
        let current_line = &mut self.lines[self.cursor_y];
        current_line.truncate(self.cursor_x);
    }
    
    pub fn delete_word_backward(&mut self) {
        let current_line = &mut self.lines[self.cursor_y];
        if self.cursor_x == 0 {
            return;
        }
        
        let mut new_x = self.cursor_x;
        let chars: Vec<char> = current_line.chars().collect();
        
        while new_x > 0 && chars[new_x - 1].is_whitespace() {
            new_x -= 1;
        }
        
        while new_x > 0 && !chars[new_x - 1].is_whitespace() {
            new_x -= 1;
        }
        
        current_line.drain(new_x..self.cursor_x);
        self.cursor_x = new_x;
    }
    
    pub fn get_wrapped_lines(&self) -> Vec<String> {
        let mut wrapped = Vec::new();
        for line in &self.lines {
            wrapped.extend(wrap_text(line, self.max_width - 4));
        }
        wrapped
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CursorDirection {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}

/// Available CLI commands
#[derive(Debug, Clone)]
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
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "help" => Some(Self::Help),
            "history" => Some(Self::History),
            "clear" => Some(Self::Clear),
            "config" => Some(Self::Config),
            "templates" => Some(Self::Templates),
            "export" => Some(Self::Export),
            "quit" | "exit" | "q" => Some(Self::Quit),
            "save" => Some(Self::Save),
            "load" => Some(Self::Load),
            "theme" => Some(Self::Theme),
            "keybinds" => Some(Self::KeyBinds),
            _ => None,
        }
    }
    
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
}

/// File system entry with metadata
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub size: Option<u64>,
}

/// Main CLI prompter application
pub struct CliPrompter {
    config: CliConfig,
    history: CommandHistory,
    editor: TextEditor,
    current_directory: PathBuf,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    should_exit: bool,
}

impl CliPrompter {
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
        
        Ok(Self {
            config,
            history,
            editor,
            current_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            terminal,
            should_exit: false,
        })
    }
    
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
    
    fn cleanup(&mut self) -> io::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            terminal::LeaveAlternateScreen,
            cursor::Show
        )?;
        Ok(())
    }
    
    async fn show_welcome(&mut self) -> io::Result<()> {
        let welcome_text = vec![
            "<� KAI Enhanced CLI Prompter".to_string(),
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
                
            let text: Vec<Line> = welcome_text.iter()
                .map(|line| Line::from(Span::styled(line.clone(), Style::default().fg(RatatuiColor::White))))
                .collect();
                
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: true });
                
            f.render_widget(paragraph, area);
        })?;
        
        tokio::time::sleep(Duration::from_millis(2000)).await;
        Ok(())
    }
    
    
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
                    self.editor.insert_char(ch);
                }
            }
            KeyCode::Backspace => self.editor.delete_char(),
            KeyCode::Delete => self.editor.delete_char_forward(),
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
            KeyCode::Up => self.editor.move_cursor(CursorDirection::Up),
            KeyCode::Down => self.editor.move_cursor(CursorDirection::Down),
            KeyCode::Home => self.editor.move_cursor(CursorDirection::Home),
            KeyCode::End => self.editor.move_cursor(CursorDirection::End),
            KeyCode::Esc => {
                if !self.editor.get_text().trim().is_empty() {
                    let (width, _) = terminal::size()?;
                    self.editor = TextEditor::new(width as usize);
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn handle_control_keys(&mut self, key: KeyEvent) -> io::Result<()> {
        match key.code {
            KeyCode::Char('c') => {
                self.should_exit = true;
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
            _ => {}
        }
        Ok(())
    }
    
    fn is_at_word_boundary(&self) -> bool {
        if self.editor.cursor_x == 0 {
            return true;
        }
        
        if let Some(line) = self.editor.lines.get(self.editor.cursor_y) {
            if let Some(prev_char) = line.chars().nth(self.editor.cursor_x - 1) {
                return prev_char.is_whitespace();
            }
        }
        
        true
    }
    
    async fn show_command_menu(&mut self) -> io::Result<()> {
        let commands = vec![
            format!("help - {}", CliCommand::Help.description()),
            format!("history - {}", CliCommand::History.description()),
            format!("clear - {}", CliCommand::Clear.description()),
            format!("config - {}", CliCommand::Config.description()),
            format!("templates - {}", CliCommand::Templates.description()),
            format!("export - {}", CliCommand::Export.description()),
            format!("save - {}", CliCommand::Save.description()),
            format!("load - {}", CliCommand::Load.description()),
            format!("theme - {}", CliCommand::Theme.description()),
            format!("keybinds - {}", CliCommand::KeyBinds.description()),
            format!("quit - {}", CliCommand::Quit.description()),
        ];
        
        terminal::disable_raw_mode()?;
        
        let selection = Select::new("Select command:", commands)
            .with_page_size(10)
            .with_help_message("Use arrow keys to navigate, Enter to select, Esc to cancel")
            .prompt();
            
        terminal::enable_raw_mode()?;
        
        match selection {
            Ok(cmd_str) => {
                let cmd_name = cmd_str.split(' ').next().unwrap_or("");
                if let Some(command) = CliCommand::from_str(cmd_name) {
                    self.execute_command(command).await?;
                }
            }
            Err(InquireError::OperationCanceled) => {},
            Err(e) => {
                self.show_error(&format!("Command menu error: {}", e))?;
            }
        }
        
        Ok(())
    }
    
    async fn show_file_browser(&mut self) -> io::Result<()> {
        let mut current_path = self.current_directory.clone();
        
        loop {
            let entries = self.read_directory(&current_path)?;
            let mut display_entries = vec![".. (parent directory)".to_string()];
            
            for entry in &entries {
                let icon = if entry.is_directory { "=�" } else { "=�" };
                let size_info = if let Some(size) = entry.size {
                    format!(" ({})", format_file_size(size))
                } else {
                    String::new()
                };
                
                display_entries.push(format!("{} {}{}", icon, entry.name, size_info));
            }
            
            terminal::disable_raw_mode()?;
            
            let selection = Select::new(
                &format!("Browse: {} - Select file or directory:", current_path.display()),
                display_entries
            )
            .with_page_size(15)
            .with_help_message("Use arrow keys to navigate, Enter to select, Esc to cancel")
            .prompt();
            
            terminal::enable_raw_mode()?;
            
            match selection {
                Ok(selected) => {
                    if selected.starts_with(".. (parent") {
                        if let Some(parent) = current_path.parent() {
                            current_path = parent.to_path_buf();
                        }
                    } else {
                        let entry_name = selected.split(' ').skip(1).next().unwrap_or("");
                        if let Some(entry) = entries.iter().find(|e| e.name == entry_name) {
                            if entry.is_directory {
                                current_path = entry.path.clone();
                            } else {
                                self.editor.insert_char('@');
                                for ch in entry.path.to_string_lossy().chars() {
                                    self.editor.insert_char(ch);
                                }
                                self.editor.insert_char(' ');
                                break;
                            }
                        }
                    }
                }
                Err(InquireError::OperationCanceled) => break,
                Err(e) => {
                    self.show_error(&format!("File browser error: {}", e))?;
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    fn read_directory(&self, path: &Path) -> io::Result<Vec<FileEntry>> {
        let mut entries = Vec::new();
        
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_directory = metadata.is_dir();
            let size = if is_directory { None } else { Some(metadata.len()) };
            
            entries.push(FileEntry {
                name,
                path: entry.path(),
                is_directory,
                size,
            });
        }
        
        entries.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        
        Ok(entries)
    }
    
    async fn show_history_search(&mut self) -> io::Result<()> {
        let history_items = self.history.get_all();
        if history_items.is_empty() {
            self.show_info("No command history available")?;
            return Ok(());
        }
        
        terminal::disable_raw_mode()?;
        
        let selection = Select::new("Command History - Select to insert:", history_items)
            .with_page_size(10)
            .with_help_message("Use arrow keys to navigate, Enter to select, Esc to cancel")
            .prompt();
            
        terminal::enable_raw_mode()?;
        
        match selection {
            Ok(selected_command) => {
                let (width, _) = terminal::size()?;
                self.editor = TextEditor::from_text(&selected_command, width as usize);
            }
            Err(InquireError::OperationCanceled) => {}
            Err(e) => {
                self.show_error(&format!("History search error: {}", e))?;
            }
        }
        
        Ok(())
    }
    
    async fn execute_command(&mut self, command: CliCommand) -> io::Result<()> {
        match command {
            CliCommand::Help => self.show_help()?,
            CliCommand::History => self.show_history_details()?,
            CliCommand::Clear => {
                self.terminal.clear()?;
                let (width, _) = terminal::size()?;
                self.editor = TextEditor::new(width as usize);
            }
            CliCommand::Quit => self.should_exit = true,
            CliCommand::Config => self.show_config()?,
            CliCommand::Theme => self.show_theme_selector().await?,
            _ => self.show_info(&format!("Command '{:?}' not fully implemented yet", command))?,
        }
        Ok(())
    }
    
    fn show_help(&mut self) -> io::Result<()> {
        let help_text = vec![
            "<� KAI CLI Prompter - Help".to_string(),
            "".to_string(),
            "SPECIAL KEYS:".to_string(),
            "  /          - Open command menu".to_string(),
            "  @          - Open file browser".to_string(),
            "".to_string(),
            "KEYBOARD SHORTCUTS:".to_string(),
            "  Ctrl+A     - Move to beginning of line".to_string(),
            "  Ctrl+E     - Move to end of line".to_string(),
            "  Ctrl+D     - Delete character forward".to_string(),
            "  Ctrl+H     - Delete character backward".to_string(),
            "  Ctrl+K     - Delete to end of line".to_string(),
            "  Ctrl+U     - Delete entire line".to_string(),
            "  Ctrl+W     - Delete word backward".to_string(),
            "  Ctrl+L     - Clear screen".to_string(),
            "  Ctrl+R     - Search command history".to_string(),
            "  Ctrl+C     - Exit application".to_string(),
            "".to_string(),
            "NAVIGATION:".to_string(),
            "  ����       - Move cursor".to_string(),
            "  Home/End   - Line start/end".to_string(),
            "  Enter      - New line or submit".to_string(),
            "  Esc        - Clear current input".to_string(),
        ];
        
        self.show_message_frame("Help", &help_text)?;
        Ok(())
    }
    
    fn show_config(&mut self) -> io::Result<()> {
        let config_text = vec![
            "�  Configuration".to_string(),
            "".to_string(),
            format!("Frame Color: {}", self.config.frame_color),
            format!("Text Color: {}", self.config.text_color),
            format!("Command Prefix: {}", self.config.command_prefix),
            format!("File Browser Prefix: {}", self.config.file_browser_prefix),
            format!("Auto Save History: {}", self.config.auto_save_history),
            format!("Max History Size: {}", self.config.max_history_size),
            format!("Theme: {}", self.config.theme_name),
            "".to_string(),
            "Press any key to continue...".to_string(),
        ];
        
        self.show_message_frame("Configuration", &config_text)?;
        Ok(())
    }
    
    async fn show_theme_selector(&mut self) -> io::Result<()> {
        let themes = vec![
            "default - Blue frame, white text".to_string(),
            "dark - Black frame, green text".to_string(),
            "light - Gray frame, black text".to_string(),
            "ocean - Cyan frame, white text".to_string(),
            "sunset - Magenta frame, yellow text".to_string(),
        ];
        
        terminal::disable_raw_mode()?;
        
        let selection = Select::new("Select Theme:", themes)
            .with_page_size(5)
            .with_help_message("Use arrow keys to navigate, Enter to select, Esc to cancel")
            .prompt();
            
        terminal::enable_raw_mode()?;
        
        match selection {
            Ok(theme_str) => {
                let theme_name = theme_str.split(' ').next().unwrap_or("default");
                self.apply_theme(theme_name);
                self.show_info(&format!("Applied theme: {}", theme_name))?;
            }
            Err(InquireError::OperationCanceled) => {}
            Err(e) => {
                self.show_error(&format!("Theme selection error: {}", e))?;
            }
        }
        
        Ok(())
    }
    
    fn apply_theme(&mut self, theme_name: &str) {
        match theme_name {
            "dark" => {
                self.config.frame_color = "Black".to_string();
                self.config.text_color = "Green".to_string();
            }
            "light" => {
                self.config.frame_color = "Gray".to_string();
                self.config.text_color = "Black".to_string();
            }
            "ocean" => {
                self.config.frame_color = "Cyan".to_string();
                self.config.text_color = "White".to_string();
            }
            "sunset" => {
                self.config.frame_color = "Magenta".to_string();
                self.config.text_color = "Yellow".to_string();
            }
            _ => {
                self.config.frame_color = "DarkBlue".to_string();
                self.config.text_color = "White".to_string();
            }
        }
        self.config.theme_name = theme_name.to_string();
    }
    
    fn show_history_details(&mut self) -> io::Result<()> {
        let history_items = self.history.get_all();
        let mut history_text = vec![
            "=� Command History".to_string(),
            format!("Total commands: {}", history_items.len()),
            "".to_string(),
        ];
        
        for (i, cmd) in history_items.iter().rev().take(20).enumerate() {
            history_text.push(format!("{:2}. {}", i + 1, cmd));
        }
        
        if history_items.len() > 20 {
            history_text.push(format!("... and {} more", history_items.len() - 20));
        }
        
        self.show_message_frame("History", &history_text)?;
        Ok(())
    }
    
    fn show_message_frame(&mut self, title: &str, lines: &[String]) -> io::Result<()> {
        let frame_color = self.get_frame_color();
        self.terminal.draw(|f| {
            let area = f.size();
            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(Style::default().fg(frame_color));
                
            let text: Vec<Line> = lines.iter()
                .map(|line| Line::from(Span::styled(line.clone(), Style::default().fg(RatatuiColor::White))))
                .collect();
                
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: true });
                
            f.render_widget(paragraph, area);
        })?;
        
        loop {
            if let Event::Key(_) = event::read()? {
                break;
            }
        }
        
        Ok(())
    }
    
    fn show_error(&mut self, message: &str) -> io::Result<()> {
        self.show_message_frame("Error", &[format!("L {}", message), "".to_string(), "Press any key to continue...".to_string()])?;
        Ok(())
    }
    
    fn show_info(&mut self, message: &str) -> io::Result<()> {
        self.show_message_frame("Info", &[format!("9  {}", message), "".to_string(), "Press any key to continue...".to_string()])?;
        Ok(())
    }
    
    async fn process_input(&mut self, input: &str) -> io::Result<()> {
        if input.trim().is_empty() {
            return Ok(());
        }
        
        self.show_info(&format!("Processed input: {}", input.trim()))?;
        Ok(())
    }
    
    fn render_frame(&mut self) -> io::Result<()> {
        let editor_text = self.editor.get_text();
        let frame_color = self.get_frame_color();
        
        self.terminal.draw(|f| {
            let area = f.size();
            
            let content_width = (area.width as usize).min(120).max(60);
            let wrapped_lines = wrap_text(&editor_text, content_width - 4);
            let content_height = wrapped_lines.len().max(3) + 2;
            
            let frame_rect = Rect {
                x: (area.width.saturating_sub(content_width as u16)) / 2,
                y: (area.height.saturating_sub(content_height as u16)) / 2,
                width: content_width as u16,
                height: content_height as u16,
            };
            
            let block = Block::default()
                .title("KAI 🦀")
                .borders(Borders::ALL)
                .style(Style::default().fg(frame_color));
            
            let _inner_area = block.inner(frame_rect);
            
            let text: Vec<Line> = wrapped_lines.iter()
                .map(|line| Line::from(Span::styled(line.clone(), Style::default().fg(RatatuiColor::White))))
                .collect();
            
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: false });
            
            f.render_widget(paragraph, frame_rect);
        })?;
        
        Ok(())
    }
    
    fn calculate_cursor_position(&self, area: Rect, wrapped_lines: &[String]) -> (u16, u16) {
        let _line_idx = 0;
        let mut char_count = 0;
        
        for (y, current_line) in self.editor.lines.iter().enumerate() {
            if y == self.editor.cursor_y {
                let line_prefix_count = char_count;
                let target_char = line_prefix_count + self.editor.cursor_x;
                
                for (wrap_idx, wrapped_line) in wrapped_lines.iter().enumerate() {
                    let line_start = char_count;
                    let line_end = char_count + wrapped_line.len();
                    
                    if target_char >= line_start && target_char <= line_end {
                        let x_offset = target_char - line_start;
                        return (
                            area.x + x_offset as u16,
                            area.y + wrap_idx as u16
                        );
                    }
                    
                    char_count += wrapped_line.len();
                    if wrap_idx < wrapped_lines.len() - 1 {
                        char_count += 1; // for line break
                    }
                }
                break;
            } else {
                char_count += current_line.len() + 1; // +1 for line break
            }
        }
        
        (area.x, area.y)
    }
    
    fn get_frame_color(&self) -> RatatuiColor {
        match self.config.frame_color.as_str() {
            "Black" => RatatuiColor::Black,
            "DarkBlue" => RatatuiColor::Blue,
            "Blue" => RatatuiColor::Blue,
            "Cyan" => RatatuiColor::Cyan,
            "Gray" => RatatuiColor::Gray,
            "Magenta" => RatatuiColor::Magenta,
            "Green" => RatatuiColor::Green,
            "Red" => RatatuiColor::Red,
            "Yellow" => RatatuiColor::Yellow,
            _ => RatatuiColor::Blue,
        }
    }
    
    fn get_text_color(&self) -> RatatuiColor {
        match self.config.text_color.as_str() {
            "Black" => RatatuiColor::Black,
            "White" => RatatuiColor::White,
            "Green" => RatatuiColor::Green,
            "Yellow" => RatatuiColor::Yellow,
            "Red" => RatatuiColor::Red,
            "Blue" => RatatuiColor::Blue,
            "Cyan" => RatatuiColor::Cyan,
            "Magenta" => RatatuiColor::Magenta,
            _ => RatatuiColor::White,
        }
    }
}

impl Drop for CliPrompter {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Utility function to wrap text to specified width
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    
    let mut wrapped = Vec::new();
    
    for line in text.lines() {
        if line.len() <= max_width {
            wrapped.push(line.to_string());
        } else {
            let mut start = 0;
            while start < line.len() {
                let end = (start + max_width).min(line.len());
                let substring = &line[start..end];
                
                if end < line.len() {
                    if let Some(space_idx) = substring.rfind(char::is_whitespace) {
                        wrapped.push(line[start..start + space_idx].to_string());
                        start += space_idx + 1;
                    } else {
                        wrapped.push(substring.to_string());
                        start = end;
                    }
                } else {
                    wrapped.push(substring.to_string());
                    break;
                }
            }
        }
    }
    
    wrapped
}

/// Format file size in human readable format
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{:.0} {}", size, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wrap_text() {
        let text = "This is a long line that should be wrapped at word boundaries";
        let wrapped = wrap_text(text, 20);
        assert!(wrapped.len() > 1);
        assert!(wrapped.iter().all(|line| line.len() <= 20));
    }
    
    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
    }
    
    #[test]
    fn test_command_history() {
        let mut history = CommandHistory::new(3);
        history.add_command("cmd1".to_string());
        history.add_command("cmd2".to_string());
        history.add_command("cmd3".to_string());
        history.add_command("cmd4".to_string());
        
        assert_eq!(history.commands.len(), 3);
        assert_eq!(history.commands.back().unwrap(), "cmd4");
    }
    
    #[test]
    fn test_text_editor() {
        let mut editor = TextEditor::new(80);
        editor.insert_char('H');
        editor.insert_char('i');
        assert_eq!(editor.get_text(), "Hi");
        
        editor.delete_char();
        assert_eq!(editor.get_text(), "H");
    }
}