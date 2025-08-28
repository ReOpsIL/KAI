use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use crate::context::harvesters::{ModuleInfo, FileInfo};

/// Context data store for managing harvester output
pub struct ContextDataStore {
    root_path: PathBuf,
    context_dir: PathBuf,
}

impl ContextDataStore {
    /// Create a new context data store with the given root path
    pub fn new(root_path: PathBuf) -> Self {
        let context_dir = root_path.join(".context");
        Self {
            root_path,
            context_dir,
        }
    }
    
    /// Create a new context data store with current working directory as root
    pub fn with_current_dir() -> Result<Self, std::io::Error> {
        let root_path = std::env::current_dir()?;
        Ok(Self::new(root_path))
    }
    
    /// Ensure the .context directory exists
    fn ensure_context_dir(&self) -> Result<(), std::io::Error> {
        if !self.context_dir.exists() {
            fs::create_dir_all(&self.context_dir)?;
        }
        Ok(())
    }
    
    /// Save file information as markdown
    pub fn save_file_info(&self, file_info: &crate::context::FileInfo) -> Result<(), Box<dyn std::error::Error>> {
        self.ensure_context_dir()?;
        
        // Create a safe filename from the relative path
        let safe_filename = self.create_safe_filename(&file_info.relative_path, "file");
        let markdown_path = self.context_dir.join(format!("{}.md", safe_filename));
        
        let content = self.format_file_info_as_markdown(file_info);
        fs::write(markdown_path, content)?;
        
        Ok(())
    }
    
    /// Save module information as markdown
    pub fn save_module_info(&self, module_info: &crate::context::ModuleInfo) -> Result<(), Box<dyn std::error::Error>> {
        self.ensure_context_dir()?;
        
        // Create a safe filename from the module path
        let safe_filename = self.create_safe_filename(&module_info.path, "module");
        let markdown_path = self.context_dir.join(format!("{}.md", safe_filename));
        
        let content = self.format_module_info_as_markdown(module_info);
        fs::write(markdown_path, content)?;
        
        Ok(())
    }
    
    /// Save all harvester results
    pub fn save_harvester_results(&self, modules: &[crate::context::ModuleInfo]) -> Result<(), Box<dyn std::error::Error>> {
        self.ensure_context_dir()?;
        
        // Save individual file information
        for module in modules {
            for file_info in &module.files {
                if let Err(e) = self.save_file_info(file_info) {
                    eprintln!("Failed to save file info for {}: {}", file_info.relative_path.display(), e);
                }
            }
            
            // Save module information
            if let Err(e) = self.save_module_info(module) {
                eprintln!("Failed to save module info for {}: {}", module.name, e);
            }
        }
        
        // Create a summary file
        self.save_summary(modules)?;
        
        Ok(())
    }
    
    /// Create a safe filename from a path
    fn create_safe_filename(&self, path: &Path, prefix: &str) -> String {
        let path_str = path.to_string_lossy();
        let safe_str = path_str
            .replace('/', "_")
            .replace('\\', "_")
            .replace(':', "_")
            .replace('<', "_")
            .replace('>', "_")
            .replace('"', "_")
            .replace('|', "_")
            .replace('?', "_")
            .replace('*', "_");
        
        format!("{}_{}", prefix, safe_str)
    }
    
    /// Format file information as markdown
    fn format_file_info_as_markdown(&self, file_info: &crate::context::FileInfo) -> String {
        let mut content = String::new();
        content.push_str(&format!("# File: {}\n\n", file_info.relative_path.display()));
        content.push_str(&format!("**Original Path:** `{}`\n\n", file_info.path.display()));
        content.push_str(&format!("**Size:** {} bytes\n\n", file_info.size));
        
        if let Some(ext) = &file_info.extension {
            content.push_str(&format!("**Extension:** {}\n\n", ext));
        }
        
        content.push_str("## Description\n\n");
        if let Some(description) = &file_info.description {
            content.push_str(description);
        } else {
            content.push_str("*No description available*");
        }
        content.push_str("\n\n");
        
        content.push_str(&format!("---\n*Generated on: {}*\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        content
    }
    
    /// Format module information as markdown
    fn format_module_info_as_markdown(&self, module_info: &crate::context::ModuleInfo) -> String {
        let mut content = String::new();
        content.push_str(&format!("# Module: {}\n\n", module_info.name));
        content.push_str(&format!("**Path:** `{}`\n\n", module_info.path.display()));
        
        content.push_str("## Description\n\n");
        if let Some(description) = &module_info.description {
            content.push_str(description);
        } else {
            content.push_str("*No description available*");
        }
        content.push_str("\n\n");
        
        if let Some(architecture_notes) = &module_info.architecture_notes {
            content.push_str("## Architecture Notes\n\n");
            content.push_str(architecture_notes);
            content.push_str("\n\n");
        }
        
        content.push_str("## Files in Module\n\n");
        for file_info in &module_info.files {
            content.push_str(&format!("### {}\n\n", file_info.relative_path.display()));
            content.push_str(&format!("- **Size:** {} bytes\n", file_info.size));
            if let Some(ext) = &file_info.extension {
                content.push_str(&format!("- **Extension:** {}\n", ext));
            }
            if let Some(description) = &file_info.description {
                content.push_str("- **Description:** ");
                // Take only the first line for summary
                if let Some(first_line) = description.lines().next() {
                    content.push_str(first_line);
                }
                content.push('\n');
            }
            content.push('\n');
        }
        
        content.push_str(&format!("---\n*Generated on: {}*\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        content
    }
    
    /// Create a summary file of all harvested information
    fn save_summary(&self, modules: &[crate::context::ModuleInfo]) -> Result<(), Box<dyn std::error::Error>> {
        let summary_path = self.context_dir.join("_SUMMARY.md");
        let mut content = String::new();
        
        content.push_str("# Project Context Summary\n\n");
        content.push_str(&format!("**Generated on:** {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        content.push_str(&format!("**Root Path:** `{}`\n\n", self.root_path.display()));
        
        // Overall statistics
        let total_files: usize = modules.iter().map(|m| m.files.len()).sum();
        let total_modules = modules.len();
        
        content.push_str("## Statistics\n\n");
        content.push_str(&format!("- **Total Modules:** {}\n", total_modules));
        content.push_str(&format!("- **Total Files:** {}\n\n", total_files));
        
        content.push_str("## Modules Overview\n\n");
        for module in modules {
            content.push_str(&format!("### {}\n\n", module.name));
            content.push_str(&format!("- **Path:** `{}`\n", module.path.display()));
            content.push_str(&format!("- **Files:** {}\n", module.files.len()));
            
            if let Some(description) = &module.description {
                content.push_str("- **Description:** ");
                // Take only the first line for summary
                if let Some(first_line) = description.lines().next() {
                    content.push_str(first_line);
                }
                content.push('\n');
            }
            content.push('\n');
        }
        
        content.push_str("## Files by Extension\n\n");
        let mut extensions: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for module in modules {
            for file in &module.files {
                if let Some(ext) = &file.extension {
                    *extensions.entry(ext.clone()).or_insert(0) += 1;
                } else {
                    *extensions.entry("(no extension)".to_string()).or_insert(0) += 1;
                }
            }
        }
        
        let mut sorted_extensions: Vec<_> = extensions.into_iter().collect();
        sorted_extensions.sort_by(|a, b| b.1.cmp(&a.1));
        
        for (ext, count) in sorted_extensions {
            content.push_str(&format!("- **.{}:** {} files\n", ext, count));
        }
        
        fs::write(summary_path, content)?;
        Ok(())
    }
    
    /// Get the context directory path
    pub fn context_dir_path(&self) -> &Path {
        &self.context_dir
    }
    
    /// Check if context directory exists
    pub fn context_dir_exists(&self) -> bool {
        self.context_dir.exists()
    }
    
    /// Clear all context files
    pub fn clear_context(&self) -> Result<(), std::io::Error> {
        if self.context_dir.exists() {
            fs::remove_dir_all(&self.context_dir)?;
        }
        Ok(())
    }
}
