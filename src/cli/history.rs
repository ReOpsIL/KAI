//! Command History Module
//!
//! Efficient command history management with search and navigation capabilities.
//! Uses a ring buffer for memory efficiency and provides history traversal.

use std::collections::VecDeque;

/// Command history manager with efficient storage
#[derive(Debug, Clone)]
pub struct CommandHistory {
    commands: VecDeque<String>,
    max_size: usize,
    current_index: Option<usize>,
    /// Temporary storage for the current typed line during navigation
    temp_current_line: Option<String>,
}

impl CommandHistory {
    /// Create a new command history with specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            commands: VecDeque::new(),
            max_size,
            current_index: None,
            temp_current_line: None,
        }
    }
    
    /// Add a command to the history
    /// Duplicates of the most recent command are ignored
    pub fn add_command(&mut self, cmd: String) {
        let cmd = cmd.trim().to_string();
        
        // Skip empty commands and duplicates of the most recent command
        if !cmd.is_empty() && !self.commands.back().map_or(false, |last| last == &cmd) {
            // Remove oldest command if at capacity
            if self.commands.len() >= self.max_size {
                self.commands.pop_front();
            }
            self.commands.push_back(cmd);
        }
        
        // Reset navigation index and clear temp storage when new command is added
        self.current_index = None;
        self.temp_current_line = None;
    }
    
    /// Get the previous command in history (up arrow functionality)
    pub fn previous(&mut self) -> Option<String> {
        if self.commands.is_empty() {
            return None;
        }
        
        match self.current_index {
            None => {
                // Start from the most recent command
                self.current_index = Some(self.commands.len() - 1);
                self.commands.back().cloned()
            }
            Some(idx) if idx > 0 => {
                // Move to previous command
                self.current_index = Some(idx - 1);
                self.commands.get(idx - 1).cloned()
            }
            Some(_) => {
                // Already at the oldest command
                self.commands.get(0).cloned()
            }
        }
    }
    
    /// Get the next command in history (down arrow functionality)
    /// Returns the stored temporary line when moving past the newest command
    pub fn next(&mut self) -> Option<String> {
        match self.current_index {
            None => None,
            Some(idx) if idx < self.commands.len() - 1 => {
                // Move to next command
                self.current_index = Some(idx + 1);
                self.commands.get(idx + 1).cloned()
            }
            Some(_) => {
                // Reset to no selection - return temporary current line if stored
                self.current_index = None;
                self.temp_current_line.clone() // Clone instead of take to preserve it
            }
        }
    }
    
    /// Get all commands as a vector (most recent first)
    pub fn get_all(&self) -> Vec<String> {
        self.commands.iter().rev().cloned().collect()
    }
    
    /// Get all commands in chronological order (oldest first)
    pub fn get_chronological(&self) -> Vec<String> {
        self.commands.iter().cloned().collect()
    }
    
    /// Search for commands containing the given pattern
    pub fn search(&self, pattern: &str) -> Vec<String> {
        let pattern = pattern.to_lowercase();
        self.commands
            .iter()
            .filter(|cmd| cmd.to_lowercase().contains(&pattern))
            .rev() // Most recent first
            .cloned()
            .collect()
    }
    
    /// Get the most recent command
    pub fn last_command(&self) -> Option<&String> {
        self.commands.back()
    }
    
    /// Get total number of commands in history
    pub fn len(&self) -> usize {
        self.commands.len()
    }
    
    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
    
    /// Clear all history
    pub fn clear(&mut self) {
        self.commands.clear();
        self.current_index = None;
        self.temp_current_line = None;
    }
    
    /// Get current navigation index
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }
    
    /// Reset navigation index
    pub fn reset_navigation(&mut self) {
        self.current_index = None;
        self.temp_current_line = None;
    }
    
    /// Store the current typed line temporarily for history navigation
    pub fn store_current_line(&mut self, line: String) {
        self.temp_current_line = Some(line);
    }
    
    /// Check if there's a stored current line
    pub fn has_stored_line(&self) -> bool {
        self.temp_current_line.is_some()
    }
    
    /// Clear the stored current line
    pub fn clear_stored_line(&mut self) {
        self.temp_current_line = None;
    }
    
    /// Check if currently at the stored line position (past all history)
    pub fn is_at_stored_line(&self) -> bool {
        self.current_index.is_none() && self.temp_current_line.is_some()
    }
    
    /// Get history summary for display
    pub fn get_summary(&self, limit: Option<usize>) -> Vec<String> {
        let mut summary = vec![
            "📜 Command History".to_string(),
            format!("Total commands: {}", self.len()),
            "".to_string(),
        ];
        
        let display_limit = limit.unwrap_or(20);
        let commands = self.get_all();
        
        for (i, cmd) in commands.iter().take(display_limit).enumerate() {
            summary.push(format!("{:2}. {}", i + 1, cmd));
        }
        
        if commands.len() > display_limit {
            summary.push(format!("... and {} more", commands.len() - display_limit));
        }
        
        if commands.is_empty() {
            summary.push("No commands in history yet.".to_string());
        }
        
        summary
    }
    
    /// Export history to a format suitable for persistence
    pub fn export(&self) -> Vec<String> {
        self.get_chronological()
    }
    
    /// Import history from exported format
    pub fn import(&mut self, commands: Vec<String>) {
        self.clear();
        for cmd in commands {
            self.add_command(cmd);
        }
    }
    
    /// Get statistics about the history
    pub fn get_stats(&self) -> HistoryStats {
        let commands = self.get_chronological();
        let total_chars: usize = commands.iter().map(|cmd| cmd.len()).sum();
        let avg_length = if commands.is_empty() { 0.0 } else { total_chars as f64 / commands.len() as f64 };
        
        let longest = commands.iter().max_by_key(|cmd| cmd.len()).cloned().unwrap_or_default();
        let shortest = commands.iter().min_by_key(|cmd| cmd.len()).cloned().unwrap_or_default();
        
        HistoryStats {
            total_commands: commands.len(),
            total_characters: total_chars,
            average_length: avg_length,
            longest_command: longest,
            shortest_command: shortest,
        }
    }
}

/// Statistics about command history
#[derive(Debug, Clone)]
pub struct HistoryStats {
    pub total_commands: usize,
    pub total_characters: usize,
    pub average_length: f64,
    pub longest_command: String,
    pub shortest_command: String,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_history_basic() {
        let mut history = CommandHistory::new(3);
        
        history.add_command("cmd1".to_string());
        history.add_command("cmd2".to_string());
        history.add_command("cmd3".to_string());
        history.add_command("cmd4".to_string());
        
        // Should only keep the last 3 commands
        assert_eq!(history.len(), 3);
        assert_eq!(history.last_command().unwrap(), "cmd4");
    }
    
    #[test]
    fn test_command_history_navigation() {
        let mut history = CommandHistory::new(5);
        
        history.add_command("first".to_string());
        history.add_command("second".to_string());
        history.add_command("third".to_string());
        
        // Test previous navigation
        assert_eq!(history.previous().unwrap(), "third");
        assert_eq!(history.previous().unwrap(), "second");
        assert_eq!(history.previous().unwrap(), "first");
        
        // Test next navigation
        assert_eq!(history.next().unwrap(), "second");
        assert_eq!(history.next().unwrap(), "third");
        assert!(history.next().is_none()); // Should reset to None
    }
    
    #[test]
    fn test_temp_current_line_storage() {
        let mut history = CommandHistory::new(5);
        
        history.add_command("first".to_string());
        history.add_command("second".to_string());
        
        // Store current typing
        history.store_current_line("currently typing...".to_string());
        assert!(history.has_stored_line());
        
        // Navigate through history
        assert_eq!(history.previous().unwrap(), "second");
        assert_eq!(history.previous().unwrap(), "first");
        
        // Navigate forward - should restore the stored current line
        assert_eq!(history.next().unwrap(), "second");
        assert_eq!(history.next().unwrap(), "currently typing...");
        
        // Should preserve the temp storage (cloned, not taken)
        assert!(history.has_stored_line());
        
        // Can navigate back to stored line again
        assert_eq!(history.previous().unwrap(), "second");
        assert_eq!(history.next().unwrap(), "currently typing...");
    }
    
    #[test]
    fn test_command_history_duplicates() {
        let mut history = CommandHistory::new(5);
        
        history.add_command("cmd1".to_string());
        history.add_command("cmd1".to_string()); // Duplicate
        history.add_command("cmd2".to_string());
        
        assert_eq!(history.len(), 2); // Should ignore duplicate
    }
    
    #[test]
    fn test_command_history_search() {
        let mut history = CommandHistory::new(10);
        
        history.add_command("git status".to_string());
        history.add_command("git commit".to_string());
        history.add_command("ls -la".to_string());
        history.add_command("git push".to_string());
        
        let git_commands = history.search("git");
        assert_eq!(git_commands.len(), 3);
        assert!(git_commands.contains(&"git push".to_string()));
    }
    
    #[test]
    fn test_command_history_empty() {
        let mut history = CommandHistory::new(5);
        
        assert!(history.is_empty());
        assert!(history.previous().is_none());
        assert!(history.next().is_none());
        
        history.add_command("   ".to_string()); // Only whitespace
        assert!(history.is_empty());
    }
    
    #[test]
    fn test_history_stats() {
        let mut history = CommandHistory::new(10);
        
        history.add_command("short".to_string());
        history.add_command("this is a longer command".to_string());
        
        let stats = history.get_stats();
        assert_eq!(stats.total_commands, 2);
        assert_eq!(stats.shortest_command, "short");
        assert_eq!(stats.longest_command, "this is a longer command");
    }
    
    #[test]
    fn test_history_import_export() {
        let mut history1 = CommandHistory::new(10);
        history1.add_command("cmd1".to_string());
        history1.add_command("cmd2".to_string());
        
        let exported = history1.export();
        
        let mut history2 = CommandHistory::new(10);
        history2.import(exported);
        
        assert_eq!(history1.len(), history2.len());
        assert_eq!(history1.last_command(), history2.last_command());
    }
}