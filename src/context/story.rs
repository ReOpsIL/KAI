use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Represents a user prompt with timestamp
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Prompt {
    /// The user's input text
    pub content: String,
    /// When this prompt was created
    pub timestamp: DateTime<Utc>,
    /// Source identifier (to distinguish user prompts from system/harvester prompts)
    pub source: PromptSource,
}

/// Represents an LLM response with timestamp
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Response {
    /// The LLM's response text
    pub content: String,
    /// When this response was generated
    pub timestamp: DateTime<Utc>,
    /// Optional metadata about the response
    pub metadata: Option<ResponseMetadata>,
}

/// Source of the prompt to filter out non-user prompts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromptSource {
    User,
    Harvester,
    System,
    Module(String),
}

/// Optional metadata for responses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseMetadata {
    /// Model used for the response
    pub model: Option<String>,
    /// Token count if available
    pub token_count: Option<usize>,
    /// Processing time in milliseconds
    pub processing_time_ms: Option<u64>,
}

/// A prompt-response pair
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptResponsePair {
    pub prompt: Prompt,
    pub response: Option<Response>, // Response might be pending
}

/// Story object that maintains the history of user prompts and LLM responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    /// Chronologically ordered list of prompt-response pairs
    entries: VecDeque<PromptResponsePair>,
    /// Maximum number of entries to keep in memory
    max_entries: usize,
}

impl Prompt {
    /// Create a new user prompt
    pub fn new_user(content: String) -> Self {
        Self {
            content,
            timestamp: Utc::now(),
            source: PromptSource::User,
        }
    }
    
    /// Create a new prompt with specified source
    pub fn new_with_source(content: String, source: PromptSource) -> Self {
        Self {
            content,
            timestamp: Utc::now(),
            source,
        }
    }
    
    /// Check if this is a user prompt
    pub fn is_user_prompt(&self) -> bool {
        matches!(self.source, PromptSource::User)
    }
}

impl Response {
    /// Create a new response
    pub fn new(content: String) -> Self {
        Self {
            content,
            timestamp: Utc::now(),
            metadata: None,
        }
    }
    
    /// Create a new response with metadata
    pub fn new_with_metadata(content: String, metadata: ResponseMetadata) -> Self {
        Self {
            content,
            timestamp: Utc::now(),
            metadata: Some(metadata),
        }
    }
}

impl ResponseMetadata {
    /// Create new response metadata
    pub fn new() -> Self {
        Self {
            model: None,
            token_count: None,
            processing_time_ms: None,
        }
    }
    
    /// Set model name
    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }
    
    /// Set token count
    pub fn with_token_count(mut self, count: usize) -> Self {
        self.token_count = Some(count);
        self
    }
    
    /// Set processing time
    pub fn with_processing_time(mut self, time_ms: u64) -> Self {
        self.processing_time_ms = Some(time_ms);
        self
    }
}

impl Default for ResponseMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl Story {
    /// Create a new empty story
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries: 1000, // Default maximum entries
        }
    }
    
    /// Create a new story with specified maximum entries
    pub fn new_with_capacity(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
        }
    }
    
    /// Add a user prompt (creates a new entry without response)
    pub fn add_user_prompt(&mut self, content: String) {
        let prompt = Prompt::new_user(content);
        let entry = PromptResponsePair {
            prompt,
            response: None,
        };
        
        self.entries.push_back(entry);
        
        // Maintain maximum size
        if self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }
    
    /// Add a response to the most recent prompt
    pub fn add_response(&mut self, content: String, metadata: Option<ResponseMetadata>) {
        if let Some(last_entry) = self.entries.back_mut() {
            if last_entry.response.is_none() {
                last_entry.response = Some(if let Some(meta) = metadata {
                    Response::new_with_metadata(content, meta)
                } else {
                    Response::new(content)
                });
            }
        }
    }
    
    /// Get entries within a specific time frame (last N days)
    pub fn get_time_frame_entries(&self, days: u32) -> Vec<&PromptResponsePair> {
        let cutoff_time = Utc::now() - Duration::days(days as i64);
        
        self.entries
            .iter()
            .filter(|entry| {
                // Include entry if prompt is within timeframe
                entry.prompt.timestamp >= cutoff_time && entry.prompt.is_user_prompt()
            })
            .collect()
    }
    
    /// Get all user prompts and responses within time frame
    pub fn get_user_interactions_in_timeframe(&self, days: u32) -> Vec<(String, Option<String>, DateTime<Utc>)> {
        self.get_time_frame_entries(days)
            .into_iter()
            .map(|entry| (
                entry.prompt.content.clone(),
                entry.response.as_ref().map(|r| r.content.clone()),
                entry.prompt.timestamp,
            ))
            .collect()
    }
    
    /// Get total number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if story is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    
    /// Get all user entries (filtering out harvester/system prompts)
    pub fn get_user_entries(&self) -> Vec<&PromptResponsePair> {
        self.entries
            .iter()
            .filter(|entry| entry.prompt.is_user_prompt())
            .collect()
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    
    /// Get the most recent entry
    pub fn get_latest_entry(&self) -> Option<&PromptResponsePair> {
        self.entries.back()
    }
}

impl Default for Story {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_prompt_creation() {
        let prompt = Prompt::new_user("Hello world".to_string());
        assert_eq!(prompt.content, "Hello world");
        assert!(prompt.is_user_prompt());
    }
    
    #[test]
    fn test_response_creation() {
        let response = Response::new("Hello back!".to_string());
        assert_eq!(response.content, "Hello back!");
    }
    
    #[test]
    fn test_story_add_prompt_and_response() {
        let mut story = Story::new();
        
        story.add_user_prompt("What is Rust?".to_string());
        assert_eq!(story.len(), 1);
        
        story.add_response("Rust is a systems programming language.".to_string(), None);
        
        let latest = story.get_latest_entry().unwrap();
        assert_eq!(latest.prompt.content, "What is Rust?");
        assert!(latest.response.is_some());
        assert_eq!(latest.response.as_ref().unwrap().content, "Rust is a systems programming language.");
    }
    
    #[test]
    fn test_time_frame_filtering() {
        let mut story = Story::new();
        
        // Add a user prompt
        story.add_user_prompt("Recent question".to_string());
        story.add_response("Recent answer".to_string(), None);
        
        // Get entries from last 7 days
        let recent_entries = story.get_time_frame_entries(7);
        assert_eq!(recent_entries.len(), 1);
        
        // Get entries from last 0 days (should be empty)
        let no_entries = story.get_time_frame_entries(0);
        assert_eq!(no_entries.len(), 0);
    }
    
    #[test]
    fn test_user_prompt_filtering() {
        let mut story = Story::new();
        
        // Add user prompt
        let user_prompt = Prompt::new_user("User question".to_string());
        let user_entry = PromptResponsePair {
            prompt: user_prompt,
            response: Some(Response::new("Answer".to_string())),
        };
        story.entries.push_back(user_entry);
        
        // Add harvester prompt
        let harvester_prompt = Prompt::new_with_source("System info".to_string(), PromptSource::Harvester);
        let harvester_entry = PromptResponsePair {
            prompt: harvester_prompt,
            response: None,
        };
        story.entries.push_back(harvester_entry);
        
        // Only user entries should be returned
        let user_entries = story.get_user_entries();
        assert_eq!(user_entries.len(), 1);
        assert_eq!(user_entries[0].prompt.content, "User question");
    }
}