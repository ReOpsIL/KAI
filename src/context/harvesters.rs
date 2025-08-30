use crate::cli::config::OpenRouterConfig;
use crate::llm::OpenRouterClient;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// File information collected by the harvester
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub extension: Option<String>,
    pub size: u64,
    pub description: Option<String>,
}

/// Module information for higher-level architecture analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub path: PathBuf,
    pub files: Vec<FileInfo>,
    pub description: Option<String>,
    pub architecture_notes: Option<String>,
}

/// Configuration for the harvester
#[derive(Debug, Clone)]
pub struct HarvesterConfig {
    pub root_path: PathBuf,
    pub exclude_patterns: HashSet<String>,
    pub include_extensions: HashSet<String>,
    pub max_file_size_mb: u64,
    pub openrouter_model: String,
}

impl Default for HarvesterConfig {
    fn default() -> Self {
        let mut exclude_patterns = HashSet::new();
        // Default exclusions for common cache/build directories
        exclude_patterns.insert("target".to_string()); // Rust
        exclude_patterns.insert("node_modules".to_string()); // Node.js
        exclude_patterns.insert("venv".to_string()); // Python virtual env
        exclude_patterns.insert(".venv".to_string()); // Python virtual env
        exclude_patterns.insert("env".to_string()); // Python virtual env
        exclude_patterns.insert(".env".to_string()); // Python virtual env
        exclude_patterns.insert("__pycache__".to_string()); // Python cache
        exclude_patterns.insert(".git".to_string()); // Git
        exclude_patterns.insert(".svn".to_string()); // SVN
        exclude_patterns.insert("build".to_string()); // General build directory
        exclude_patterns.insert("dist".to_string()); // Distribution directory
        exclude_patterns.insert(".gradle".to_string()); // Gradle
        exclude_patterns.insert("bin".to_string()); // Binaries
        exclude_patterns.insert("obj".to_string()); // Object files
        exclude_patterns.insert(".vs".to_string()); // Visual Studio
        exclude_patterns.insert(".vscode".to_string()); // VSCode (optional)
        exclude_patterns.insert(".idea".to_string()); // IntelliJ IDEA (optional)
        exclude_patterns.insert(".context".to_string()); // Our own context directory

        let mut include_extensions = HashSet::new();
        // Common source code and configuration file extensions
        include_extensions.insert("rs".to_string()); // Rust
        include_extensions.insert("py".to_string()); // Python
        include_extensions.insert("js".to_string()); // JavaScript
        include_extensions.insert("ts".to_string()); // TypeScript
        include_extensions.insert("java".to_string()); // Java
        include_extensions.insert("c".to_string()); // C
        include_extensions.insert("cpp".to_string()); // C++
        include_extensions.insert("h".to_string()); // Header files
        include_extensions.insert("hpp".to_string()); // C++ headers
        include_extensions.insert("cs".to_string()); // C#
        include_extensions.insert("go".to_string()); // Go
        include_extensions.insert("rb".to_string()); // Ruby
        include_extensions.insert("php".to_string()); // PHP
        include_extensions.insert("swift".to_string()); // Swift
        include_extensions.insert("kt".to_string()); // Kotlin
        include_extensions.insert("scala".to_string()); // Scala
        include_extensions.insert("clj".to_string()); // Clojure
        include_extensions.insert("hs".to_string()); // Haskell
        include_extensions.insert("ml".to_string()); // ML
        include_extensions.insert("r".to_string()); // R
        include_extensions.insert("m".to_string()); // Objective-C/Matlab
        include_extensions.insert("vue".to_string()); // Vue.js
        include_extensions.insert("jsx".to_string()); // React JSX
        include_extensions.insert("tsx".to_string()); // React TSX

        // Configuration files
        include_extensions.insert("toml".to_string()); // Cargo.toml, etc.
        include_extensions.insert("yaml".to_string()); // YAML configs
        include_extensions.insert("yml".to_string()); // YAML configs
        include_extensions.insert("json".to_string()); // JSON configs
        include_extensions.insert("xml".to_string()); // XML configs
        include_extensions.insert("ini".to_string()); // INI configs
        include_extensions.insert("cfg".to_string()); // Config files
        include_extensions.insert("conf".to_string()); // Config files
        include_extensions.insert("properties".to_string()); // Java properties
        include_extensions.insert("gradle".to_string()); // Gradle build files
        include_extensions.insert("cmake".to_string()); // CMake files
        include_extensions.insert("dockerfile".to_string()); // Docker files
        include_extensions.insert("md".to_string()); // Markdown
        include_extensions.insert("txt".to_string()); // Text files
        include_extensions.insert("makefile".to_string()); // Makefiles

        Self {
            root_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            exclude_patterns,
            include_extensions,
            max_file_size_mb: 5, // 5MB max file size
            openrouter_model: OpenRouterConfig::default().midrange_model,
        }
    }
}

/// The main harvester that traverses files and generates descriptions
pub struct Harvester {
    config: HarvesterConfig,
    openrouter_client: Option<OpenRouterClient>,
}

impl Harvester {
    /// Create a new harvester with the given configuration
    pub fn new(config: HarvesterConfig) -> Self {
        Self {
            config,
            openrouter_client: None,
        }
    }

    /// Create a new harvester with default configuration
    pub fn with_defaults() -> Self {
        Self::new(HarvesterConfig::default())
    }

    /// Set the OpenRouter client for LLM integration
    pub fn with_openrouter(mut self, client: OpenRouterClient) -> Self {
        self.openrouter_client = Some(client);
        self
    }

    /// Add custom exclude patterns to the configuration
    pub fn add_exclude_patterns(mut self, patterns: Vec<String>) -> Self {
        for pattern in patterns {
            self.config.exclude_patterns.insert(pattern);
        }
        self
    }

    /// Add custom include extensions to the configuration
    pub fn add_include_extensions(mut self, extensions: Vec<String>) -> Self {
        for ext in extensions {
            self.config.include_extensions.insert(ext);
        }
        self
    }

    /// Check if a path should be excluded based on the exclusion patterns
    fn should_exclude_path(&self, path: &Path) -> bool {
        for component in path.components() {
            if let Some(name) = component.as_os_str().to_str() {
                if self.config.exclude_patterns.contains(name) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a file should be included based on its extension
    fn should_include_file(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                return self
                    .config
                    .include_extensions
                    .contains(ext_str.to_lowercase().as_str());
            }
        }

        // Special handling for files without extensions (like Dockerfile, Makefile)
        if let Some(filename) = path.file_name() {
            if let Some(name_str) = filename.to_str() {
                let name_lower = name_str.to_lowercase();
                return self.config.include_extensions.contains(&name_lower);
            }
        }

        false
    }

    /// Discover all relevant files in the project directory
    pub fn discover_files(&self) -> Result<Vec<FileInfo>, Box<dyn std::error::Error>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.config.root_path)
            .into_iter()
            .filter_entry(|e| !self.should_exclude_path(e.path()))
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && self.should_include_file(path) {
                let metadata = fs::metadata(path)?;
                let size_mb = metadata.len() / (1024 * 1024);

                // Skip files that are too large
                if size_mb > self.config.max_file_size_mb {
                    continue;
                }

                let relative_path = path
                    .strip_prefix(&self.config.root_path)
                    .unwrap_or(path)
                    .to_path_buf();

                let extension = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| s.to_lowercase());

                files.push(FileInfo {
                    path: path.to_path_buf(),
                    relative_path,
                    extension,
                    size: metadata.len(),
                    description: None,
                });
            }
        }

        Ok(files)
    }

    /// Generate description for a single file using LLM
    async fn generate_file_description(
        &self,
        file_info: &FileInfo,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let client = self
            .openrouter_client
            .as_ref()
            .ok_or("OpenRouter client not configured")?;

        // Read file content
        let content = fs::read_to_string(&file_info.path)?;

        // Create prompt for file description
        let prompt = format!(
            r#"Analyze the following source code file and provide a structured technical description for project context management.

## File Information
- **Path**: {}
- **Extension**: {}
- **Size**: {} bytes
- **Language**: {}

## Analysis Requirements
Provide a comprehensive but concise analysis covering:

### Primary Purpose
- Core functionality and role within the project
- Primary responsibility or domain this file handles

### Technical Structure
- Main classes, interfaces, structs, or modules defined
- Key functions, methods, or endpoints exposed
- Data models, schemas, or types declared
- Configuration or constants defined

### Architecture & Patterns
- Design patterns implemented (MVC, Observer, Factory, etc.)
- Architectural layer (presentation, business logic, data access, etc.)
- Code organization principles followed

### Dependencies & Integration
- External libraries or frameworks used
- Internal modules or files imported/referenced
- Database, API, or service integrations
- Environment or configuration dependencies

### Notable Characteristics
- Performance considerations or optimizations
- Security implementations
- Error handling approaches
- Testing strategies present

### Project Context
- How this file relates to other project components
- Potential impact areas if this file were modified
- Entry points or public interfaces other files might use

## Constraints
- DO NOT reproduce any source code
- Keep each section concise (2-3 sentences max)
- Focus on information useful for project navigation and understanding
- Omit sections that don't apply to this specific file

## Reference
Source file: {}

---
Source code:
{}"#,
            file_info.relative_path.display(),
            file_info.extension.as_deref().unwrap_or("none"),
            file_info.size,
            detect_language_from_extension(file_info.extension.as_deref().unwrap_or("")),
            file_info.relative_path.display(),
            content
        );

        // Helper function you might want to add:
        fn detect_language_from_extension(ext: &str) -> &str {
            match ext.to_lowercase().as_str() {
                "rs" => "Rust",
                "py" => "Python",
                "js" | "mjs" => "JavaScript",
                "ts" => "TypeScript",
                "java" => "Java",
                "cpp" | "cc" | "cxx" => "C++",
                "c" => "C",
                "go" => "Go",
                "rb" => "Ruby",
                "php" => "PHP",
                "cs" => "C#",
                "swift" => "Swift",
                "kt" => "Kotlin",
                "scala" => "Scala",
                "clj" => "Clojure",
                "hs" => "Haskell",
                "ml" => "OCaml",
                "fs" => "F#",
                "dart" => "Dart",
                "elm" => "Elm",
                "ex" | "exs" => "Elixir",
                "erl" => "Erlang",
                "lua" => "Lua",
                "r" => "R",
                "m" => "Objective-C/MATLAB",
                "pl" => "Perl",
                "sh" | "bash" => "Shell Script",
                "ps1" => "PowerShell",
                "sql" => "SQL",
                "html" => "HTML",
                "css" => "CSS",
                "scss" | "sass" => "Sass/SCSS",
                "xml" => "XML",
                "json" => "JSON",
                "yaml" | "yml" => "YAML",
                "toml" => "TOML",
                "md" => "Markdown",
                "dockerfile" => "Dockerfile",
                "makefile" => "Makefile",
                _ => "Unknown",
            }
        }

        let response = client
            .send_prompt(
                &self.config.openrouter_model,
                &prompt,
                Some(500), // Max 500 tokens for description
                Some(0.3), // Lower temperature for more focused descriptions
            )
            .await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err("No response from LLM".into())
        }
    }

    /// Generate descriptions for all files using LLM
    pub async fn generate_file_descriptions(
        &self,
        files: &mut [FileInfo],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.openrouter_client.is_none() {
            return Err("OpenRouter client not configured".into());
        }

        for file_info in files.iter_mut() {
            match self.generate_file_description(file_info).await {
                Ok(description) => {
                    file_info.description = Some(description);
                    println!(
                        "Generated description for: {}",
                        file_info.relative_path.display()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "Failed to generate description for {}: {}",
                        file_info.relative_path.display(),
                        e
                    );
                    // Continue with other files even if one fails
                }
            }

            // Small delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Organize files into modules based on directory structure
    pub fn organize_into_modules(&self, files: Vec<FileInfo>) -> Vec<ModuleInfo> {
        use std::collections::HashMap;

        let mut modules: HashMap<PathBuf, Vec<FileInfo>> = HashMap::new();

        for file_info in files {
            let module_path = if let Some(parent) = file_info.relative_path.parent() {
                parent.to_path_buf()
            } else {
                PathBuf::from(".")
            };

            modules.entry(module_path).or_default().push(file_info);
        }

        modules
            .into_iter()
            .map(|(path, files)| {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("root")
                    .to_string();

                ModuleInfo {
                    name,
                    path,
                    files,
                    description: None,
                    architecture_notes: None,
                }
            })
            .collect()
    }

    /// Generate high-level module descriptions
    pub async fn generate_module_descriptions(
        &self,
        modules: &mut [ModuleInfo],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = self
            .openrouter_client
            .as_ref()
            .ok_or("OpenRouter client not configured")?;

        for module in modules.iter_mut() {
            let file_summaries: Vec<String> = module
                .files
                .iter()
                .filter_map(|f| {
                    f.description.as_ref().map(|desc| {
                        format!(
                            "- {}: {}",
                            f.relative_path.display(),
                            desc.lines().next().unwrap_or("")
                        )
                    })
                })
                .collect();

            if file_summaries.is_empty() {
                continue;
            }

            let prompt = format!(
                r#"Analyze the following module/directory structure and provide a comprehensive architectural overview for project context management.

## Module Information
- **Name**: {}
- **Path**: {}
- **File Count**: {}

## Analysis Requirements
Provide a structured architectural analysis covering:

### Module Purpose & Responsibility
- Primary domain or functional area this module handles
- Core business logic or technical capability provided
- Module's role within the overall system architecture

### Internal Architecture
- How files within this module collaborate and depend on each other
- Data flow patterns between components
- Internal API boundaries and interfaces
- Separation of concerns within the module

### Design Patterns & Principles
- Architectural patterns implemented (Repository, Service Layer, MVC, etc.)
- SOLID principles or other design principles followed
- Code organization strategies (layered architecture, hexagonal, etc.)
- Common abstractions or interfaces used throughout the module

### External Integration
- Dependencies on other modules within the project
- External libraries or frameworks this module relies on
- APIs or services this module integrates with
- Configuration or environment dependencies

### Module Boundaries & Contracts
- Public interfaces exposed to other modules
- Entry points and primary facades
- Data models or types exported for external use
- Events, callbacks, or hooks provided

### Architectural Quality Indicators
- Cohesion level within the module (high/medium/low)
- Coupling with other modules (tight/loose)
- Testability and separation of concerns
- Extensibility and maintainability factors

### System Context
- Upstream modules or systems that depend on this module
- Downstream dependencies this module requires
- Potential ripple effects if this module changes
- Alternative implementations or competing modules

## Constraints
- Focus on architectural and design aspects, not implementation details
- Emphasize relationships and interactions between components
- Keep each section concise (3-4 sentences max)
- Omit sections that don't apply to this specific module
- Prioritize information useful for system understanding and navigation

## Reference
Module path: {}

---
File Details:
{}"#,
                module.name,
                module.path.display(),
                file_summaries.len(),
                module.path.display(),
                file_summaries.join("\n\n---\n\n")
            );

            fn extract_file_name(summary: &str) -> &str {
                // Extract just the filename from the summary for structure display
                summary
                    .lines()
                    .find(|line| line.starts_with("- **Path**:") || line.contains("File:"))
                    .and_then(|line| line.split('/').last())
                    .unwrap_or("Unknown file")
            }

            match client
                .send_prompt(
                    &self.config.openrouter_model,
                    &prompt,
                    Some(300), // Max 300 tokens for module description
                    Some(0.3), // Lower temperature for focused descriptions
                )
                .await
            {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        module.description = Some(choice.message.content.clone());
                        println!("Generated module description for: {}", module.name);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to generate module description for {}: {}",
                        module.name, e
                    );
                }
            }

            // Small delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Run the complete harvesting process
    pub async fn harvest(&self) -> Result<Vec<ModuleInfo>, Box<dyn std::error::Error>> {
        println!("Starting file discovery...");
        let mut files = self.discover_files()?;
        println!("Discovered {} files", files.len());

        if self.openrouter_client.is_some() {
            println!("Generating file descriptions...");
            self.generate_file_descriptions(&mut files).await?;
        }

        println!("Organizing into modules...");
        let mut modules = self.organize_into_modules(files);

        if self.openrouter_client.is_some() {
            println!("Generating module descriptions...");
            self.generate_module_descriptions(&mut modules).await?;
        }

        println!("Harvesting complete!");
        Ok(modules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_harvester_creation() {
        let harvester = Harvester::with_defaults();
        assert!(!harvester.config.exclude_patterns.is_empty());
        assert!(!harvester.config.include_extensions.is_empty());
    }

    #[test]
    fn test_exclusion_patterns() {
        let harvester = Harvester::with_defaults();

        assert!(harvester.should_exclude_path(Path::new("target/debug")));
        assert!(harvester.should_exclude_path(Path::new("node_modules/package")));
        assert!(harvester.should_exclude_path(Path::new("src/.git/config")));
        assert!(!harvester.should_exclude_path(Path::new("src/main.rs")));
    }

    #[test]
    fn test_inclusion_patterns() {
        let harvester = Harvester::with_defaults();

        assert!(harvester.should_include_file(Path::new("main.rs")));
        assert!(harvester.should_include_file(Path::new("config.toml")));
        assert!(harvester.should_include_file(Path::new("Dockerfile")));
        assert!(!harvester.should_include_file(Path::new("binary.exe")));
    }

    #[tokio::test]
    async fn test_file_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create test files
        fs::create_dir_all(temp_path.join("src")).unwrap();
        fs::write(temp_path.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(temp_path.join("Cargo.toml"), "[package]").unwrap();

        // Create excluded directory
        fs::create_dir_all(temp_path.join("target/debug")).unwrap();
        fs::write(temp_path.join("target/debug/binary"), "binary").unwrap();

        let config = HarvesterConfig {
            root_path: temp_path,
            ..Default::default()
        };

        let harvester = Harvester::new(config);
        let files = harvester.discover_files().unwrap();

        assert_eq!(files.len(), 2); // main.rs and Cargo.toml
        assert!(files
            .iter()
            .any(|f| f.relative_path == Path::new("src/main.rs")));
        assert!(files
            .iter()
            .any(|f| f.relative_path == Path::new("Cargo.toml")));
    }
}
