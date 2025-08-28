//! CLI Utilities Module
//!
//! Common utilities and helper functions used throughout the CLI prompter.
//! Includes text processing, formatting, and display utilities.

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::io;

/// Utility function to wrap text to specified width
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
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

/// Format file size in human readable format
pub fn format_file_size(size: u64) -> String {
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

/// Wait for any key press from the user
pub fn wait_for_key() -> io::Result<KeyEvent> {
    loop {
        if let Event::Key(key) = event::read()? {
            return Ok(key);
        }
    }
}

/// Wait for Enter key specifically
pub fn wait_for_enter() -> io::Result<()> {
    loop {
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Enter {
                return Ok(());
            }
        }
    }
}

/// Create a centered text block for display
pub fn center_text(text: &str, width: usize) -> String {
    if text.len() >= width {
        return text.to_string();
    }
    
    let padding = (width - text.len()) / 2;
    format!("{}{}", " ".repeat(padding), text)
}

/// Create a box around text with specified characters
pub fn create_text_box(lines: &[String], border_chars: Option<&str>) -> Vec<String> {
    if lines.is_empty() {
        return vec![];
    }
    
    let chars = border_chars.unwrap_or("┌─┐│└─┘");
    let chars: Vec<char> = chars.chars().collect();
    if chars.len() < 7 {
        return lines.to_vec(); // Return original if invalid border chars
    }
    
    let max_width = lines.iter().map(|line| line.len()).max().unwrap_or(0);
    let mut result = Vec::new();
    
    // Top border
    result.push(format!("{}{}{}", 
        chars[0], 
        chars[1].to_string().repeat(max_width + 2), 
        chars[2]
    ));
    
    // Content lines
    for line in lines {
        result.push(format!("{} {:width$} {}", 
            chars[3], 
            line, 
            chars[3],
            width = max_width
        ));
    }
    
    // Bottom border
    result.push(format!("{}{}{}", 
        chars[4], 
        chars[5].to_string().repeat(max_width + 2), 
        chars[6]
    ));
    
    result
}

/// Truncate text to fit within specified width
pub fn truncate_text(text: &str, max_width: usize, ellipsis: bool) -> String {
    if text.len() <= max_width {
        return text.to_string();
    }
    
    if ellipsis && max_width > 3 {
        format!("{}...", &text[..max_width - 3])
    } else {
        text[..max_width].to_string()
    }
}

/// Pad text to specified width
pub fn pad_text(text: &str, width: usize, align: TextAlign) -> String {
    if text.len() >= width {
        return text.to_string();
    }
    
    let padding = width - text.len();
    match align {
        TextAlign::Left => format!("{}{}", text, " ".repeat(padding)),
        TextAlign::Right => format!("{}{}", " ".repeat(padding), text),
        TextAlign::Center => {
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
    }
}

/// Text alignment options
pub enum TextAlign {
    Left,
    Right,
    Center,
}

/// Create a progress bar string
pub fn create_progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return "█".repeat(width);
    }
    
    let progress = (current as f64 / total as f64).min(1.0);
    let filled = (progress * width as f64) as usize;
    let empty = width - filled;
    
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Format duration in human readable format
pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}m {}s", minutes, secs)
    } else {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
}

/// Highlight text with ANSI color codes
pub fn highlight_text(text: &str, color: HighlightColor) -> String {
    let color_code = match color {
        HighlightColor::Red => "\x1b[31m",
        HighlightColor::Green => "\x1b[32m",
        HighlightColor::Yellow => "\x1b[33m",
        HighlightColor::Blue => "\x1b[34m",
        HighlightColor::Magenta => "\x1b[35m",
        HighlightColor::Cyan => "\x1b[36m",
        HighlightColor::White => "\x1b[37m",
        HighlightColor::Bold => "\x1b[1m",
        HighlightColor::Underline => "\x1b[4m",
    };
    
    format!("{}{}\x1b[0m", color_code, text)
}

/// Color options for text highlighting
pub enum HighlightColor {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Bold,
    Underline,
}

/// Create a table from data with headers
pub fn create_table(headers: &[String], rows: &[Vec<String>]) -> Vec<String> {
    if headers.is_empty() || rows.is_empty() {
        return vec![];
    }
    
    // Calculate column widths
    let mut col_widths = headers.iter().map(|h| h.len()).collect::<Vec<_>>();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }
    }
    
    let mut result = Vec::new();
    
    // Header
    let header_row = headers.iter()
        .enumerate()
        .map(|(i, h)| pad_text(h, col_widths[i], TextAlign::Left))
        .collect::<Vec<_>>()
        .join(" │ ");
    result.push(format!("│ {} │", header_row));
    
    // Separator
    let separator = col_widths.iter()
        .map(|&w| "─".repeat(w))
        .collect::<Vec<_>>()
        .join("─┼─");
    result.push(format!("├─{}─┤", separator));
    
    // Data rows
    for row in rows {
        let data_row = row.iter()
            .enumerate()
            .map(|(i, cell)| {
                let width = col_widths.get(i).copied().unwrap_or(0);
                pad_text(cell, width, TextAlign::Left)
            })
            .collect::<Vec<_>>()
            .join(" │ ");
        result.push(format!("│ {} │", data_row));
    }
    
    result
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
    fn test_center_text() {
        assert_eq!(center_text("hello", 10), "  hello");
        assert_eq!(center_text("hello", 11), "   hello");
    }
    
    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello world", 5, false), "hello");
        assert_eq!(truncate_text("hello world", 8, true), "hello...");
    }
    
    #[test]
    fn test_pad_text() {
        assert_eq!(pad_text("hello", 10, TextAlign::Left), "hello     ");
        assert_eq!(pad_text("hello", 10, TextAlign::Right), "     hello");
        assert_eq!(pad_text("hello", 10, TextAlign::Center), "  hello   ");
    }
    
    #[test]
    fn test_create_progress_bar() {
        assert_eq!(create_progress_bar(5, 10, 10), "█████░░░░░");
        assert_eq!(create_progress_bar(10, 10, 10), "██████████");
    }
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(125), "2m 5s");
        assert_eq!(format_duration(3665), "1h 1m");
    }
    
    #[test]
    fn test_create_table() {
        let headers = vec!["Name".to_string(), "Age".to_string()];
        let rows = vec![
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];
        
        let table = create_table(&headers, &rows);
        assert!(!table.is_empty());
        assert!(table[0].contains("Name"));
        assert!(table[0].contains("Age"));
    }
}