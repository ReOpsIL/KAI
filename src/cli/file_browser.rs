//! File Browser Module
//!
//! Interactive file system navigation and selection functionality.
//! Provides a comprehensive file browser with metadata display,
//! sorting, filtering, and intuitive navigation.

use std::fs;
use std::path::{Path, PathBuf};
use std::io;

/// File system entry with metadata
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub extension: Option<String>,
    pub is_hidden: bool,
}

impl FileEntry {
    /// Create a new FileEntry from a path
    pub fn from_path(path: PathBuf) -> io::Result<Self> {
        let metadata = fs::metadata(&path)?;
        let name = path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        let is_directory = metadata.is_dir();
        let size = if is_directory { None } else { Some(metadata.len()) };
        let is_hidden = name.starts_with('.');
        
        let extension = if is_directory {
            None
        } else {
            path.extension().map(|ext| ext.to_string_lossy().to_string())
        };
        
        Ok(Self {
            name,
            path,
            is_directory,
            size,
            extension,
            is_hidden,
        })
    }
    
    /// Get display name with icon
    pub fn display_name(&self) -> String {
        let icon = if self.is_directory { "📁" } else { "📄" };
        let size_info = if let Some(size) = self.size {
            format!(" ({})", format_file_size(size))
        } else {
            String::new()
        };
        
        format!("{} {}{}", icon, self.name, size_info)
    }
    
    /// Get file type description
    pub fn file_type(&self) -> String {
        if self.is_directory {
            "Directory".to_string()
        } else if let Some(ext) = &self.extension {
            match ext.to_lowercase().as_str() {
                "rs" => "Rust Source".to_string(),
                "py" => "Python Script".to_string(),
                "js" => "JavaScript".to_string(),
                "ts" => "TypeScript".to_string(),
                "html" => "HTML Document".to_string(),
                "css" => "Stylesheet".to_string(),
                "json" => "JSON Data".to_string(),
                "toml" => "TOML Config".to_string(),
                "yaml" | "yml" => "YAML Config".to_string(),
                "md" => "Markdown".to_string(),
                "txt" => "Text File".to_string(),
                "log" => "Log File".to_string(),
                "png" | "jpg" | "jpeg" | "gif" => "Image".to_string(),
                "pdf" => "PDF Document".to_string(),
                "zip" | "tar" | "gz" => "Archive".to_string(),
                _ => format!("{} File", ext.to_uppercase()),
            }
        } else {
            "File".to_string()
        }
    }
}

/// File browser configuration
#[derive(Debug, Clone)]
pub struct FileBrowserConfig {
    pub show_hidden: bool,
    pub sort_by: SortBy,
    pub sort_direction: SortDirection,
    pub max_entries: Option<usize>,
    pub file_filters: Vec<String>,
}

impl Default for FileBrowserConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            sort_by: SortBy::Name,
            sort_direction: SortDirection::Ascending,
            max_entries: None,
            file_filters: vec![],
        }
    }
}

/// Sorting criteria for file entries
#[derive(Debug, Clone, Copy)]
pub enum SortBy {
    Name,
    Size,
    Type,
    Modified,
}

/// Sorting direction
#[derive(Debug, Clone, Copy)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Main file browser functionality
pub struct FileBrowser {
    config: FileBrowserConfig,
    current_path: PathBuf,
    history: Vec<PathBuf>,
    history_index: usize,
}

impl FileBrowser {
    /// Create a new file browser starting at the given path
    pub fn new(starting_path: PathBuf) -> Self {
        let current_path = if starting_path.exists() && starting_path.is_dir() {
            starting_path
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
        };
        
        let history = vec![current_path.clone()];
        
        Self {
            config: FileBrowserConfig::default(),
            current_path,
            history,
            history_index: 0,
        }
    }
    
    /// Get current directory path
    pub fn current_path(&self) -> &Path {
        &self.current_path
    }
    
    /// Read entries in the current directory
    pub fn read_current_directory(&self) -> io::Result<Vec<FileEntry>> {
        self.read_directory(&self.current_path)
    }
    
    /// Read entries in a specific directory
    pub fn read_directory(&self, path: &Path) -> io::Result<Vec<FileEntry>> {
        let mut entries = Vec::new();
        
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            match FileEntry::from_path(entry.path()) {
                Ok(file_entry) => {
                    // Apply filters
                    if !self.config.show_hidden && file_entry.is_hidden {
                        continue;
                    }
                    
                    if !self.config.file_filters.is_empty() {
                        if let Some(ext) = &file_entry.extension {
                            if !self.config.file_filters.contains(ext) {
                                continue;
                            }
                        } else if !file_entry.is_directory {
                            continue; // Skip files without extension if filters are active
                        }
                    }
                    
                    entries.push(file_entry);
                }
                Err(_) => continue, // Skip entries we can't read
            }
        }
        
        // Sort entries
        self.sort_entries(&mut entries);
        
        // Limit entries if configured
        if let Some(max) = self.config.max_entries {
            entries.truncate(max);
        }
        
        Ok(entries)
    }
    
    /// Navigate to a specific path
    pub fn navigate_to(&mut self, path: PathBuf) -> io::Result<()> {
        if !path.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Path does not exist"));
        }
        
        if !path.is_dir() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Path is not a directory"));
        }
        
        self.current_path = path.clone();
        
        // Add to history if different from current
        if self.history.is_empty() || self.history[self.history_index] != path {
            // Remove future history if we're not at the end
            if self.history_index < self.history.len() - 1 {
                self.history.truncate(self.history_index + 1);
            }
            
            self.history.push(path);
            self.history_index = self.history.len() - 1;
            
            // Limit history size
            if self.history.len() > 50 {
                self.history.remove(0);
                self.history_index -= 1;
            }
        }
        
        Ok(())
    }
    
    /// Navigate to parent directory
    pub fn navigate_up(&mut self) -> io::Result<()> {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to(parent.to_path_buf())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "Already at root"))
        }
    }
    
    /// Navigate back in history
    pub fn navigate_back(&mut self) -> io::Result<()> {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_path = self.history[self.history_index].clone();
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "No previous directory"))
        }
    }
    
    /// Navigate forward in history
    pub fn navigate_forward(&mut self) -> io::Result<()> {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.current_path = self.history[self.history_index].clone();
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "No next directory"))
        }
    }
    
    /// Get display entries for the current directory
    pub fn get_display_entries(&self) -> io::Result<Vec<String>> {
        let entries = self.read_current_directory()?;
        let mut display_entries = vec![".. (parent directory)".to_string()];
        
        for entry in entries {
            display_entries.push(entry.display_name());
        }
        
        Ok(display_entries)
    }
    
    /// Process user selection from display entries
    pub fn process_selection(&mut self, selection: &str, entries: &[FileEntry]) -> SelectionResult {
        if selection.starts_with(".. (parent") {
            match self.navigate_up() {
                Ok(()) => SelectionResult::NavigatedUp,
                Err(_) => SelectionResult::Error("Cannot navigate to parent directory".to_string()),
            }
        } else {
            // Extract entry name from display string
            let entry_name = if let Some(space_idx) = selection.find(' ') {
                let after_icon = &selection[space_idx + 1..];
                if let Some(paren_idx) = after_icon.find(" (") {
                    &after_icon[..paren_idx]
                } else {
                    after_icon
                }
            } else {
                selection
            };
            
            if let Some(entry) = entries.iter().find(|e| e.name == entry_name) {
                if entry.is_directory {
                    match self.navigate_to(entry.path.clone()) {
                        Ok(()) => SelectionResult::NavigatedTo(entry.path.clone()),
                        Err(e) => SelectionResult::Error(format!("Cannot navigate: {}", e)),
                    }
                } else {
                    SelectionResult::FileSelected(entry.path.clone())
                }
            } else {
                SelectionResult::Error("Entry not found".to_string())
            }
        }
    }
    
    /// Configure the file browser
    pub fn configure(&mut self, config: FileBrowserConfig) {
        self.config = config;
    }
    
    /// Get current configuration
    pub fn config(&self) -> &FileBrowserConfig {
        &self.config
    }
    
    /// Get navigation history
    pub fn history(&self) -> &[PathBuf] {
        &self.history
    }
    
    /// Sort entries according to current configuration
    fn sort_entries(&self, entries: &mut Vec<FileEntry>) {
        entries.sort_by(|a, b| {
            // Always put directories first
            match (a.is_directory, b.is_directory) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            
            let ordering = match self.config.sort_by {
                SortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortBy::Size => a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0)),
                SortBy::Type => a.file_type().cmp(&b.file_type()),
                SortBy::Modified => std::cmp::Ordering::Equal, // Would need metadata
            };
            
            match self.config.sort_direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });
    }
}

/// Result of processing a user selection
#[derive(Debug, Clone)]
pub enum SelectionResult {
    FileSelected(PathBuf),
    NavigatedTo(PathBuf),
    NavigatedUp,
    Error(String),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_file_browser_creation() {
        let current_dir = env::current_dir().unwrap();
        let browser = FileBrowser::new(current_dir.clone());
        assert_eq!(browser.current_path(), current_dir.as_path());
    }
    
    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
    }
    
    #[test]
    fn test_file_entry_display() {
        let temp_dir = env::temp_dir();
        if let Ok(entry) = FileEntry::from_path(temp_dir) {
            let display = entry.display_name();
            assert!(display.contains("📁"));
            assert!(display.len() > 2); // Should have icon and name
        }
    }
    
    #[test]
    fn test_file_browser_navigation() {
        let current_dir = env::current_dir().unwrap();
        let mut browser = FileBrowser::new(current_dir);
        
        // Test navigation up
        if browser.current_path().parent().is_some() {
            assert!(browser.navigate_up().is_ok());
            assert!(browser.navigate_back().is_ok());
        }
    }
    
    #[test]
    fn test_file_browser_config() {
        let mut browser = FileBrowser::new(env::current_dir().unwrap());
        let mut config = FileBrowserConfig::default();
        config.show_hidden = true;
        config.sort_by = SortBy::Size;
        
        browser.configure(config);
        assert!(browser.config().show_hidden);
        assert!(matches!(browser.config().sort_by, SortBy::Size));
    }
}