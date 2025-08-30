//! Prompt Management Module
//!
//! Centralized prompt templates and management for consistent LLM interactions.

pub mod prompts;

// Re-export main types
pub use prompts::PromptManager;