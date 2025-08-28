use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use glob::glob;
use regex::Regex;

/// File system tools for OpenRouter LLM integration
/// These tools provide comprehensive file and directory operations with wildcard support

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSystemTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: ToolParameters,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolParameters {
    #[serde(rename = "type")]
    pub param_type: String,
    pub properties: serde_json::Value,
    pub required: Vec<String>,
}

/// Result structure for tool execution
#[derive(Serialize, Deserialize, Debug)]
pub struct ToolResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// File system operations implementation
pub struct FileSystemOperations;

impl FileSystemOperations {
    /// Read file contents
    pub fn read_file(path: &str) -> ToolResult {
        match std::fs::read_to_string(path) {
            Ok(content) => ToolResult {
                success: true,
                data: Some(serde_json::json!({
                    "path": path,
                    "content": content,
                    "size": content.len()
                })),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                data: None,
                error: Some(format!("Failed to read file '{}': {}", path, e)),
            },
        }
    }

    /// Write content to file
    pub fn write_file(path: &str, content: &str, append: Option<bool>) -> ToolResult {
        let append_mode = append.unwrap_or(false);
        
        let result = if append_mode {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut file| file.write_all(content.as_bytes()))
        } else {
            std::fs::write(path, content)
        };

        match result {
            Ok(_) => ToolResult {
                success: true,
                data: Some(serde_json::json!({
                    "path": path,
                    "bytes_written": content.len(),
                    "mode": if append_mode { "append" } else { "write" }
                })),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                data: None,
                error: Some(format!("Failed to write to file '{}': {}", path, e)),
            },
        }
    }

    /// List directory contents with optional wildcard pattern
    pub fn list_directory(path: &str, pattern: Option<&str>, recursive: Option<bool>) -> ToolResult {
        let recursive_mode = recursive.unwrap_or(false);
        let search_pattern = if let Some(p) = pattern {
            format!("{}/{}", path, p)
        } else {
            format!("{}/*", path)
        };

        let glob_pattern = if recursive_mode {
            format!("{}/**/*", path)
        } else {
            search_pattern
        };

        match glob(&glob_pattern) {
            Ok(entries) => {
                let mut files = Vec::new();
                let mut dirs = Vec::new();
                
                for entry in entries {
                    match entry {
                        Ok(path_buf) => {
                            let path_str = path_buf.to_string_lossy().to_string();
                            let metadata = std::fs::metadata(&path_buf);
                            
                            if let Ok(meta) = metadata {
                                let entry_info = serde_json::json!({
                                    "path": path_str,
                                    "name": path_buf.file_name().unwrap_or_default().to_string_lossy(),
                                    "size": meta.len(),
                                    "is_file": meta.is_file(),
                                    "is_dir": meta.is_dir(),
                                    "modified": meta.modified().ok()
                                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                        .map(|d| d.as_secs())
                                });
                                
                                if meta.is_file() {
                                    files.push(entry_info);
                                } else if meta.is_dir() {
                                    dirs.push(entry_info);
                                }
                            }
                        }
                        Err(e) => {
                            return ToolResult {
                                success: false,
                                data: None,
                                error: Some(format!("Error reading directory entry: {}", e)),
                            };
                        }
                    }
                }

                ToolResult {
                    success: true,
                    data: Some(serde_json::json!({
                        "path": path,
                        "pattern": pattern,
                        "recursive": recursive_mode,
                        "directories": dirs,
                        "files": files,
                        "total_count": files.len() + dirs.len()
                    })),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                data: None,
                error: Some(format!("Failed to list directory '{}': {}", path, e)),
            },
        }
    }

    /// Create file or directory
    pub fn create_path(path: &str, is_directory: Option<bool>) -> ToolResult {
        let create_dir = is_directory.unwrap_or(false);
        
        let result = if create_dir {
            std::fs::create_dir_all(path)
        } else {
            // Create parent directories if they don't exist
            if let Some(parent) = Path::new(path).parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return ToolResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to create parent directories: {}", e)),
                    };
                }
            }
            std::fs::File::create(path).map(|_| ())
        };

        match result {
            Ok(_) => ToolResult {
                success: true,
                data: Some(serde_json::json!({
                    "path": path,
                    "type": if create_dir { "directory" } else { "file" },
                    "created": true
                })),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                data: None,
                error: Some(format!("Failed to create {}: {}", 
                    if create_dir { "directory" } else { "file" }, e)),
            },
        }
    }

    /// Delete file or directory with wildcard support
    pub fn delete_path(pattern: &str, recursive: Option<bool>) -> ToolResult {
        let recursive_mode = recursive.unwrap_or(false);
        let mut deleted_items = Vec::new();
        let mut errors = Vec::new();

        match glob(pattern) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(path_buf) => {
                            let path_str = path_buf.to_string_lossy().to_string();
                            
                            let result = if path_buf.is_dir() {
                                if recursive_mode {
                                    std::fs::remove_dir_all(&path_buf)
                                } else {
                                    std::fs::remove_dir(&path_buf)
                                }
                            } else {
                                std::fs::remove_file(&path_buf)
                            };

                            match result {
                                Ok(_) => deleted_items.push(path_str),
                                Err(e) => errors.push(format!("Failed to delete '{}': {}", path_str, e)),
                            }
                        }
                        Err(e) => errors.push(format!("Pattern error: {}", e)),
                    }
                }

                ToolResult {
                    success: errors.is_empty(),
                    data: Some(serde_json::json!({
                        "pattern": pattern,
                        "deleted_items": deleted_items,
                        "deleted_count": deleted_items.len(),
                        "errors": errors
                    })),
                    error: if errors.is_empty() { None } else { Some(errors.join("; ")) },
                }
            }
            Err(e) => ToolResult {
                success: false,
                data: None,
                error: Some(format!("Invalid glob pattern '{}': {}", pattern, e)),
            },
        }
    }

    /// Search for text in files using grep-like functionality
    pub fn grep_files(pattern: &str, file_pattern: &str, case_sensitive: Option<bool>, 
                      line_numbers: Option<bool>, context_lines: Option<u32>) -> ToolResult {
        let case_sens = case_sensitive.unwrap_or(true);
        let show_line_nums = line_numbers.unwrap_or(true);
        let context = context_lines.unwrap_or(0);

        let regex = match if case_sens {
            Regex::new(pattern)
        } else {
            Regex::new(&format!("(?i){}", pattern))
        } {
            Ok(r) => r,
            Err(e) => return ToolResult {
                success: false,
                data: None,
                error: Some(format!("Invalid regex pattern '{}': {}", pattern, e)),
            },
        };

        let mut results = Vec::new();
        let mut total_matches = 0;

        match glob(file_pattern) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(path_buf) => {
                            if path_buf.is_file() {
                                match std::fs::read_to_string(&path_buf) {
                                    Ok(content) => {
                                        let lines: Vec<&str> = content.lines().collect();
                                        let mut file_matches = Vec::new();

                                        for (line_num, line) in lines.iter().enumerate() {
                                            if regex.is_match(line) {
                                                let mut match_info = serde_json::json!({
                                                    "line": line.to_string(),
                                                    "line_number": line_num + 1
                                                });

                                                if context > 0 {
                                                    let start = line_num.saturating_sub(context as usize);
                                                    let end = std::cmp::min(line_num + context as usize + 1, lines.len());
                                                    let context_lines: Vec<String> = lines[start..end]
                                                        .iter()
                                                        .enumerate()
                                                        .map(|(i, l)| format!("{}: {}", start + i + 1, l))
                                                        .collect();
                                                    match_info.as_object_mut().unwrap()
                                                        .insert("context".to_string(), serde_json::json!(context_lines));
                                                }

                                                file_matches.push(match_info);
                                                total_matches += 1;
                                            }
                                        }

                                        if !file_matches.is_empty() {
                                            results.push(serde_json::json!({
                                                "file": path_buf.to_string_lossy(),
                                                "matches": file_matches,
                                                "match_count": file_matches.len()
                                            }));
                                        }
                                    }
                                    Err(e) => {
                                        // Skip files that can't be read (binary files, etc.)
                                        continue;
                                    }
                                }
                            }
                        }
                        Err(_) => continue,
                    }
                }

                ToolResult {
                    success: true,
                    data: Some(serde_json::json!({
                        "pattern": pattern,
                        "file_pattern": file_pattern,
                        "case_sensitive": case_sens,
                        "results": results,
                        "total_matches": total_matches,
                        "files_with_matches": results.len()
                    })),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                data: None,
                error: Some(format!("Invalid file pattern '{}': {}", file_pattern, e)),
            },
        }
    }

    /// Search and replace text in files
    pub fn search_replace(search_pattern: &str, replace_text: &str, file_pattern: &str, 
                         case_sensitive: Option<bool>, backup: Option<bool>) -> ToolResult {
        let case_sens = case_sensitive.unwrap_or(true);
        let create_backup = backup.unwrap_or(true);

        let regex = match if case_sens {
            Regex::new(search_pattern)
        } else {
            Regex::new(&format!("(?i){}", search_pattern))
        } {
            Ok(r) => r,
            Err(e) => return ToolResult {
                success: false,
                data: None,
                error: Some(format!("Invalid regex pattern '{}': {}", search_pattern, e)),
            },
        };

        let mut modified_files = Vec::new();
        let mut errors = Vec::new();
        let mut total_replacements = 0;

        match glob(file_pattern) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(path_buf) => {
                            if path_buf.is_file() {
                                match std::fs::read_to_string(&path_buf) {
                                    Ok(content) => {
                                        let original_content = content.clone();
                                        let new_content = regex.replace_all(&content, replace_text);
                                        
                                        if new_content != original_content {
                                            // Create backup if requested
                                            if create_backup {
                                                let backup_path = format!("{}.backup", path_buf.to_string_lossy());
                                                if let Err(e) = std::fs::write(&backup_path, &original_content) {
                                                    errors.push(format!("Failed to create backup for '{}': {}", 
                                                        path_buf.to_string_lossy(), e));
                                                    continue;
                                                }
                                            }

                                            // Write modified content
                                            match std::fs::write(&path_buf, new_content.as_ref()) {
                                                Ok(_) => {
                                                    let replacements = regex.find_iter(&original_content).count();
                                                    total_replacements += replacements;
                                                    modified_files.push(serde_json::json!({
                                                        "file": path_buf.to_string_lossy(),
                                                        "replacements": replacements,
                                                        "backup_created": create_backup
                                                    }));
                                                }
                                                Err(e) => {
                                                    errors.push(format!("Failed to write to '{}': {}", 
                                                        path_buf.to_string_lossy(), e));
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        errors.push(format!("Failed to read '{}': {}", 
                                            path_buf.to_string_lossy(), e));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Pattern error: {}", e));
                        }
                    }
                }

                ToolResult {
                    success: errors.is_empty(),
                    data: Some(serde_json::json!({
                        "search_pattern": search_pattern,
                        "replace_text": replace_text,
                        "file_pattern": file_pattern,
                        "case_sensitive": case_sens,
                        "backup_created": create_backup,
                        "modified_files": modified_files,
                        "files_modified": modified_files.len(),
                        "total_replacements": total_replacements,
                        "errors": errors
                    })),
                    error: if errors.is_empty() { None } else { Some(errors.join("; ")) },
                }
            }
            Err(e) => ToolResult {
                success: false,
                data: None,
                error: Some(format!("Invalid file pattern '{}': {}", file_pattern, e)),
            },
        }
    }

    /// Find files by name pattern
    pub fn find_files(name_pattern: &str, base_path: Option<&str>, file_type: Option<&str>) -> ToolResult {
        let search_path = base_path.unwrap_or(".");
        let search_pattern = format!("{}/{}", search_path, name_pattern);
        
        match glob(&search_pattern) {
            Ok(entries) => {
                let mut results = Vec::new();
                
                for entry in entries {
                    match entry {
                        Ok(path_buf) => {
                            let metadata = std::fs::metadata(&path_buf);
                            
                            if let Ok(meta) = metadata {
                                // Filter by file type if specified
                                let include = match file_type {
                                    Some("file") => meta.is_file(),
                                    Some("dir") => meta.is_dir(),
                                    _ => true,
                                };

                                if include {
                                    results.push(serde_json::json!({
                                        "path": path_buf.to_string_lossy(),
                                        "name": path_buf.file_name().unwrap_or_default().to_string_lossy(),
                                        "size": meta.len(),
                                        "is_file": meta.is_file(),
                                        "is_dir": meta.is_dir(),
                                        "modified": meta.modified().ok()
                                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                            .map(|d| d.as_secs())
                                    }));
                                }
                            }
                        }
                        Err(_) => continue,
                    }
                }

                ToolResult {
                    success: true,
                    data: Some(serde_json::json!({
                        "name_pattern": name_pattern,
                        "base_path": search_path,
                        "file_type": file_type,
                        "results": results,
                        "count": results.len()
                    })),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                data: None,
                error: Some(format!("Invalid search pattern '{}': {}", search_pattern, e)),
            },
        }
    }
}

/// Generate OpenRouter-compatible tool definitions
pub fn get_file_system_tools() -> Vec<FileSystemTool> {
    vec![
        // File reading tool
        FileSystemTool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "read_file".to_string(),
                description: "Read the complete contents of a file. Returns the file content, size, and path information.".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: serde_json::json!({
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read. Supports absolute and relative paths."
                        }
                    }),
                    required: vec!["path".to_string()],
                },
            },
        },

        // File writing tool
        FileSystemTool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "write_file".to_string(),
                description: "Write content to a file. Can create new files or overwrite existing ones. Optionally append to existing files.".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: serde_json::json!({
                        "path": {
                            "type": "string",
                            "description": "Path where the file should be written. Parent directories will be created if they don't exist."
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write to the file."
                        },
                        "append": {
                            "type": "boolean",
                            "description": "If true, append content to existing file. If false or omitted, overwrite the file.",
                            "default": false
                        }
                    }),
                    required: vec!["path".to_string(), "content".to_string()],
                },
            },
        },

        // Directory listing tool
        FileSystemTool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "list_directory".to_string(),
                description: "List files and directories in a specified path. Supports wildcard patterns and recursive listing.".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: serde_json::json!({
                        "path": {
                            "type": "string",
                            "description": "Directory path to list. Use '.' for current directory."
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Optional wildcard pattern to filter results (e.g., '*.txt', '*.rs'). If omitted, lists all items."
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "If true, list files recursively in subdirectories.",
                            "default": false
                        }
                    }),
                    required: vec!["path".to_string()],
                },
            },
        },

        // Path creation tool
        FileSystemTool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "create_path".to_string(),
                description: "Create a new file or directory. Parent directories will be created automatically if they don't exist.".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: serde_json::json!({
                        "path": {
                            "type": "string",
                            "description": "Path of the file or directory to create."
                        },
                        "is_directory": {
                            "type": "boolean",
                            "description": "If true, create a directory. If false or omitted, create an empty file.",
                            "default": false
                        }
                    }),
                    required: vec!["path".to_string()],
                },
            },
        },

        // Path deletion tool
        FileSystemTool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "delete_path".to_string(),
                description: "Delete files or directories. Supports wildcard patterns for batch deletion. Use with caution!".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: serde_json::json!({
                        "pattern": {
                            "type": "string",
                            "description": "File or directory path, or wildcard pattern (e.g., '*.tmp', '/tmp/*', 'logs/*.log'). Matches will be deleted."
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "If true, delete directories and all their contents. Required for non-empty directories.",
                            "default": false
                        }
                    }),
                    required: vec!["pattern".to_string()],
                },
            },
        },

        // Grep tool
        FileSystemTool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "grep_files".to_string(),
                description: "Search for text patterns in files using regular expressions. Similar to Unix grep command with advanced features.".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: serde_json::json!({
                        "pattern": {
                            "type": "string",
                            "description": "Regular expression pattern to search for. Use proper regex syntax."
                        },
                        "file_pattern": {
                            "type": "string",
                            "description": "Wildcard pattern for files to search (e.g., '*.rs', '**/*.txt', 'src/**/*.py')."
                        },
                        "case_sensitive": {
                            "type": "boolean",
                            "description": "If true, search is case-sensitive. If false, search is case-insensitive.",
                            "default": true
                        },
                        "line_numbers": {
                            "type": "boolean",
                            "description": "If true, include line numbers in results.",
                            "default": true
                        },
                        "context_lines": {
                            "type": "integer",
                            "description": "Number of context lines to show around each match.",
                            "default": 0,
                            "minimum": 0
                        }
                    }),
                    required: vec!["pattern".to_string(), "file_pattern".to_string()],
                },
            },
        },

        // Search and replace tool
        FileSystemTool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "search_replace".to_string(),
                description: "Search and replace text in files using regular expressions. Supports batch operations across multiple files with backup creation.".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: serde_json::json!({
                        "search_pattern": {
                            "type": "string",
                            "description": "Regular expression pattern to search for. Use proper regex syntax with capture groups if needed."
                        },
                        "replace_text": {
                            "type": "string",
                            "description": "Replacement text. Can include regex capture group references like $1, $2, etc."
                        },
                        "file_pattern": {
                            "type": "string",
                            "description": "Wildcard pattern for files to modify (e.g., '*.txt', 'src/**/*.rs')."
                        },
                        "case_sensitive": {
                            "type": "boolean",
                            "description": "If true, search is case-sensitive. If false, search is case-insensitive.",
                            "default": true
                        },
                        "backup": {
                            "type": "boolean",
                            "description": "If true, create .backup files before modification.",
                            "default": true
                        }
                    }),
                    required: vec!["search_pattern".to_string(), "replace_text".to_string(), "file_pattern".to_string()],
                },
            },
        },

        // File finding tool
        FileSystemTool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "find_files".to_string(),
                description: "Find files and directories by name pattern. Similar to Unix find command with wildcard support.".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: serde_json::json!({
                        "name_pattern": {
                            "type": "string",
                            "description": "Name pattern with wildcards (e.g., '*.rs', 'test*', '**/*.json'). Use ** for recursive search."
                        },
                        "base_path": {
                            "type": "string",
                            "description": "Base directory to start search from. Defaults to current directory.",
                            "default": "."
                        },
                        "file_type": {
                            "type": "string",
                            "description": "Filter by type: 'file' for files only, 'dir' for directories only, or omit for both.",
                            "enum": ["file", "dir"]
                        }
                    }),
                    required: vec!["name_pattern".to_string()],
                },
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_read_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        let content = "Hello, World!";

        // Test write
        let write_result = FileSystemOperations::write_file(file_path_str, content, None);
        assert!(write_result.success);

        // Test read
        let read_result = FileSystemOperations::read_file(file_path_str);
        assert!(read_result.success);
        
        if let Some(data) = read_result.data {
            assert_eq!(data["content"], content);
        }
    }

    #[test]
    fn test_list_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_str().unwrap();
        
        // Create test files
        fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(temp_dir.path().join("file2.rs"), "content2").unwrap();
        
        let result = FileSystemOperations::list_directory(dir_path, Some("*.txt"), None);
        assert!(result.success);
        
        if let Some(data) = result.data {
            let files = data["files"].as_array().unwrap();
            assert_eq!(files.len(), 1);
            assert!(files[0]["name"].as_str().unwrap().contains("file1.txt"));
        }
    }

    #[test]
    fn test_grep_files() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World\nThis is a test\nHello again").unwrap();
        
        let file_pattern = format!("{}/*.txt", temp_dir.path().to_str().unwrap());
        let result = FileSystemOperations::grep_files("Hello", &file_pattern, None, None, None);
        
        assert!(result.success);
        if let Some(data) = result.data {
            assert_eq!(data["total_matches"], 2);
        }
    }
}