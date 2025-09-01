use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use crate::context::harvesters::{Harvester, HarvesterConfig, ModuleInfo, FileInfo};
use crate::context::story::{Story, ResponseMetadata};
use crate::llm::OpenRouterClient;

/// Enhanced context object that manages contextual information with file tracking and updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// The date and time when this context was created
    pub date: DateTime<Utc>,
    /// The date and time when this context was last updated
    pub last_updated: DateTime<Utc>,
    /// File modification timestamps for tracking changes
    pub file_timestamps: HashMap<PathBuf, DateTime<Utc>>,
    /// Root path for the context
    pub root_path: PathBuf,
    /// Whether the context has been initialized
    pub initialized: bool,
    /// Story object containing user prompts and LLM responses
    pub story: Story,
}

impl Context {
    /// Create a new empty context with the current date and time
    pub fn new() -> Self {
        Self {
            date: Utc::now(),
            last_updated: Utc::now(),
            file_timestamps: HashMap::new(),
            root_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            initialized: false,
            story: Story::new(),
        }
    }
    
    /// Create a new context with specified root path
    pub fn new_with_root(root_path: PathBuf) -> Self {
        Self {
            date: Utc::now(),
            last_updated: Utc::now(),
            file_timestamps: HashMap::new(),
            root_path,
            initialized: false,
            story: Story::new(),
        }
    }
    
    /// Update the context by running the harvester and refreshing context information
    pub async fn update(
        &mut self, 
        data_store: &crate::context::context_data_store::ContextDataStore, 
        openrouter_client: Option<OpenRouterClient>,
        force_refresh: bool
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting context update...");
        
        // Check if force refresh is requested or if this is the first initialization
        let needs_full_update = force_refresh || !self.initialized;
        
        if needs_full_update {
            println!("Performing full context refresh...");
            self.perform_full_update(data_store, openrouter_client).await?;
        } else {
            println!("Checking for modified files...");
            let modified_files = self.detect_modified_files()?;
            
            if !modified_files.is_empty() {
                println!("Found {} modified files, updating context...", modified_files.len());
                self.update_modified_files(data_store, openrouter_client, modified_files).await?;
            } else {
                println!("No file modifications detected, context is up to date.");
            }
        }
        
        self.last_updated = Utc::now();
        self.initialized = true;
        
        println!("Context update completed successfully.");
        Ok(())
    }
    
    /// Perform a full context update (harvest all files) with optimization for unchanged files
    async fn perform_full_update(
        &mut self,
        data_store: &crate::context::context_data_store::ContextDataStore,
        openrouter_client: Option<OpenRouterClient>
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create harvester configuration
        let config = HarvesterConfig {
            root_path: self.root_path.clone(),
            ..Default::default()
        };
        
        let mut harvester = Harvester::new(config);
        if let Some(client) = openrouter_client {
            harvester = harvester.with_openrouter(client);
        }
        
        // Get context directory path for optimization
        let context_dir = data_store.context_dir_path();
        let context_dir_option = if context_dir.exists() {
            Some(context_dir)
        } else {
            None
        };
        
        // Run optimized harvester to get all modules and files
        let modules = harvester.harvest_with_context_dir(context_dir_option).await?;
        
        // Update file timestamps
        self.update_file_timestamps(&modules)?;
        
        // Save results to data store
        data_store.save_harvester_results(&modules)?;
        
        Ok(())
    }
    
    /// Update only specific modified files
    async fn update_modified_files(
        &mut self,
        data_store: &crate::context::context_data_store::ContextDataStore,
        openrouter_client: Option<OpenRouterClient>,
        modified_files: Vec<PathBuf>
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create harvester configuration
        let config = HarvesterConfig {
            root_path: self.root_path.clone(),
            ..Default::default()
        };
        
        let mut harvester = Harvester::new(config);
        if let Some(client) = openrouter_client {
            harvester = harvester.with_openrouter(client);
        }
        
        // Process each modified file individually
        for file_path in modified_files {
            if let Ok(file_info) = self.create_file_info(&file_path) {
                // Update timestamp
                if let Ok(metadata) = fs::metadata(&file_path) {
                    if let Ok(modified) = metadata.modified() {
                        let modified_time = DateTime::<Utc>::from(modified);
                        self.file_timestamps.insert(file_path.clone(), modified_time);
                    }
                }
                
                // Save individual file info
                data_store.save_file_info(&file_info)?;
                
                println!("Updated context for file: {}", file_path.display());
            }
        }
        
        Ok(())
    }
    
    /// Detect files that have been modified since last update
    fn detect_modified_files(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut modified_files = Vec::new();
        
        // Create harvester to discover current files
        let config = HarvesterConfig {
            root_path: self.root_path.clone(),
            ..Default::default()
        };
        let harvester = Harvester::new(config);
        let current_files = harvester.discover_files()?;
        
        for file_info in current_files {
            let file_path = &file_info.path;
            
            // Check if file is new (not in our timestamp tracking)
            if !self.file_timestamps.contains_key(file_path) {
                modified_files.push(file_path.clone());
                continue;
            }
            
            // Check if file has been modified
            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(modified_time) = metadata.modified() {
                    let modified_datetime = DateTime::<Utc>::from(modified_time);
                    
                    if let Some(stored_time) = self.file_timestamps.get(file_path) {
                        if modified_datetime > *stored_time {
                            modified_files.push(file_path.clone());
                        }
                    }
                }
            }
        }
        
        Ok(modified_files)
    }
    
    /// Update file timestamps from module information
    fn update_file_timestamps(&mut self, modules: &[ModuleInfo]) -> Result<(), Box<dyn std::error::Error>> {
        for module in modules {
            for file_info in &module.files {
                if let Ok(metadata) = fs::metadata(&file_info.path) {
                    if let Ok(modified) = metadata.modified() {
                        let modified_time = DateTime::<Utc>::from(modified);
                        self.file_timestamps.insert(file_info.path.clone(), modified_time);
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Create FileInfo for a single file (for single file resolution)
    fn create_file_info(&self, file_path: &Path) -> Result<FileInfo, Box<dyn std::error::Error>> {
        let metadata = fs::metadata(file_path)?;
        
        let relative_path = file_path.strip_prefix(&self.root_path)
            .unwrap_or(file_path)
            .to_path_buf();
        
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());
        
        // Get file modification time
        let last_modified = metadata
            .modified()
            .ok()
            .map(|time| DateTime::<Utc>::from(time));
        
        Ok(FileInfo {
            path: file_path.to_path_buf(),
            relative_path,
            extension,
            size: metadata.len(),
            description: None,
            last_modified,
        })
    }
    
    /// Check if context needs refresh
    pub fn needs_refresh(&self, force_refresh: bool) -> bool {
        force_refresh || !self.initialized
    }
    
    /// Get the number of tracked files
    pub fn tracked_files_count(&self) -> usize {
        self.file_timestamps.len()
    }
    
    /// Add a user prompt to the story
    pub fn add_user_prompt(&mut self, content: String) {
        self.story.add_user_prompt(content);
    }
    
    /// Add a response to the most recent prompt
    pub fn add_response(&mut self, content: String, metadata: Option<ResponseMetadata>) {
        self.story.add_response(content, metadata);
    }
    
    /// Get user interactions within a specific time frame (in days)
    pub fn get_user_interactions_in_timeframe(&self, days: u32) -> Vec<(String, Option<String>, DateTime<Utc>)> {
        self.story.get_user_interactions_in_timeframe(days)
    }
    
    /// Query the story for time frame information and return formatted context
    pub fn query_story_timeframe(&self, days: u32) -> String {
        let interactions = self.get_user_interactions_in_timeframe(days);
        
        if interactions.is_empty() {
            return format!("No user interactions found in the last {} days.", days);
        }
        
        let mut context = format!("User interactions from the last {} days:\n\n", days);
        
        for (prompt, response, timestamp) in interactions {
            context.push_str(&format!("Time: {}\n", timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
            context.push_str(&format!("User: {}\n", prompt));
            
            if let Some(resp) = response {
                context.push_str(&format!("Assistant: {}\n", resp));
            } else {
                context.push_str("Assistant: [No response recorded]\n");
            }
            context.push_str("\n---\n\n");
        }
        
        context
    }
    
    /// Get the total number of story entries
    pub fn story_entries_count(&self) -> usize {
        self.story.len()
    }
    
    /// Clear the story history
    pub fn clear_story(&mut self) {
        self.story.clear();
    }
    
    /// Get only user entries from the story (filtering out harvester/system prompts)
    pub fn get_user_story_entries(&self) -> Vec<(String, Option<String>, DateTime<Utc>)> {
        self.story.get_user_entries()
            .into_iter()
            .map(|entry| (
                entry.prompt.content.clone(),
                entry.response.as_ref().map(|r| r.content.clone()),
                entry.prompt.timestamp,
            ))
            .collect()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let context = Context::new();
        assert!(context.date <= Utc::now());
    }

    #[test]
    fn test_context_default() {
        let context = Context::default();
        assert!(context.date <= Utc::now());
    }
    
    #[test]
    fn test_story_integration() {
        let mut context = Context::new();
        
        // Test adding user prompts and responses
        context.add_user_prompt("What is Rust?".to_string());
        assert_eq!(context.story_entries_count(), 1);
        
        context.add_response("Rust is a systems programming language.".to_string(), None);
        
        // Test time frame query
        let interactions = context.get_user_interactions_in_timeframe(1);
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].0, "What is Rust?");
        assert_eq!(interactions[0].1, Some("Rust is a systems programming language.".to_string()));
        
        // Test formatted context query
        let formatted = context.query_story_timeframe(1);
        assert!(formatted.contains("What is Rust?"));
        assert!(formatted.contains("Rust is a systems programming language"));
        
        // Test user entries filtering
        let user_entries = context.get_user_story_entries();
        assert_eq!(user_entries.len(), 1);
        
        // Test clearing story
        context.clear_story();
        assert_eq!(context.story_entries_count(), 0);
    }
    
    #[test]
    fn test_story_time_frame_filtering() {
        let mut context = Context::new();
        
        // Add a prompt and response
        context.add_user_prompt("Recent question".to_string());
        context.add_response("Recent answer".to_string(), None);
        
        // Should find entries from last 7 days
        let recent = context.get_user_interactions_in_timeframe(7);
        assert_eq!(recent.len(), 1);
        
        // Should not find entries from last 0 days
        let none = context.get_user_interactions_in_timeframe(0);
        assert_eq!(none.len(), 0);
    }
    
    #[test]
    fn test_story_response_metadata() {
        let mut context = Context::new();
        
        context.add_user_prompt("Test with metadata".to_string());
        
        let metadata = ResponseMetadata::new()
            .with_model("gpt-4".to_string())
            .with_token_count(100)
            .with_processing_time(1500);
            
        context.add_response("Response with metadata".to_string(), Some(metadata));
        
        assert_eq!(context.story_entries_count(), 1);
        
        let latest = context.story.get_latest_entry().unwrap();
        assert!(latest.response.is_some());
        
        let response = latest.response.as_ref().unwrap();
        assert!(response.metadata.is_some());
        
        let meta = response.metadata.as_ref().unwrap();
        assert_eq!(meta.model, Some("gpt-4".to_string()));
        assert_eq!(meta.token_count, Some(100));
        assert_eq!(meta.processing_time_ms, Some(1500));
    }
}