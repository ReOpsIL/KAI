//! LLM Integration Module
//!
//! Provides integration with various LLM providers for AI-powered functionality.

pub mod openrouter;

// Re-export main types
pub use openrouter::{OpenRouterClient, ChatRequest, ChatResponse, Message};