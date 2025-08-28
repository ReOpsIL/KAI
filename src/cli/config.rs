//! CLI Configuration and Theme Management
//!
//! This module handles configuration settings, theme management,
//! and color schemes for the CLI prompter.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use ratatui::style::Color as RatatuiColor;

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

impl CliConfig {
    /// Create a new configuration with custom settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Apply a theme by name
    pub fn apply_theme(&mut self, theme_name: &str) {
        match theme_name {
            "dark" => {
                self.frame_color = "Black".to_string();
                self.text_color = "Green".to_string();
            }
            "light" => {
                self.frame_color = "Gray".to_string();
                self.text_color = "Black".to_string();
            }
            "ocean" => {
                self.frame_color = "Cyan".to_string();
                self.text_color = "White".to_string();
            }
            "sunset" => {
                self.frame_color = "Magenta".to_string();
                self.text_color = "Yellow".to_string();
            }
            _ => {
                self.frame_color = "DarkBlue".to_string();
                self.text_color = "White".to_string();
            }
        }
        self.theme_name = theme_name.to_string();
    }
    
    /// Get frame color as RatatuiColor
    pub fn get_frame_color(&self) -> RatatuiColor {
        match self.frame_color.as_str() {
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
    
    /// Get text color as RatatuiColor
    pub fn get_text_color(&self) -> RatatuiColor {
        match self.text_color.as_str() {
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
    
    /// Get available themes
    pub fn get_available_themes() -> Vec<String> {
        vec![
            "default - Blue frame, white text".to_string(),
            "dark - Black frame, green text".to_string(),
            "light - Gray frame, black text".to_string(),
            "ocean - Cyan frame, white text".to_string(),
            "sunset - Magenta frame, yellow text".to_string(),
        ]
    }
    
    /// Get configuration summary for display
    pub fn get_summary(&self) -> Vec<String> {
        vec![
            "⚙️  Configuration".to_string(),
            "".to_string(),
            format!("Frame Color: {}", self.frame_color),
            format!("Text Color: {}", self.text_color),
            format!("Command Prefix: {}", self.command_prefix),
            format!("File Browser Prefix: {}", self.file_browser_prefix),
            format!("Auto Save History: {}", self.auto_save_history),
            format!("Max History Size: {}", self.max_history_size),
            format!("Theme: {}", self.theme_name),
            "".to_string(),
            "Press any key to continue...".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = CliConfig::default();
        assert_eq!(config.command_prefix, '/');
        assert_eq!(config.file_browser_prefix, '@');
        assert_eq!(config.theme_name, "default");
    }
    
    #[test]
    fn test_theme_application() {
        let mut config = CliConfig::new();
        config.apply_theme("dark");
        assert_eq!(config.theme_name, "dark");
        assert_eq!(config.frame_color, "Black");
        assert_eq!(config.text_color, "Green");
    }
    
    #[test]
    fn test_color_conversion() {
        let config = CliConfig::default();
        let frame_color = config.get_frame_color();
        let text_color = config.get_text_color();
        
        // Should not panic and return valid colors
        assert!(matches!(frame_color, RatatuiColor::Blue));
        assert!(matches!(text_color, RatatuiColor::White));
    }
}