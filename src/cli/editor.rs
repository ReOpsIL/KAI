//! Text Editor Module
//!
//! Multi-line text editor with advanced cursor management,
//! text manipulation, and navigation capabilities.

/// Cursor movement directions
#[derive(Debug, Clone, Copy)]
pub enum CursorDirection {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
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
    /// Create a new text editor with specified maximum width
    pub fn new(max_width: usize) -> Self {
        Self {
            lines: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
            scroll_offset: 0,
            max_width,
        }
    }
    
    /// Create a text editor from existing text
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
    
    /// Get the complete text content
    pub fn get_text(&self) -> String {
        self.lines.join("\n")
    }
    
    /// Insert a character at the current cursor position
    pub fn insert_char(&mut self, ch: char) {
        let current_line = &mut self.lines[self.cursor_y];
        current_line.insert(self.cursor_x, ch);
        self.cursor_x += 1;
    }
    
    /// Delete character before cursor (backspace)
    pub fn delete_char(&mut self) {
        let current_line = &mut self.lines[self.cursor_y];
        if self.cursor_x > 0 {
            current_line.remove(self.cursor_x - 1);
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            // Join with previous line
            let line = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.lines[self.cursor_y].len();
            self.lines[self.cursor_y].push_str(&line);
        }
    }
    
    /// Delete character at cursor (delete key)
    pub fn delete_char_forward(&mut self) {
        let current_line = &mut self.lines[self.cursor_y];
        if self.cursor_x < current_line.len() {
            current_line.remove(self.cursor_x);
        } else if self.cursor_y < self.lines.len() - 1 {
            // Join with next line
            let next_line = self.lines.remove(self.cursor_y + 1);
            self.lines[self.cursor_y].push_str(&next_line);
        }
    }
    
    /// Handle Enter key - create new line
    pub fn handle_enter(&mut self) {
        let line_content = self.lines[self.cursor_y].clone();
        let (left, right) = line_content.split_at(self.cursor_x);
        
        self.lines[self.cursor_y] = left.to_string();
        self.lines.insert(self.cursor_y + 1, right.to_string());
        
        self.cursor_y += 1;
        self.cursor_x = 0;
    }
    
    /// Move cursor in specified direction
    pub fn move_cursor(&mut self, direction: CursorDirection) {
        match direction {
            CursorDirection::Left => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    // Move to end of previous line
                    self.cursor_y -= 1;
                    self.cursor_x = self.lines[self.cursor_y].len();
                }
            }
            CursorDirection::Right => {
                let current_line_len = self.lines[self.cursor_y].len();
                if self.cursor_x < current_line_len {
                    self.cursor_x += 1;
                } else if self.cursor_y < self.lines.len() - 1 {
                    // Move to start of next line
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
    
    /// Clear the current line
    pub fn delete_line(&mut self) {
        self.lines[self.cursor_y].clear();
        self.cursor_x = 0;
    }
    
    /// Delete from cursor to end of line
    pub fn delete_to_end_of_line(&mut self) {
        let current_line = &mut self.lines[self.cursor_y];
        current_line.truncate(self.cursor_x);
    }
    
    /// Delete word backward (Ctrl+W functionality)
    pub fn delete_word_backward(&mut self) {
        let current_line = &mut self.lines[self.cursor_y];
        if self.cursor_x == 0 {
            return;
        }
        
        let mut new_x = self.cursor_x;
        let chars: Vec<char> = current_line.chars().collect();
        
        // Skip whitespace
        while new_x > 0 && chars[new_x - 1].is_whitespace() {
            new_x -= 1;
        }
        
        // Delete word characters
        while new_x > 0 && !chars[new_x - 1].is_whitespace() {
            new_x -= 1;
        }
        
        current_line.drain(new_x..self.cursor_x);
        self.cursor_x = new_x;
    }
    
    /// Get wrapped lines for display
    pub fn get_wrapped_lines(&self) -> Vec<String> {
        let mut wrapped = Vec::new();
        for line in &self.lines {
            wrapped.extend(wrap_text(line, self.max_width - 4));
        }
        wrapped
    }
    
    /// Get current cursor position
    pub fn get_cursor_position(&self) -> (usize, usize) {
        (self.cursor_x, self.cursor_y)
    }
    
    /// Get total number of lines
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
    
    /// Get current line content
    pub fn current_line(&self) -> &str {
        &self.lines[self.cursor_y]
    }
    
    /// Check if editor is empty
    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }
    
    /// Set maximum width and adjust layout
    pub fn set_max_width(&mut self, max_width: usize) {
        self.max_width = max_width;
    }
    
    /// Insert text at current cursor position
    pub fn insert_text(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\n' {
                self.handle_enter();
            } else {
                self.insert_char(ch);
            }
        }
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
                    // Try to break at word boundary
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_text_editor_basic_operations() {
        let mut editor = TextEditor::new(80);
        
        // Test insertion
        editor.insert_char('H');
        editor.insert_char('i');
        assert_eq!(editor.get_text(), "Hi");
        
        // Test deletion
        editor.delete_char();
        assert_eq!(editor.get_text(), "H");
        
        // Test cursor movement
        editor.move_cursor(CursorDirection::Home);
        assert_eq!(editor.get_cursor_position(), (0, 0));
    }
    
    #[test]
    fn test_text_editor_multiline() {
        let mut editor = TextEditor::new(80);
        
        editor.insert_text("First line\nSecond line");
        assert_eq!(editor.line_count(), 2);
        assert_eq!(editor.get_text(), "First line\nSecond line");
    }
    
    #[test]
    fn test_word_deletion() {
        let mut editor = TextEditor::new(80);
        editor.insert_text("hello world");
        editor.delete_word_backward();
        assert_eq!(editor.get_text(), "hello ");
    }
    
    #[test]
    fn test_line_operations() {
        let mut editor = TextEditor::new(80);
        editor.insert_text("test line");
        editor.delete_line();
        assert_eq!(editor.get_text(), "");
        
        editor.insert_text("another test");
        editor.move_cursor(CursorDirection::Home);
        editor.move_cursor(CursorDirection::Right);
        editor.move_cursor(CursorDirection::Right);
        editor.delete_to_end_of_line();
        assert_eq!(editor.get_text(), "an");
    }
    
    #[test]
    fn test_wrap_text() {
        let text = "This is a long line that should be wrapped at word boundaries";
        let wrapped = wrap_text(text, 20);
        assert!(wrapped.len() > 1);
        assert!(wrapped.iter().all(|line| line.len() <= 20));
    }
}