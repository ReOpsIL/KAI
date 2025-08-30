//! CLI Configuration and Theme Management
//!
//! This module handles configuration settings, theme management,
//! color schemes, and OpenRouter model configuration for the CLI prompter.

use crossterm::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenRouter model configuration with tiered model selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub simple_model: String,
    pub midrange_model: String,
    pub advanced_model: String,
    pub critical_model: String,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            simple_model: "openai/gpt-4o-mini".to_string(),
            midrange_model: "openai/gpt-4o-mini".to_string(),
            advanced_model: "openai/gpt-4o-mini".to_string(),
            critical_model: "openai/gpt-4o-mini".to_string(),
        }
    }
}

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
    pub openrouter: OpenRouterConfig,
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
            openrouter: OpenRouterConfig::default(),
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

    /// Get frame color as Color
    pub fn get_frame_color(&self) -> Color {
        match self.frame_color.as_str() {
            "Black" => Color::Black,
            "DarkBlue" => Color::DarkBlue,
            "Blue" => Color::Blue,
            "Cyan" => Color::Cyan,
            "Gray" => Color::Grey,
            "Magenta" => Color::Magenta,
            "Green" => Color::Green,
            "Red" => Color::Red,
            "Yellow" => Color::Yellow,
            _ => Color::Blue,
        }
    }

    /// Get text color as Color
    pub fn get_text_color(&self) -> Color {
        match self.text_color.as_str() {
            "Black" => Color::Black,
            "White" => Color::White,
            "Green" => Color::Green,
            "Yellow" => Color::Yellow,
            "Red" => Color::Red,
            "Blue" => Color::Blue,
            "Cyan" => Color::Cyan,
            "Magenta" => Color::Magenta,
            _ => Color::White,
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

    /// Get available model tiers with their descriptions
    pub fn get_available_model_tiers(&self) -> Vec<(String, String)> {
        vec![
            (
                "Tier 1 - Simple".to_string(),
                self.openrouter.simple_model.clone(),
            ),
            (
                "Tier 2 - MidRange".to_string(),
                self.openrouter.midrange_model.clone(),
            ),
            (
                "Tier 3 - Advanced".to_string(),
                self.openrouter.advanced_model.clone(),
            ),
            (
                "Tier 4 - Critical".to_string(),
                self.openrouter.critical_model.clone(),
            ),
        ]
    }

    /// Get model by tier (1-4)
    pub fn get_model_by_tier(&self, tier: u8) -> Option<&str> {
        match tier {
            1 => Some(&self.openrouter.simple_model),
            2 => Some(&self.openrouter.midrange_model),
            3 => Some(&self.openrouter.advanced_model),
            4 => Some(&self.openrouter.critical_model),
            _ => None,
        }
    }

    /// Set model for specific tier
    pub fn set_model_for_tier(&mut self, tier: u8, model: String) -> bool {
        match tier {
            1 => {
                self.openrouter.simple_model = model;
                true
            }
            2 => {
                self.openrouter.midrange_model = model;
                true
            }
            3 => {
                self.openrouter.advanced_model = model;
                true
            }
            4 => {
                self.openrouter.critical_model = model;
                true
            }
            _ => false,
        }
    }

    /// Get configuration summary for display
    pub fn get_summary(&self) -> Vec<String> {
        vec![
            "⚙️  Configuration".to_string(),
            "".to_string(),
            "🎨 Display Settings".to_string(),
            format!("  Frame Color: {}", self.frame_color),
            format!("  Text Color: {}", self.text_color),
            format!("  Theme: {}", self.theme_name),
            "".to_string(),
            "⌨️  Interface Settings".to_string(),
            format!("  Command Prefix: {}", self.command_prefix),
            format!("  File Browser Prefix: {}", self.file_browser_prefix),
            format!("  Auto Save History: {}", self.auto_save_history),
            format!("  Max History Size: {}", self.max_history_size),
            "".to_string(),
            "🤖 OpenRouter Models".to_string(),
            format!("  Tier 1 (Simple): {}", self.openrouter.simple_model),
            format!("  Tier 2 (MidRange): {}", self.openrouter.midrange_model),
            format!("  Tier 3 (Advanced): {}", self.openrouter.advanced_model),
            format!("  Tier 4 (Critical): {}", self.openrouter.critical_model),
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
        assert_eq!(config.openrouter.simple_model, "openai/gpt-4o-mini");
        assert_eq!(
            config.openrouter.advanced_model,
            "anthropic/claude-3.5-sonnet"
        );
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
        assert!(matches!(frame_color, Color::Blue));
        assert!(matches!(text_color, Color::White));
    }

    #[test]
    fn test_openrouter_model_tiers() {
        let config = CliConfig::default();

        // Test getting models by tier
        assert_eq!(config.get_model_by_tier(1), Some("openai/gpt-4o-mini"));
        assert_eq!(config.get_model_by_tier(2), Some("openai/gpt-4o-mini"));
        assert_eq!(config.get_model_by_tier(3), Some("openai/gpt-4o-mini"));
        assert_eq!(config.get_model_by_tier(4), Some("openai/gpt-4o-mini"));
        assert_eq!(config.get_model_by_tier(5), None);
    }

    #[test]
    fn test_model_tier_modification() {
        let mut config = CliConfig::default();

        // Test setting models for different tiers
        assert!(config.set_model_for_tier(1, "custom/model-1".to_string()));
        assert!(config.set_model_for_tier(3, "custom/model-3".to_string()));
        assert!(!config.set_model_for_tier(5, "invalid".to_string()));

        assert_eq!(config.get_model_by_tier(1), Some("openai/gpt-4o-mini"));
        assert_eq!(config.get_model_by_tier(2), Some("openai/gpt-4o-mini")); // unchanged
        assert_eq!(config.get_model_by_tier(3), Some("openai/gpt-4o-mini"));
    }

    #[test]
    fn test_available_model_tiers() {
        let config = CliConfig::default();
        let tiers = config.get_available_model_tiers();

        assert_eq!(tiers.len(), 4);
        assert_eq!(tiers[0].0, "Tier 1 - Simple");
        assert_eq!(tiers[0].1, "openai/gpt-4o-mini");
        assert_eq!(tiers[3].0, "Tier 4 - Critical");
        assert_eq!(tiers[3].1, "openai/gpt-4o-mini");
    }
}
