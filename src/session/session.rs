use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use crate::context::Context;

/// Represents a session with unique ID, name, creation date, and associated data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub created_at: u64, // Unix timestamp
    pub data: HashMap<String, String>, // Generic key-value storage for session data
    pub context: Context, // Context object for managing contextual information
}

impl Session {
    /// Creates a new session with generated 4-digit ID
    pub fn new(name: String) -> Self {
        let id = Self::generate_4_digit_id();
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            id,
            name,
            created_at,
            data: HashMap::new(),
            context: Context::new(),
        }
    }
    
    /// Creates a new session with specified root path for context
    pub fn new_with_root(name: String, root_path: PathBuf) -> Self {
        let id = Self::generate_4_digit_id();
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            id,
            name,
            created_at,
            data: HashMap::new(),
            context: Context::new_with_root(root_path),
        }
    }
    
    /// Generates a random 4-digit ID
    fn generate_4_digit_id() -> String {
        let mut rng = rand::thread_rng();
        let id: u16 = rng.gen_range(1000..10000);
        id.to_string()
    }
    
    /// Clears all data from the session
    pub fn clear_data(&mut self) {
        self.data.clear();
    }
}