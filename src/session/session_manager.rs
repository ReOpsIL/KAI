use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use super::session::Session;

/// Result type for session operations, following project patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResult {
    pub success: bool,
    pub message: String,
    pub data: Option<String>, // JSON serialized data when needed
}

impl SessionResult {
    pub fn success(message: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data: None,
        }
    }
    
    pub fn success_with_data(message: &str, data: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data: Some(data.to_string()),
        }
    }
    
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            data: None,
        }
    }
}

/// Main session manager for handling all session operations
pub struct SessionManager {
    sessions: HashMap<String, Session>, // ID -> Session mapping
    active_session_id: Option<String>,
    storage_path: PathBuf,
}

impl SessionManager {
    /// Creates a new session manager with specified storage path
    pub fn new(storage_path: impl AsRef<Path>) -> Self {
        let storage_path = storage_path.as_ref().to_path_buf();
        let mut manager = Self {
            sessions: HashMap::new(),
            active_session_id: None,
            storage_path,
        };
        
        // Try to load existing sessions
        let _ = manager.load_sessions();
        manager
    }
    
    /// Creates a new session with the given name
    pub fn create_session(&mut self, name: &str) -> SessionResult {
        if name.trim().is_empty() {
            return SessionResult::error("Session name cannot be empty");
        }
        
        let session = Session::new(name.to_string());
        let session_id = session.id.clone();
        
        self.sessions.insert(session_id.clone(), session);
        
        // Save sessions to persistence
        if let Err(e) = self.save_sessions() {
            return SessionResult::error(&format!("Failed to save session: {}", e));
        }
        
        SessionResult::success_with_data(
            &format!("Session '{}' created with ID: {}", name, session_id),
            &session_id
        )
    }
    
    /// Deletes a session by ID or name
    pub fn delete_session(&mut self, identifier: &str) -> SessionResult {
        // Try to find session by ID first, then by name
        let session_id = if let Some(session) = self.sessions.get(identifier) {
            Some(identifier.to_string())
        } else {
            // Search by name
            self.sessions.iter()
                .find(|(_, session)| session.name == identifier)
                .map(|(id, _)| id.clone())
        };
        
        match session_id {
            Some(id) => {
                let session = self.sessions.remove(&id).unwrap();
                
                // Clear active session if it was the deleted one
                if self.active_session_id.as_ref() == Some(&id) {
                    self.active_session_id = None;
                }
                
                // Save sessions to persistence
                if let Err(e) = self.save_sessions() {
                    return SessionResult::error(&format!("Failed to save after deletion: {}", e));
                }
                
                SessionResult::success(&format!("Session '{}' (ID: {}) deleted", session.name, id))
            }
            None => SessionResult::error(&format!("Session '{}' not found", identifier))
        }
    }
    
    /// Lists all sessions with optional name filter, ordered by date
    pub fn list_sessions(&self, name_filter: Option<&str>) -> SessionResult {
        let mut sessions: Vec<&Session> = self.sessions.values().collect();
        
        // Apply name filter if provided
        if let Some(filter) = name_filter {
            sessions.retain(|session| session.name.contains(filter));
        }
        
        // Sort by creation date (newest first)
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        if sessions.is_empty() {
            let message = match name_filter {
                Some(filter) => format!("No sessions found matching '{}'", filter),
                None => "No sessions found".to_string(),
            };
            return SessionResult::success(&message);
        }
        
        // Format session list
        let session_list = sessions.iter()
            .map(|s| {
                let active = if self.active_session_id.as_ref() == Some(&s.id) { " (active)" } else { "" };
                let date = chrono::DateTime::from_timestamp(s.created_at as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown date".to_string());
                format!("ID: {} | Name: {} | Created: {}{}", s.id, s.name, date, active)
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        let message = match name_filter {
            Some(filter) => format!("Sessions matching '{}' (ordered by date):", filter),
            None => "All sessions (ordered by date):".to_string(),
        };
        
        SessionResult::success_with_data(&message, &session_list)
    }
    
    /// Selects/activates a session for use
    pub fn select_session(&mut self, identifier: &str) -> SessionResult {
        // Find session by ID or name
        let session_id = if self.sessions.contains_key(identifier) {
            Some(identifier.to_string())
        } else {
            // Search by name
            self.sessions.iter()
                .find(|(_, session)| session.name == identifier)
                .map(|(id, _)| id.clone())
        };
        
        match session_id {
            Some(id) => {
                let session = &self.sessions[&id];
                self.active_session_id = Some(id.clone());
                
                // Save sessions to persistence (to save active session)
                if let Err(e) = self.save_sessions() {
                    return SessionResult::error(&format!("Failed to save active session: {}", e));
                }
                
                SessionResult::success(&format!("Session '{}' (ID: {}) is now active", session.name, id))
            }
            None => SessionResult::error(&format!("Session '{}' not found", identifier))
        }
    }
    
    /// Gets the currently active session
    pub fn get_active_session(&self) -> Option<&Session> {
        self.active_session_id.as_ref()
            .and_then(|id| self.sessions.get(id))
    }
    
    /// Clears data from a session
    pub fn clean_session_data(&mut self, identifier: &str) -> SessionResult {
        // Find session by ID or name
        let session_id = if self.sessions.contains_key(identifier) {
            Some(identifier.to_string())
        } else {
            // Search by name
            self.sessions.iter()
                .find(|(_, session)| session.name == identifier)
                .map(|(id, _)| id.clone())
        };
        
        match session_id {
            Some(id) => {
                let session_name = if let Some(session) = self.sessions.get_mut(&id) {
                    session.clear_data();
                    session.name.clone()
                } else {
                    return SessionResult::error("Internal error: session not found");
                };
                
                // Save sessions to persistence
                if let Err(e) = self.save_sessions() {
                    return SessionResult::error(&format!("Failed to save after cleaning: {}", e));
                }
                
                SessionResult::success(&format!("Data cleared for session '{}' (ID: {})", session_name, id))
            }
            None => SessionResult::error(&format!("Session '{}' not found", identifier))
        }
    }
    
    /// Saves sessions to persistent storage
    fn save_sessions(&self) -> Result<(), std::io::Error> {
        // Create storage directory if it doesn't exist
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let data = SessionManagerData {
            sessions: self.sessions.clone(),
            active_session_id: self.active_session_id.clone(),
        };
        
        let json = serde_json::to_string_pretty(&data)?;
        fs::write(&self.storage_path, json)?;
        Ok(())
    }
    
    /// Loads sessions from persistent storage
    fn load_sessions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.storage_path.exists() {
            return Ok(()); // No existing sessions to load
        }
        
        let contents = fs::read_to_string(&self.storage_path)?;
        let data: SessionManagerData = serde_json::from_str(&contents)?;
        
        self.sessions = data.sessions;
        self.active_session_id = data.active_session_id;
        
        Ok(())
    }
}

/// Helper struct for serializing session manager state
#[derive(Serialize, Deserialize)]
struct SessionManagerData {
    sessions: HashMap<String, Session>,
    active_session_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_create_session() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("sessions.json");
        let mut manager = SessionManager::new(&storage_path);
        
        let result = manager.create_session("Test Session");
        assert!(result.success);
        assert!(result.data.is_some());
        
        // Verify session was created
        assert_eq!(manager.sessions.len(), 1);
        let session = manager.sessions.values().next().unwrap();
        assert_eq!(session.name, "Test Session");
        assert_eq!(session.id.len(), 4);
    }
    
    #[test]
    fn test_delete_session() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("sessions.json");
        let mut manager = SessionManager::new(&storage_path);
        
        let result = manager.create_session("Test Session");
        let session_id = result.data.unwrap();
        
        // Delete by ID
        let result = manager.delete_session(&session_id);
        assert!(result.success);
        assert_eq!(manager.sessions.len(), 0);
    }
    
    #[test]
    fn test_list_sessions() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("sessions.json");
        let mut manager = SessionManager::new(&storage_path);
        
        manager.create_session("First Session");
        manager.create_session("Second Session");
        
        let result = manager.list_sessions(None);
        assert!(result.success);
        assert!(result.data.is_some());
        
        // Test filtering
        let result = manager.list_sessions(Some("First"));
        assert!(result.success);
        assert!(result.data.unwrap().contains("First Session"));
    }
    
    #[test]
    fn test_select_session() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("sessions.json");
        let mut manager = SessionManager::new(&storage_path);
        
        let result = manager.create_session("Test Session");
        let session_id = result.data.unwrap();
        
        let result = manager.select_session(&session_id);
        assert!(result.success);
        assert!(manager.get_active_session().is_some());
    }
    
    #[test]
    fn test_clean_session_data() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("sessions.json");
        let mut manager = SessionManager::new(&storage_path);
        
        let result = manager.create_session("Test Session");
        let session_id = result.data.unwrap();
        
        // Add some data to the session
        if let Some(session) = manager.sessions.get_mut(&session_id) {
            session.data.insert("test_key".to_string(), "test_value".to_string());
        }
        
        let result = manager.clean_session_data(&session_id);
        assert!(result.success);
        
        let session = &manager.sessions[&session_id];
        assert!(session.data.is_empty());
    }
}