use crate::config::{Config, ConfigManager};
use crate::context::{UnifiedContext, ContextConfig, ExecutionContext};
use crate::execution_stack::{ExecutionStack, StackRequest, StackResponse};
use crate::openrouter::{Message, OpenRouterClient, ModelTier};
use crate::prompts::PromptManager;
use crate::story::StoryLogger;
use crate::tools::ToolExecutor;
use inquire::Autocomplete;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

// Removed complex LLM-based analysis structures

/// Extract JSON content from markdown-wrapped responses
fn extract_json_from_markdown(content: &str) -> String {
    let content = content.trim();
    
    // Check for markdown code blocks with json/JSON language tag
    if content.starts_with("```json") && content.ends_with("```") {
        // Extract content between ```json and ```
        let start = content.find("```json").unwrap() + 7; // length of "```json"
        let end = content.rfind("```").unwrap();
        return content[start..end].trim().to_string();
    }
    
    // Check for plain markdown code blocks
    if content.starts_with("```") && content.ends_with("```") {
        // Extract content between ``` and ```
        let start = content.find("```").unwrap() + 3;
        let end = content.rfind("```").unwrap();
        return content[start..end].trim().to_string();
    }
    
    // Return as-is if no markdown wrapping
    content.to_string()
}

/// Represents conversation context and semantic understanding
#[derive(Debug, Clone)]
pub struct ConversationContext {
    /// Recent messages with semantic importance
    pub important_messages: VecDeque<Message>,
    /// Current conversation thread (what the user is working on)
    pub current_thread: Option<String>,
    /// Tools that are contextually relevant
    pub available_tools: Vec<String>,
    /// Do we need tools for handling prompt
    pub tools_needed: bool,
    /// Conversation state (planning, executing, questioning, etc.)
    pub state: ConversationState,
    /// Working memory for ongoing tasks
    pub working_memory: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConversationState {
    /// User is asking questions or having a discussion
    Conversational,
    /// User needs help planning something
    Planning,
    /// User is actively working on implementation
    Implementing,
    /// User is stuck and needs guidance
    Troubleshooting,
    /// User is exploring/investigating something
    Exploring,
}

/// JSON response schema for LLM conversation state analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConversationStateAnalysis {
    /// Detected conversation state
    state: String,
    /// Confidence level (0.0 to 1.0)
    confidence: f32,
    /// Brief reasoning for the classification
    reasoning: String,
    /// Extracted main topic/thread from the input
    topic: Option<String>,
    /// Suggested tools that might be relevant
    suggested_tools: Vec<String>,
}

/// JSON response schema for LLM criticality analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CriticalityAnalysis {
    /// Whether the messages indicate a critical/high-stakes operation
    is_critical: bool,
    /// Confidence level (0.0 to 1.0)
    confidence: f32,
    /// Brief reasoning for the classification
    reasoning: String,
}

impl Default for ConversationContext {
    fn default() -> Self {
        Self {
            important_messages: VecDeque::with_capacity(20),
            current_thread: None,
            available_tools: vec![
                // "create_file".to_string(),
                // "run_command".to_string(),
                // "read_file".to_string(),
                // "create_directory".to_string(),
            ],
            tools_needed: false,
            state: ConversationState::Conversational,
            working_memory: Vec::new(),
        }
    }
}

/// Semantic conversation engine that adapts to user needs
pub struct SemanticEngine {
    pub openrouter_client: OpenRouterClient,
    pub tool_executor: ToolExecutor,
    pub story_logger: StoryLogger,
    pub config: Config,
    pub working_dir: String,
    pub session_id: String,
    
    /// Unified context management system
    pub unified_context: UnifiedContext,
    
    /// Legacy context for backward compatibility (will be phased out)
    pub context: ConversationContext,
    pub execution_stack: ExecutionStack,
    pub auto_execute_stack: bool,
}

impl SemanticEngine {
    pub async fn new(
        working_dir: String,
        cli_model: Option<String>,
        cli_verbose: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        use crate::config::ConfigManager;
        use uuid::Uuid;

        let mut config = ConfigManager::load_config()?;

        if let Some(model) = cli_model {
            config.openrouter.model = model;
        }

        if cli_verbose {
            config.preferences.verbose = true;
        }

        let openrouter_client = OpenRouterClient::new(config.clone()).await?;
        
        // Simple tier-based model system - no preloading needed
        
        let tool_executor = ToolExecutor::new(working_dir.clone(), config.preferences.verbose);
        let session_id = Uuid::new_v4().to_string();
        let story_logger = StoryLogger::new(working_dir.clone(), session_id.clone());

        // Initialize UnifiedContext
        let context_config = ContextConfig::default();
        let working_path = PathBuf::from(&working_dir);
        let unified_context = UnifiedContext::new(working_path, context_config, session_id.clone()).await?;

        Ok(Self {
            openrouter_client,
            tool_executor,
            story_logger,
            config,
            working_dir,
            session_id,
            unified_context,
            context: ConversationContext::default(),
            execution_stack: ExecutionStack::new(),
            auto_execute_stack: true,
        })
    }

    /// Process a conversation turn with semantic understanding and intelligent task routing
    pub async fn process_conversation(&mut self, user_input: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create user message
        let user_message = Message {
            role: "user".to_string(),
            content: user_input.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        // Add message to unified context with execution context
        let execution_context = Some(ExecutionContext {
            stack_position: None,
            parent_tasks: Vec::new(),
            generated_files: Vec::new(),
            modified_files: Vec::new(),
            command_results: Vec::new(),
            current_phase: Some("user_input".to_string()),
        });

        let _message_id = self.unified_context
            .add_message(user_message.clone(), execution_context)
            .await?;

        // Legacy support - also add to old system
        self.context.important_messages.push_back(user_message);
        self.story_logger.log_user_prompt(user_input);

        // Manage context size (legacy method, will be replaced by unified context management)
        self.manage_context_size();

        // Analyze user intent and update conversation context
        self.analyze_and_update_context(user_input).await?;

        // ===== INTELLIGENT TASK ROUTING =====
        // Determine if this requires multi-step planning or can be handled directly
        let needs_planning = self.requires_multi_step_planning(user_input).await?;
        
        if needs_planning {
            // Use stack-based planning for complex tasks
            self.handle_with_stack_planning(user_input).await?;
        } else {
            // For simple tasks, check if we should use stack execution (for priority/queue management)
            // or handle directly (for immediate response)
            if self.should_use_stack_for_simple_task(user_input) {
                let priority = self.determine_priority(user_input);
                println!("📥 Queuing simple request with priority {}: {}", priority, user_input);
                self.execution_stack.push_user_prompt(user_input.to_string(), priority);
                
                // Start stack processing if enabled
                if self.auto_execute_stack {
                    self.start_stack_execution_with_updates().await?;
                }
            } else {
                // Use direct conversation for immediate simple responses
                self.handle_direct_conversation(user_input).await?;
            }
        }

        Ok(())
    }

    /// Analyze user input and update conversation context using LLM
    async fn analyze_and_update_context(&mut self, user_input: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Use LLM to analyze conversation state
        let analysis = self.analyze_conversation_state_with_llm(user_input).await
            .map_err(|e| format!("❌ Failed to analyze conversation context: {}\n\nPlease try rephrasing your request or check your API connection.", e))?;

        // Parse the detected state
        self.context.state = self.parse_conversation_state(&analysis.state);

        // Update current thread/topic if provided
        if let Some(topic) = analysis.topic {
            if !topic.trim().is_empty() {
                self.context.current_thread = Some(topic);
            }
        }

        // Update available tools based on LLM suggestions
        if !analysis.suggested_tools.is_empty() {
            self.context.available_tools.extend(analysis.suggested_tools.clone());
            self.context.available_tools.sort();
            self.context.available_tools.dedup();
            self.context.tools_needed = analysis.suggested_tools.len() > 0;
        }

        // Add analysis reasoning to working memory for context
        if !analysis.reasoning.trim().is_empty() {
            self.context.working_memory.push(format!("Analysis: {}", analysis.reasoning));
        }

        Ok(())
    }

    /// Create an adaptive system message based on current context
    async fn create_adaptive_system_message(&self) -> Message {
        // Use the unified context to generate a dynamic, context-aware system message
        let unified_system_message = match self.unified_context.generate_dynamic_system_message().await {
            Ok(msg) => msg,
            Err(e) => {
                if self.config.preferences.verbose {
                    println!("⚠️ Failed to generate dynamic system message: {}, using fallback", e);
                }
                // Fallback to basic system message
                PromptManager::get_system_prompt()
            }
        };

        // Start with the unified context system message
        let mut content = unified_system_message;

        // Add working directory context
        content.push_str(&format!("\n\nWorking directory: {}.", self.working_dir));

        // Add active task context if available
        if let Some(active_task) = self.unified_context.execution_history.get_active_task_context() {
            content.push_str(&format!("\n{}", active_task));
        }

        // Add session context
        content.push_str(&format!("\n\nSession ID: {}", self.session_id));
        
        // Add current conversation context
        let message_count = self.unified_context.conversation_history.len();
        if message_count > 0 {
            content.push_str(&format!(
                "\n\nConversation History: {} messages",
                message_count
            ));
        }

        // Add conversation state-specific extensions
        match self.context.state {
            ConversationState::Planning => {
                content.push_str(" PLANNING MODE: You are helping the user plan their approach.
                    Create detailed, step-by-step action plans that break down complex requests into specific, executable actions.
                    Each action should specify exactly which tools to use and what operations to perform.
                    Ask clarifying questions if the request is ambiguous. Focus on creating comprehensive plans before implementation.
                    Use the action plan format from the prompts when creating detailed plans.");
            }
            ConversationState::Implementing => {
                content.push_str(" IMPLEMENTATION MODE: The user is ready to build and implement.
                    Use available tools proactively to execute the necessary steps.
                    Follow systematic approaches: Read files before editing, verify changes after implementation,
                    and use appropriate tools for each operation (Edit, MultiEdit, Write, Bash, etc.).
                    Be thorough in your execution and provide clear feedback about what you're doing.");
            }
            ConversationState::Troubleshooting => {
                content.push_str(" TROUBLESHOOTING MODE: The user is experiencing difficulties.
                    Help them debug issues systematically. First, gather information about the problem,
                    examine relevant files and logs, reproduce the issue if possible,
                    then provide specific solutions. Use discovery tools (Read, Grep, Glob, LS, Bash)
                    to investigate before suggesting fixes.");
            }
            ConversationState::Exploring => {
                content.push_str(" EXPLORATION MODE: The user wants to investigate and understand existing code/systems.
                    Help them explore systematically using discovery tools.
                    Examine code structure, understand implementations, explain findings clearly,
                    and guide them through the codebase. Focus on understanding before making changes.");
            }
            ConversationState::Conversational => {
                content.push_str(" CONVERSATIONAL MODE: Engage in natural conversation while being helpful and responsive.
                    Provide clear explanations, answer questions thoroughly, and be ready to switch modes
                    based on the user's needs. Use tools when necessary to provide accurate information.");
            }
        }

        // Add available tools context
        if !self.context.available_tools.is_empty() {
            content.push_str(&format!(" Available tools: {}.", self.context.available_tools.join(", ")));
        }

        // Add current thread/focus context
        if let Some(thread) = &self.context.current_thread {
            content.push_str(&format!(" Current focus: {}.", thread));
        }

        // Add working memory context
        if !self.context.working_memory.is_empty() {
            let recent_context = self.context.working_memory
                .iter()
                .rev()
                .take(3)
                .rev()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            content.push_str(&format!(" Recent context: {}.", recent_context));
        }

        // Add tool selection guidance based on conversation state
        match self.context.state {
            ConversationState::Planning => {
                content.push_str(" For planning: Focus on understanding requirements first, then create structured plans.
                     Use Read/Grep/LS tools for discovery, then provide detailed action plans.");
            }
            ConversationState::Implementing => {
                content.push_str(" For implementation: Use Edit/MultiEdit for code changes, Write for new files,
                    Bash for commands/tests. Always read files before editing.");
            }
            ConversationState::Troubleshooting => {
                content.push_str(" For troubleshooting: Start with Read/Grep to examine code, Bash to reproduce issues,
                then use Edit tools to fix problems. Verify fixes with additional commands.");
            }
            ConversationState::Exploring => {
                content.push_str(" For exploration: Use Read, Grep, Glob, and LS extensively to understand the codebase.
                Explain what you find and guide the user through the structure.");
            }
            _ => {}
        }

        Message {
            role: "system".to_string(),
            content,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Get relevant conversation context for the LLM
    fn get_relevant_context(&self) -> Vec<Message> {
        // Use unified context to get contextually relevant messages
        // This provides intelligent message selection based on semantic similarity, importance, and recency
        
        // For now, get recent conversation context as query
        let _recent_context = if let Some(recent) = self.context.important_messages.back() {
            &recent.content
        } else {
            "current conversation"
        };
        
        // Get contextually relevant messages from unified context
        let _max_tokens = 8000; // Reserve tokens for system message and user input
        match tokio::runtime::Handle::try_current() {
            Ok(_) => {
                // We're already in an async context, but can't use await here
                // Fall back to legacy method for now
                self.get_legacy_context()
            }
            Err(_) => {
                // Not in async context, use legacy method
                self.get_legacy_context()
            }
        }
    }

    /// Legacy context retrieval method (fallback)
    fn get_legacy_context(&self) -> Vec<Message> {
        self.context.important_messages
            .iter()
            .rev()
            .take(10)
            .rev()
            .cloned()
            .collect()
    }

    /// Get contextually relevant messages using the unified context system
    async fn get_contextual_messages(&mut self, query: &str, max_tokens: usize) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        // Use the unified context to get semantically relevant messages
        self.unified_context
            .get_contextual_messages(query, max_tokens)
            .await
    }

    /// Extract affected files from tool execution results with comprehensive extension support
    fn extract_affected_files_from_result(&self, result: &str) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        // First attempt: Enhanced pattern matching with comprehensive extensions
        files.extend(self.extract_files_with_pattern_matching(result));
        
        // Second attempt: Use LLM for more accurate extraction if pattern matching found few results
        if files.len() < 2 && result.len() > 50 {
            // Use async LLM extraction in background for better accuracy
            let client = self.openrouter_client.clone();
            let result_content = result.to_string();
            
            // Spawn background task for LLM-based extraction
            tokio::spawn(async move {
                match Self::extract_files_with_llm(&client, &result_content).await {
                    Ok(llm_files) => {
                        if !llm_files.is_empty() {
                            println!("🧠 LLM extracted {} additional files from tool result", llm_files.len());
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️ LLM file extraction failed: {}", e);
                    }
                }
            });
        }
        
        // Remove duplicates and invalid paths
        files.sort();
        files.dedup();
        files.into_iter()
            .filter(|path| self.is_valid_file_path(path))
            .collect()
    }
    
    /// Enhanced pattern matching with comprehensive software engineering extensions
    fn extract_files_with_pattern_matching(&self, result: &str) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        // Comprehensive list of software engineering file extensions
        let extensions = [
            // Programming Languages
            ".rs", ".py", ".js", ".ts", ".jsx", ".tsx", ".java", ".kt", ".scala",
            ".c", ".cpp", ".cc", ".cxx", ".h", ".hpp", ".hxx", ".cs", ".fs", ".fsx",
            ".go", ".php", ".rb", ".swift", ".m", ".mm", ".dart", ".lua", ".pl",
            ".r", ".jl", ".elm", ".clj", ".cljs", ".hs", ".ml", ".mli", ".erl",
            ".ex", ".exs", ".nim", ".cr", ".zig", ".odin", ".v", ".d", ".pas",
            
            // Web Technologies
            ".html", ".htm", ".css", ".scss", ".sass", ".less", ".vue", ".svelte",
            ".astro", ".mdx", ".pug", ".jade", ".ejs", ".hbs", ".mustache",
            
            // Configuration & Data
            ".json", ".yaml", ".yml", ".toml", ".ini", ".cfg", ".conf", ".config",
            ".xml", ".plist", ".properties", ".env", ".envrc", ".editorconfig",
            
            // Documentation
            ".md", ".rst", ".txt", ".adoc", ".asciidoc", ".org", ".tex", ".rtf",
            
            // Database & Query
            ".sql", ".psql", ".mysql", ".sqlite", ".db", ".mdb", ".cypher",
            ".graphql", ".gql", ".prisma",
            
            // Build & Deployment
            ".dockerfile", ".containerfile", ".docker-compose.yml", ".docker-compose.yaml",
            ".makefile", ".cmake", ".gradle", ".sbt", ".pom.xml", ".package.json",
            ".cargo.toml", ".setup.py", ".requirements.txt", ".pipfile", ".poetry.lock",
            ".yarn.lock", ".package-lock.json", ".gemfile", ".gemfile.lock",
            
            // Infrastructure & DevOps
            ".tf", ".tfvars", ".hcl", ".nomad", ".consul", ".k8s", ".kube",
            ".helm", ".ansible", ".playbook", ".inventory", ".vagrant",
            ".pulumi", ".serverless", ".sam", ".cloudformation",
            
            // Shell & Scripts
            ".sh", ".bash", ".zsh", ".fish", ".ps1", ".psm1", ".bat", ".cmd",
            ".awk", ".sed", ".vim", ".vimrc", ".tmux", ".bashrc", ".zshrc",
            
            // Mobile Development
            ".swift", ".m", ".mm", ".kt", ".java", ".dart", ".xaml", ".storyboard",
            ".xib", ".plist", ".gradle", ".pbxproj", ".xcconfig", ".entitlements",
            
            // Game Development
            ".cs", ".cpp", ".h", ".hlsl", ".glsl", ".shader", ".mat", ".prefab",
            ".unity", ".unreal", ".gd", ".tres", ".tscn", ".godot",
            
            // Data Science & ML
            ".ipynb", ".py", ".r", ".rmd", ".jl", ".scala", ".mat", ".csv",
            ".tsv", ".parquet", ".hdf5", ".pkl", ".npy", ".npz", ".arrow",
            
            // Testing
            ".test.js", ".test.ts", ".spec.js", ".spec.ts", ".test.py", ".spec.py",
            ".test.rs", ".feature", ".scenario", ".testcase", ".junit",
            
            // Miscellaneous
            ".log", ".lock", ".pid", ".tmp", ".temp", ".bak", ".orig", ".patch",
            ".diff", ".tar", ".gz", ".zip", ".rar", ".7z", ".deb", ".rpm",
            ".dmg", ".pkg", ".msi", ".exe", ".app", ".ipa", ".apk", ".aab"
        ];
        
        // Create a set for faster lookup
        let extension_set: std::collections::HashSet<&str> = extensions.iter().cloned().collect();
        
        for line in result.lines() {
            // Skip very long lines (likely not containing file paths)
            if line.len() > 500 {
                continue;
            }
            
            // Look for file path patterns
            if line.contains('/') || line.contains('\\') {
                // Split on various delimiters to find potential paths
                let delimiters = [' ', '\t', '\'', '"', '`', '(', ')', '[', ']', '{', '}', '<', '>', ',', ';', ':', '|'];
                let mut words = Vec::new();
                let mut current_word = String::new();
                
                for ch in line.chars() {
                    if delimiters.contains(&ch) {
                        if !current_word.is_empty() {
                            words.push(current_word.clone());
                            current_word.clear();
                        }
                    } else {
                        current_word.push(ch);
                    }
                }
                if !current_word.is_empty() {
                    words.push(current_word);
                }
                
                for word in words {
                    if self.looks_like_file_path(&word, &extension_set) {
                        let cleaned_path = self.clean_file_path(&word);
                        if !cleaned_path.is_empty() {
                            files.push(PathBuf::from(cleaned_path));
                        }
                    }
                }
            }
        }
        
        files
    }
    
    /// Check if a word looks like a file path
    fn looks_like_file_path(&self, word: &str, extension_set: &std::collections::HashSet<&str>) -> bool {
        // Must contain path separator
        if !word.contains('/') && !word.contains('\\') {
            return false;
        }
        
        // Must have reasonable length
        if word.len() < 3 || word.len() > 300 {
            return false;
        }
        
        // Check for known extensions
        for ext in extension_set {
            if word.ends_with(ext) {
                return true;
            }
        }
        
        // Check for common patterns even without specific extensions
        let word_lower = word.to_lowercase();
        let path_indicators = [
            "/src/", "/lib/", "/bin/", "/test/", "/tests/", "/spec/", "/docs/", "/doc/",
            "/config/", "/configs/", "/scripts/", "/build/", "/dist/", "/target/",
            "/node_modules/", "/vendor/", "/third_party/", "/external/", "/deps/",
            ".git/", ".github/", ".vscode/", ".idea/", "Cargo.toml", "package.json",
            "Dockerfile", "Makefile", "CMakeLists.txt", "build.gradle", "pom.xml"
        ];
        
        for indicator in &path_indicators {
            if word_lower.contains(indicator) {
                return true;
            }
        }
        
        false
    }
    
    /// Clean file path by removing common prefixes and suffixes
    fn clean_file_path(&self, path: &str) -> String {
        let mut cleaned = path.to_string();
        
        // Remove common prefixes
        let prefixes = ["file://", "http://", "https://", "./", "../"];
        for prefix in &prefixes {
            if cleaned.starts_with(prefix) {
                cleaned = cleaned[prefix.len()..].to_string();
                break;
            }
        }
        
        // Remove common suffixes
        let suffixes = [",", ";", ":", ")", "]", "}", ">", "'", "\"", "`"];
        for suffix in &suffixes {
            if cleaned.ends_with(suffix) {
                cleaned = cleaned[..cleaned.len() - suffix.len()].to_string();
            }
        }
        
        // Remove line numbers and column indicators
        if let Some(colon_pos) = cleaned.rfind(':') {
            let after_colon = &cleaned[colon_pos + 1..];
            if after_colon.chars().all(|c| c.is_ascii_digit()) {
                cleaned = cleaned[..colon_pos].to_string();
            }
        }
        
        cleaned.trim().to_string()
    }
    
    /// Validate if path looks reasonable
    fn is_valid_file_path(&self, path: &PathBuf) -> bool {
        let path_str = path.to_string_lossy();
        
        // Skip obviously invalid paths
        if path_str.is_empty() 
            || path_str.len() > 300 
            || path_str.contains('\0')
            || path_str.contains("..//")
            || path_str.starts_with("http")
            || path_str.starts_with("ftp") {
            return false;
        }
        
        // Must look like a reasonable file path
        path_str.contains('/') || path_str.contains('\\')
    }
    
    /// Extract files using LLM for more accurate parsing
    async fn extract_files_with_llm(
        client: &OpenRouterClient,
        result: &str
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let system_prompt = r#"You are an expert at extracting file paths from tool execution output and logs.

Analyze the provided tool execution result and extract ALL file paths mentioned. Respond with ONLY a JSON object:

{
  "files": [
    {"path": "src/main.rs", "confidence": 0.95},
    {"path": "tests/integration_test.py", "confidence": 0.85},
    {"path": "config/database.yml", "confidence": 0.90}
  ]
}

EXTRACTION RULES:
1. Look for any text that appears to be a file or directory path
2. Include absolute paths (starting with /) and relative paths
3. Include paths with various extensions (.rs, .py, .js, .json, .md, etc.)
4. Include configuration files (Dockerfile, Makefile, package.json, etc.)
5. Include paths mentioned in error messages, logs, and output
6. Include both source files and generated files
7. Remove line numbers, column numbers, and other suffixes (file.rs:123 → file.rs)
8. Assign confidence based on how certain you are it's a real file path

CONFIDENCE LEVELS:
- 0.9-1.0: Clearly a file path (has extension, looks like valid path structure)
- 0.7-0.89: Likely a file path (matches common patterns)
- 0.5-0.69: Possibly a file path (ambiguous but could be)
- Below 0.5: Skip (too uncertain)

IGNORE:
- URLs (http://, https://, ftp://)
- Binary data or encoded strings
- Very long strings (>300 chars)
- Strings with unusual characters that don't look like paths

Extract from error messages, success messages, file listings, diff output, build logs, test output, etc.

Respond with ONLY the JSON object, no other text."#;

        let messages = vec![
            crate::openrouter::Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            crate::openrouter::Message {
                role: "user".to_string(),
                content: format!("Extract file paths from this tool execution result:\n\n{}", 
                    result.chars().take(2000).collect::<String>()), // Limit to avoid token overflow
                tool_calls: None,
                tool_call_id: None,
            }
        ];

        let config = crate::config::ConfigManager::load_config()?;
        let response = client.chat_completion_with_model(
            messages, 
            Some(config.openrouter.simple_model), 
            false
        ).await?;
        
        let content = &response.choices[0].message.content;
        let json_content = extract_json_from_markdown(content);
        
        #[derive(serde::Deserialize)]
        struct FileExtraction {
            files: Vec<FileWithConfidence>,
        }
        
        #[derive(serde::Deserialize)]
        struct FileWithConfidence {
            path: String,
            confidence: f64,
        }
        
        let extraction: FileExtraction = serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse LLM file extraction: {}", e))?;
        
        // Filter by confidence and convert to PathBuf
        let files: Vec<PathBuf> = extraction.files
            .into_iter()
            .filter(|f| f.confidence >= 0.6) // Only include reasonably confident extractions
            .map(|f| PathBuf::from(f.path))
            .collect();
        
        Ok(files)
    }

    /// Manage context size by pruning old messages (legacy method, now handled by unified context)
    fn manage_context_size(&mut self) {
        // Legacy context management - the unified context handles this more intelligently
        
        // Prune important messages queue (legacy support)
        while self.context.important_messages.len() > 20 {
            self.context.important_messages.pop_front();
        }

        // Prune working memory (legacy support)
        while self.context.working_memory.len() > 10 {
            self.context.working_memory.remove(0);
        }
    }

    /// Update working memory with insights from assistant responses using LLM analysis
    fn update_working_memory(&mut self, content: &str) {
        // Use LLM to extract meaningful insights from assistant responses
        // Note: This is async, so we'll update memory later via background task
        let client = self.openrouter_client.clone();
        let content = content.to_string();
        
        tokio::spawn(async move {
            if let Err(e) = Self::analyze_response_for_memory_insight(&client, &content).await {
                eprintln!("⚠️  Working memory analysis failed: {}", e);
            }
        });
        
        // For now, don't add anything to memory synchronously
        // The LLM analysis will be more accurate than hardcoded patterns
    }

    /// Analyze assistant response to extract meaningful memory insights
    async fn analyze_response_for_memory_insight(
        client: &OpenRouterClient, 
        content: &str
    ) -> Result<String, Box<dyn std::error::Error>> {
        let system_prompt = r#"You are an expert at analyzing AI assistant responses to extract key insights for working memory.

Analyze the assistant's response and extract the most important insight or action that should be remembered. Respond with ONLY a JSON object:

{
  "insight": "Brief, actionable insight (max 50 chars)",
  "category": "one of: creation, problem, planning, analysis, completion, other",
  "importance": 0.8
}

Categories:
- creation: Something was built or created
- problem: Issue, error, or failure encountered  
- planning: Next steps or planning discussion
- analysis: Investigation or understanding gained
- completion: Task or step completed successfully
- other: General insight worth remembering

Extract the most important actionable insight. If no significant insight, return null for insight.

Respond with ONLY the JSON object."#;

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "user".to_string(),
                content: format!("Analyze this assistant response: \"{}\"", content.chars().take(500).collect::<String>()),
                tool_calls: None,
                tool_call_id: None,
            }
        ];

        let config = ConfigManager::load_config()?;
        let response = client.chat_completion_with_model(messages, Some(config.openrouter.simple_model), false).await?;
        let analysis_content = &response.choices[0].message.content;

        let json_content = extract_json_from_markdown(analysis_content);
        let analysis: serde_json::Value = serde_json::from_str(&json_content)?;
        
        if let Some(insight) = analysis["insight"].as_str() {
            if !insight.is_empty() && insight != "null" {
                return Ok(insight.to_string());
            }
        }
        
        Err("No significant insight found".into())
    }

    /// Execute tools with semantic awareness
    async fn execute_tools_semantically(
        &mut self,
        tool_calls: &[crate::openrouter::ToolCall],
        conversation_messages: &mut Vec<Message>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.config.preferences.verbose || tool_calls.len() > 1 {
            println!("🤖 Making {} tool calls", tool_calls.len());
        }

        for tool_call in tool_calls {
            if self.config.preferences.verbose {
                println!("  🔧 Executing: {}", tool_call.function.name);
                println!("     Args: {}", tool_call.function.arguments);
            }

            // Log tool execution
            let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                .unwrap_or(serde_json::Value::Null);
            self.story_logger.log_tool_execution(&tool_call.function.name, &args);

            match self.tool_executor.execute_tool_call(tool_call).await {
                Ok(result) => {
                    if self.config.preferences.verbose {
                        println!("  ✅ Success: {}", result);
                    } else {
                        println!("  ✅");
                    }

                    // Update working memory based on tool results
                    self.update_memory_from_tool_result(&tool_call.function.name, &result);

                    // Log tool result
                    self.story_logger.log_tool_result(&tool_call.function.name, true, &result);

                    // Integrate tool result with unified context
                    let affected_files = self.extract_affected_files_from_result(&result);
                    if let Err(e) = self.unified_context
                        .integrate_tool_result(&tool_call.id, &result, affected_files)
                        .await 
                    {
                        if self.config.preferences.verbose {
                            println!("  ⚠️ Warning: Failed to integrate tool result with context: {}", e);
                        }
                    }

                    // Create tool response message
                    let tool_message = Message {
                        role: "tool".to_string(),
                        content: result.clone(),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    };
                    conversation_messages.push(tool_message.clone());
                    // Note: tool message already added to unified context above

                    // Also add tool message to unified context
                    let execution_context = Some(ExecutionContext {
                        stack_position: None,
                        parent_tasks: Vec::new(),
                        generated_files: Vec::new(),
                        modified_files: Vec::new(),
                        command_results: Vec::new(),
                        current_phase: Some("tool_execution".to_string()),
                    });
                    
                    if let Err(e) = self.unified_context
                        .add_message(tool_message, execution_context)
                        .await 
                    {
                        if self.config.preferences.verbose {
                            println!("  ⚠️ Warning: Failed to add tool message to context: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("  ❌ Error: {}", e);

                    // Log tool error
                    self.story_logger.log_tool_result(&tool_call.function.name, false, &e.to_string());

                    // Create error tool response
                    let error_message = Message {
                        role: "tool".to_string(),
                        content: serde_json::json!({"status": "error", "message": e.to_string()}).to_string(),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    };
                    conversation_messages.push(error_message.clone());
                    // Note: error message should also be added to unified context (could be implemented)
                }
            }
        }

        if self.config.preferences.verbose && !tool_calls.is_empty() {
            println!(); // Add spacing after tool execution
        }

        Ok(())
    }

    /// Analyze conversation state using LLM with structured JSON response
    async fn analyze_conversation_state_with_llm(&self, user_input: &str) -> Result<ConversationStateAnalysis, Box<dyn std::error::Error>> {
        let system_prompt = r#"You are an expert at analyzing user intent and conversation context for a coding assistant. 

Analyze the user's input and determine their conversation state. Respond with ONLY a JSON object matching this exact schema:

{
  "state": "one of: Conversational, Planning, Implementing, Troubleshooting, Exploring",
  "confidence": 0.85,
  "reasoning": "Brief explanation of why you chose this state",
  "topic": "Main topic or focus area (optional, can be null)",
  "suggested_tools": ["array of potentially useful tool names"]
}

Conversation States:
- Conversational: General discussion, questions, or casual interaction
- Planning: User wants to plan, design, or think through an approach
- Implementing: User is ready to build, create, or implement something specific
- Troubleshooting: User has problems, errors, or things not working
- Exploring: User wants to investigate, understand, or explore existing code/systems

Common tools: ["create_file", "run_command", "read_file", "create_directory", "git_status", "git_add", "git_commit", "run_tests", "create_test", "package_install", "dependency_check", "database_query", "database_migration"]

Respond with ONLY the JSON object, no other text."#;

        let analysis_message = Message {
            role: "user".to_string(),
            content: format!("Analyze this user input: \"{}\"", user_input),
            tool_calls: None,
            tool_call_id: None,
        };

        let system_message = Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        let messages = vec![system_message, analysis_message];

        // Use configured analysis model for conversation state analysis - better at following JSON schemas
        let response = self.openrouter_client.chat_completion_with_model(messages, Some(self.config.openrouter.simple_model.clone()), false).await?;
        let content = &response.choices[0].message.content;

        // Extract JSON from markdown-wrapped response if needed
        let json_content = extract_json_from_markdown(content);
        
        // Parse JSON response
        let analysis: ConversationStateAnalysis = serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse LLM response as JSON: {}. Response was: {}", e, content))?;

        Ok(analysis)
    }

    /// Analyze whether messages indicate critical/high-stakes operations using LLM
    async fn analyze_criticality_with_llm(&self, messages: &[Message]) -> Result<bool, Box<dyn std::error::Error>> {
        let system_prompt = r#"You are an expert at analyzing whether user messages indicate critical, high-stakes, or production-related operations that require extra care and the most capable AI models.

Analyze the provided messages and determine if they indicate operations that are:
- Production/deployment related
- Security sensitive
- Critical system changes
- High-stakes business operations
- Operations requiring extreme precision and care

Respond with ONLY a JSON object matching this exact schema:

{
  "is_critical": true,
  "confidence": 0.85,
  "reasoning": "Brief explanation of why this is or isn't critical"
}

Consider these indicators of criticality:
- Production, deployment, live system references
- Security, authentication, authorization concerns
- Critical, important, urgent language
- Financial, legal, compliance implications
- Database migrations, schema changes
- Infrastructure changes
- User-facing features in production
- API changes affecting external systems

Respond with ONLY the JSON object, no other text."#;

        // Combine message contents for analysis
        let combined_content = messages.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let analysis_message = Message {
            role: "user".to_string(),
            content: format!("Analyze these messages for criticality:\n\n{}", combined_content),
            tool_calls: None,
            tool_call_id: None,
        };

        let system_message = Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        let analysis_messages = vec![system_message, analysis_message];

        // Use configured analysis model for criticality analysis - better at following JSON schemas
        let response = self.openrouter_client.chat_completion_with_model(analysis_messages, Some(self.config.openrouter.simple_model.clone()), false).await?;
        let content = &response.choices[0].message.content;

        // Extract JSON from markdown-wrapped response if needed
        let json_content = extract_json_from_markdown(content);
        
        // Parse JSON response
        let analysis: CriticalityAnalysis = serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse criticality analysis as JSON: {}. Response was: {}", e, content))?;

        if self.config.preferences.verbose {
            println!("🎯 Criticality assessment: {} (confidence: {:.1}%) - {}", 
                if analysis.is_critical { "CRITICAL" } else { "Normal" },
                analysis.confidence * 100.0,
                analysis.reasoning
            );
        }

        Ok(analysis.is_critical)
    }

    /// Parse string state to ConversationState enum
    fn parse_conversation_state(&self, state_str: &str) -> ConversationState {
        match state_str.to_lowercase().as_str() {
            "planning" => {
                println!("📋 Planning mode activated - Ready to design and strategize your approach");
                ConversationState::Planning
            },
            "implementing" => {
                println!("🔨 Implementation mode activated - Time to build and create");
                ConversationState::Implementing
            },
            "troubleshooting" => {
                println!("🔧 Troubleshooting mode activated - Let's debug and solve problems");
                ConversationState::Troubleshooting
            },
            "exploring" => {
                println!("🔍 Exploration mode activated - Investigating and understanding the codebase");
                ConversationState::Exploring
            },
            _ => {
                println!("💬 Conversational mode activated - Ready to discuss and answer questions");
                ConversationState::Conversational
            }
        }
    }


    /// Select the most appropriate model based on conversation state
    fn select_model_for_conversation_state(&self, _messages: &[Message]) -> Option<String> {
        let tier = match self.context.state {
            ConversationState::Conversational => ModelTier::Simple,
            ConversationState::Exploring => ModelTier::Simple, // Reading/analyzing existing code
            ConversationState::Planning => ModelTier::MidRange, // Need good reasoning but not generation
            ConversationState::Troubleshooting => ModelTier::MidRange, // Debugging requires good analysis
            ConversationState::Implementing => ModelTier::Advanced, // Code generation needs capability
        };
        
        let model_id = self.openrouter_client.get_model_for_tier(&tier);
        
        if self.config.preferences.verbose {
            println!("🧠 Model selection: Using {:?} tier -> {}", tier, model_id);
        }
        
        Some(model_id)
    }

    /// Update working memory based on tool results using LLM analysis
    fn update_memory_from_tool_result(&mut self, tool_name: &str, result: &str) {
        // Use LLM to analyze tool results for better memory insights
        let client = self.openrouter_client.clone();
        let tool_name = tool_name.to_string();
        let result = result.to_string();
        
        tokio::spawn(async move {
            if let Err(e) = Self::analyze_tool_result_for_memory(&client, &tool_name, &result).await {
                eprintln!("⚠️  Tool memory analysis failed: {}", e);
            }
        });
        
        // For now, don't add hardcoded memory entries
        // The LLM analysis will be more accurate and contextual
    }

    /// Analyze tool execution result to extract memory-worthy insights
    async fn analyze_tool_result_for_memory(
        client: &OpenRouterClient,
        tool_name: &str,
        result: &str
    ) -> Result<String, Box<dyn std::error::Error>> {
        let system_prompt = r#"You are an expert at analyzing tool execution results to extract key insights for working memory.

Analyze the tool execution and its result to extract the most important insight. Respond with ONLY a JSON object:

{
  "insight": "Brief insight about what happened (max 60 chars)",
  "success": true,
  "significance": 0.7
}

Consider:
- Did the tool achieve its intended purpose?
- Were there any errors or issues?
- What was actually accomplished?
- Is this worth remembering for context?

Extract insights like:
- "Created user.py with authentication logic"
- "Command failed: missing dependency"
- "Successfully read config.json settings"
- "Database connection established"

If the result is routine/unimportant, return null for insight.

Respond with ONLY the JSON object."#;

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "user".to_string(),
                content: format!(
                    "Tool: {}\nResult: {}", 
                    tool_name, 
                    result.chars().take(300).collect::<String>()
                ),
                tool_calls: None,
                tool_call_id: None,
            }
        ];

        let config = ConfigManager::load_config()?;
        let response = client.chat_completion_with_model(messages, Some(config.openrouter.simple_model), false).await?;
        let analysis_content = &response.choices[0].message.content;

        let json_content = extract_json_from_markdown(analysis_content);
        let analysis: serde_json::Value = serde_json::from_str(&json_content)?;
        
        if let Some(insight) = analysis["insight"].as_str() {
            if !insight.is_empty() && insight != "null" {
                return Ok(insight.to_string());
            }
        }
        
        Err("No significant insight from tool result".into())
    }

    /// Clear conversation context (semantic equivalent of /clear)
    pub fn clear_context(&mut self) -> String {
        let message_count = self.unified_context.conversation_history.len().saturating_sub(1);

        // Clear unified context (this replaces the old messages clearing)
        // We would need to implement a clear method in UnifiedContext
        // For now, we'll clear the legacy context only

        // Reset conversation context
        self.context = ConversationContext::default();

        format!("🧹 Conversation context cleared ({} messages removed)", message_count)
    }

    /// Change model (semantic equivalent of /model)
    pub async fn change_model(&mut self, new_model: &str) -> Result<String, Box<dyn std::error::Error>> {
        let old_model = self.config.openrouter.model.clone();

        if self.config.preferences.verbose {
            println!("🔄 MODEL CHANGE:");
            println!("   From: {}", old_model);
            println!("   To: {}", new_model);
            println!("   Reason: User requested model change");
        }
        self.config.openrouter.model = new_model.to_string();

        match crate::openrouter::OpenRouterClient::new(self.config.clone()).await {
            Ok(new_client) => {
                self.openrouter_client = new_client;
                Ok(format!("✅ Model changed from '{}' to '{}'", old_model, new_model))
            }
            Err(e) => {
                if self.config.preferences.verbose {
                    println!("🔄 MODEL CHANGE:");
                    println!("   From: {}", new_model);
                    println!("   To: {}", old_model);
                    println!("   Reason: Reverting failed model change");
                }
                self.config.openrouter.model = old_model;
                Err(format!("Failed to switch to model '{}': {}", new_model, e).into())
            }
        }
    }

    /// List available models (now shows tier-based models)
    pub async fn list_models(&self, search_term: &str) -> Result<String, Box<dyn std::error::Error>> {
        let models = vec![
            ("Tier 1 - Simple", &self.config.openrouter.simple_model),
            ("Tier 2 - MidRange", &self.config.openrouter.midrange_model),
            ("Tier 3 - Advanced", &self.config.openrouter.advanced_model),
            ("Tier 4 - Critical", &self.config.openrouter.critical_model),
        ];
        
        let mut result = String::from("📋 Available Model Tiers:\n");
        
        for (tier_name, model_id) in &models {
            if search_term.is_empty() || model_id.to_lowercase().contains(&search_term.to_lowercase()) {
                let current = if **model_id == self.config.openrouter.model { " (current)" } else { "" };
                result.push_str(&format!("  {} - {}{}\n", tier_name, model_id, current));
            }
        }
        
        result.push_str(&format!("\n🤖 Current model: {}\n", self.config.openrouter.model));
        result.push_str("💡 Use /tier <1-4> to switch model tiers quickly\n");
        
        Ok(result)
    }

    // ==================== STACK PLANNING METHODS ====================
    
    /// Check if a request requires multi-step planning using LLM analysis
    async fn requires_multi_step_planning(&self, input: &str) -> Result<bool, Box<dyn std::error::Error>> {
        // Use existing conversation state analysis to determine complexity
        match self.context.state {
            ConversationState::Planning => return Ok(true),  // Explicit planning mode
            ConversationState::Conversational | ConversationState::Exploring => return Ok(false), // Simple tasks
            _ => {} // Continue LLM analysis for Implementing/Troubleshooting
        }

        // Use LLM to analyze task complexity
        let system_prompt = r#"You are an expert at analyzing programming tasks to determine if they require multi-step hierarchical planning or can be handled with direct tool execution.

Analyze the user's request and determine if it requires complex multi-step planning. Respond with ONLY a JSON object matching this exact schema:

{
  "requires_planning": true,
  "confidence": 0.85,
  "reasoning": "Brief explanation of why this task does or doesn't need multi-step planning",
  "estimated_steps": 5
}

Tasks that REQUIRE multi-step planning:
- Building complete applications or systems
- Full-stack development projects
- Complex refactoring across multiple files
- System architecture changes
- Deployment pipelines
- Migration projects
- End-to-end workflows
- Projects involving multiple technologies
- Tasks with many interdependent steps

Tasks that can be handled DIRECTLY:
- Single file edits
- Simple bug fixes
- Running individual commands
- Reading/analyzing existing code
- Simple configuration changes
- One-off tool executions
- Basic questions or explanations

Consider the scope, complexity, and number of steps involved. Be conservative - when in doubt, prefer direct execution over planning.

Respond with ONLY the JSON object, no other text."#;

        let analysis_message = Message {
            role: "user".to_string(),
            content: format!("Analyze this programming task: \"{}\"", input),
            tool_calls: None,
            tool_call_id: None,
        };

        let system_message = Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        let messages = vec![system_message, analysis_message];

        // Use configured analysis model for task complexity analysis - better at following JSON schemas
        let response = self.openrouter_client.chat_completion_with_model(messages, Some(self.config.openrouter.simple_model.clone()), false).await?;
        let content = &response.choices[0].message.content;

        // Extract JSON from markdown-wrapped response if needed
        let json_content = extract_json_from_markdown(content);
        
        // Parse JSON response
        let analysis: serde_json::Value = serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse task complexity analysis as JSON: {}. Response was: {}", e, content))?;

        let requires_planning = analysis["requires_planning"].as_bool().unwrap_or(false);
        let confidence = analysis["confidence"].as_f64().unwrap_or(0.5);
        let reasoning = analysis["reasoning"].as_str().unwrap_or("No reasoning provided");

        if self.config.preferences.verbose {
            println!("🧠 Task complexity analysis: {} (confidence: {:.1}%) - {}", 
                if requires_planning { "NEEDS PLANNING" } else { "DIRECT EXECUTION" },
                confidence * 100.0,
                reasoning
            );
        }

        Ok(requires_planning)
    }

    /// Handle complex tasks using stack-based planning
    pub async fn handle_with_stack_planning(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🎯 Complex task detected - using hierarchical planning approach");

        // Generate action plan using the plan command system
        use crate::commands::PlanCommand;
        let plan_cmd = PlanCommand::new();
        
        // Generate action plan and get parsed result directly
        match plan_cmd.execute_and_parse(input).await {
            Ok(action_plan) => {
                // Display the plan
                println!("🎯 Generated Action Plan:\n\n{}", action_plan);
                
                // Push to execution stack
                println!("\n📋 Converting plan to execution stack...");
                let request_ids = self.execution_stack.push_action_plan(action_plan, None);
                println!("✅ Added {} action items to execution stack", request_ids.len());
                
                // Start stack execution
                if self.auto_execute_stack {
                    println!("\n🚀 Starting recursive execution...");
                    self.start_stack_execution_with_updates().await?;
                } else {
                    println!("💡 Stack execution disabled. Use 'execute stack' to run manually.");
                }
            }
            Err(e) => {
                return Err(format!("❌ Failed to generate action plan for complex task: {}\n\nThis appears to be a multi-step task that requires planning, but the planning system encountered an error. Please try:\n1. Simplifying your request\n2. Breaking it into smaller steps\n3. Checking your API connection\n4. Trying again in a moment", e).into());
            }
        }

        Ok(())
    }

    /// Execute stack with progress updates
    pub async fn start_stack_execution_with_updates(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.execution_stack.has_pending_requests() {
            return Ok(());
        }

        println!("🔄 Starting recursive execution stack processing...");
        println!("{}", self.execution_stack.get_status_summary());

        while self.execution_stack.has_pending_requests() {
            if let Some(request) = self.execution_stack.pop_request() {
                println!("\n🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯");
                println!("\n🎯 Processing request: {}", self.get_request_description(&request));
                println!("\n🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯🎯");

                // Mark request as started
                self.execution_stack.start_processing(request.clone());
                
                // Process the request (simplified version)
                match self.process_stack_request(request).await {
                    Ok(response) => {
                        println!("✅ Request completed successfully");
                        println!("{:?}",response);
                        self.execution_stack.push_response(response);
                    }
                    Err(e) => {
                        println!("❌ Request failed: {}", e);
                        // Create error response
                        let error_response = StackResponse {
                            request_id: format!("error_{}", uuid::Uuid::new_v4()),
                            success: false,
                            content: format!("Error: {}", e),
                            generated_requests: Vec::new(),
                            completed_actions: Vec::new(),
                        };
                        self.execution_stack.push_response(error_response);
                    }
                }

                // Small delay to prevent overwhelming
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }

        println!("\n🎉 Stack execution completed!");
        println!("{}", self.execution_stack.get_status_summary());
        Ok(())
    }

    /// Process a single stack request (simplified implementation)
    async fn process_stack_request(&mut self, request: StackRequest) -> Result<StackResponse, Box<dyn std::error::Error>> {
        match request {
            StackRequest::UserPrompt { id, content, .. } => {
                // Handle user prompt by processing it through normal conversation
                self.handle_direct_conversation(&content).await?;
                
                Ok(StackResponse {
                    request_id: id.clone(),
                    success: true,
                    content: format!("Processed user prompt: {}", content),
                    generated_requests: Vec::new(),
                    completed_actions: vec![id],
                })
            }
            StackRequest::PlanAction { id, action, .. } => {
                // Handle plan action by executing through conversation system
                println!("⚙️  Executing plan action: {}", action.title);
                println!("🎯 Action details: {}", action.purpose);
                println!("🛠️  Using tool: {} on target: {}", action.tool, action.target);
                
                // Create execution prompt based on the action details
                let execution_prompt = format!(
                    "Execute this task:\n\nTitle: {}\nPurpose: {}\nTool: {}\nTarget: {}\nOperation: {}\nSuccess Criteria: {}\n\nPlease implement this task step by step.",
                    action.title,
                    action.purpose, 
                    action.tool,
                    action.target,
                    action.operation,
                    action.success_criteria
                );
                
                // Process through conversation system for real execution
                match self.handle_direct_conversation(&execution_prompt).await {
                    Ok(()) => {
                        println!("✅ Action completed: {}", action.title);
                        Ok(StackResponse {
                            request_id: id.clone(),
                            success: true,
                            content: format!("Successfully executed: {}", action.title),
                            generated_requests: Vec::new(),
                            completed_actions: vec![id],
                        })
                    }
                    Err(e) => {
                        println!("❌ Action failed: {} - {}", action.title, e);
                        Ok(StackResponse {
                            request_id: id.clone(),
                            success: false,
                            content: format!("Failed to execute: {} - Error: {}", action.title, e),
                            generated_requests: Vec::new(),
                            completed_actions: Vec::new(),
                        })
                    }
                }
            }
            StackRequest::NestedPlan { id, request, .. } => {
                // Handle nested plan by processing as conversation
                println!("📋 Processing nested plan: {}", request);
                
                match self.handle_direct_conversation(&request).await {
                    Ok(()) => {
                        println!("✅ Nested plan completed: {}", request);
                        Ok(StackResponse {
                            request_id: id.clone(),
                            success: true,
                            content: format!("Successfully processed nested plan: {}", request),
                            generated_requests: Vec::new(),
                            completed_actions: vec![id],
                        })
                    }
                    Err(e) => {
                        println!("❌ Nested plan failed: {} - {}", request, e);
                        Ok(StackResponse {
                            request_id: id.clone(),
                            success: false,
                            content: format!("Failed to process nested plan: {} - Error: {}", request, e),
                            generated_requests: Vec::new(),
                            completed_actions: Vec::new(),
                        })
                    }
                }
            }
        }
    }

    /// Get description of a request for logging
    fn get_request_description(&self, request: &StackRequest) -> String {
        match request {
            StackRequest::UserPrompt { content, .. } => format!("User Prompt: {}", content),
            StackRequest::PlanAction { action, .. } => format!("Plan Action: {}", action.title),
            StackRequest::NestedPlan { request, depth, .. } => format!("Nested Plan (depth {}): {}", depth, request),
        }
    }

    /// Handle simple tasks with direct conversation
    async fn handle_direct_conversation(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create temporary message list with adaptive context
        let system_message = self.create_adaptive_system_message().await;
        let mut conversation_messages = vec![system_message];
        
        // Get contextually relevant messages using unified context
        let contextual_messages = self.get_contextual_messages(input, 8000).await?;
        conversation_messages.extend(contextual_messages);
        
        // Add user message
        let user_message = Message {
            role: "user".to_string(),
            content: input.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };
        conversation_messages.push(user_message);

        // Process conversation loop
        loop {
            let model_to_use = self.select_model_for_conversation_state(&conversation_messages);
            
            let response = self.openrouter_client
                .chat_completion_with_model(conversation_messages.clone(), model_to_use, self.context.tools_needed)
                .await?;

            let assistant_message = &response.choices[0].message;
            conversation_messages.push(assistant_message.clone());
            // Note: assistant message added to unified context below

            // Add assistant message to unified context
            let execution_context = Some(ExecutionContext {
                stack_position: None,
                parent_tasks: Vec::new(),
                generated_files: Vec::new(),
                modified_files: Vec::new(),
                command_results: Vec::new(),
                current_phase: Some("assistant_response".to_string()),
            });
            
            if let Err(e) = self.unified_context
                .add_message(assistant_message.clone(), execution_context)
                .await 
            {
                if self.config.preferences.verbose {
                    println!("  ⚠️ Warning: Failed to add assistant message to context: {}", e);
                }
            }

            // Log assistant response
            if !assistant_message.content.is_empty() {
                self.story_logger.log_assistant_response(&assistant_message.content);
                self.update_working_memory(&assistant_message.content);
            }

            // Handle tool calls
            if let Some(tool_calls) = &assistant_message.tool_calls {
                self.execute_tools_semantically(tool_calls, &mut conversation_messages).await?;
            } else {
                // No more tool calls, conversation complete
                if !assistant_message.content.is_empty() {
                    println!("🤖 {}", assistant_message.content);
                    println!(); // Add extra newline to ensure proper spacing
                }
                break;
            }
        }

        Ok(())
    }

    /// Get execution stack status
    pub fn get_stack_status(&self) -> String {
        self.execution_stack.get_status_summary()
    }

    /// Clear the execution stack
    pub fn clear_execution_stack(&mut self) {
        self.execution_stack.clear_all();
    }

    /// Toggle automatic stack execution
    pub fn set_auto_execute_stack(&mut self, enabled: bool) {
        self.auto_execute_stack = enabled;
        println!("🔄 Auto-execute stack: {}", if enabled { "enabled" } else { "disabled" });
    }



    // ================================
    // CONTEXT PERSISTENCE METHODS
    // ================================

    /// Save current context state to persistent storage
    pub async fn save_context(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let save_path = self.unified_context.manual_save_context().await?;
        
        if self.config.preferences.verbose {
            println!("💾 Context saved to: {}", save_path.to_string_lossy());
        }
        
        Ok(format!("Context saved to: {}", save_path.to_string_lossy()))
    }

    /// Load context from persistent storage (most recent)
    pub async fn load_context(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        self.unified_context.load_context_from_storage().await?;
        
        if self.config.preferences.verbose {
            println!("📂 Context loaded from storage");
        }
        
        Ok("Context successfully loaded from storage".to_string())
    }

    /// Load context from a specific snapshot file
    pub async fn load_context_from_file(&mut self, file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let path = std::path::PathBuf::from(file_path);
        self.unified_context.load_context_from_snapshot(path).await?;
        
        if self.config.preferences.verbose {
            println!("📂 Context loaded from: {}", file_path);
        }
        
        Ok(format!("Context loaded from: {}", file_path))
    }

    /// Get persistence information
    pub fn get_persistence_info(&self) -> String {
        let stats = self.unified_context.get_persistence_stats();
        
        let last_save = if let Some(save_time) = stats.last_save {
            format!("{:?}", save_time)
        } else {
            "Never".to_string()
        };
        
        format!(
            "Context Persistence Status:\n\
            - Enabled: {}\n\
            - Last Save: {}\n\
            - Pending Changes: {}\n\
            - Storage Size: {} bytes\n\
            - Save Errors: {}\n\
            - Backup Enabled: {}",
            stats.enabled,
            last_save,
            stats.pending_changes,
            stats.storage_size_bytes,
            stats.save_errors,
            stats.backup_enabled
        )
    }

    /// Enable or disable context persistence
    pub fn toggle_persistence(&mut self, enabled: bool) -> String {
        self.unified_context.set_persistence_enabled(enabled);
        
        if self.config.preferences.verbose {
            println!("💾 Context persistence: {}", if enabled { "enabled" } else { "disabled" });
        }
        
        format!("Context persistence: {}", if enabled { "enabled" } else { "disabled" })
    }

    // ================================
    // CONTEXT CONFIGURATION METHODS
    // ================================

    /// Configure context management settings
    pub fn configure_context(&mut self, max_context_tokens: Option<usize>, importance_threshold: Option<f64>, max_conversation_history: Option<usize>) -> String {
        let mut changes = Vec::new();
        
        if let Some(tokens) = max_context_tokens {
            self.unified_context.config.max_context_tokens = tokens;
            changes.push(format!("max_context_tokens: {}", tokens));
        }
        
        if let Some(threshold) = importance_threshold {
            self.unified_context.config.importance_threshold = threshold;
            changes.push(format!("importance_threshold: {}", threshold));
        }
        
        if let Some(history) = max_conversation_history {
            self.unified_context.config.max_conversation_history = history;
            changes.push(format!("max_conversation_history: {}", history));
        }
        
        if changes.is_empty() {
            "No configuration changes made".to_string()
        } else {
            format!("Context configuration updated: {}", changes.join(", "))
        }
    }

    /// Enable or disable specific context features
    pub fn configure_context_features(&mut self, semantic_search: Option<bool>, file_monitoring: Option<bool>, code_analysis: Option<bool>) -> String {
        let mut changes = Vec::new();
        
        if let Some(search) = semantic_search {
            self.unified_context.config.semantic_search_enabled = search;
            changes.push(format!("semantic_search: {}", search));
        }
        
        if let Some(monitoring) = file_monitoring {
            self.unified_context.config.file_monitoring_enabled = monitoring;
            changes.push(format!("file_monitoring: {}", monitoring));
        }
        
        if let Some(analysis) = code_analysis {
            self.unified_context.config.code_analysis_enabled = analysis;
            changes.push(format!("code_analysis: {}", analysis));
        }
        
        if changes.is_empty() {
            "No feature configuration changes made".to_string()
        } else {
            format!("Context features updated: {}", changes.join(", "))
        }
    }

    /// Get current context configuration
    pub fn get_context_config(&self) -> String {
        let config = &self.unified_context.config;
        
        format!(
            "Context Configuration:\n\
            - Max Context Tokens: {}\n\
            - Importance Threshold: {}\n\
            - Max Conversation History: {}\n\
            - Semantic Search: {}\n\
            - File Monitoring: {}\n\
            - Code Analysis: {}\n\
            - Persistence: {}",
            config.max_context_tokens,
            config.importance_threshold,
            config.max_conversation_history,
            config.semantic_search_enabled,
            config.file_monitoring_enabled,
            config.code_analysis_enabled,
            config.persistence_enabled
        )
    }

    /// Reset context configuration to defaults
    pub fn reset_context_config(&mut self) -> String {
        let default_config = crate::context::ContextConfig::default();
        self.unified_context.config = default_config;
        
        if self.config.preferences.verbose {
            println!("🔄 Context configuration reset to defaults");
        }
        
        "Context configuration reset to defaults".to_string()
    }

    /// Get context usage statistics
    pub fn get_context_stats(&self) -> String {
        let _analytics = &self.unified_context.analytics;
        
        // Get basic stats (would need to implement in analytics module)
        format!(
            "Context Usage Statistics:\n\
            - Messages in History: {}\n\
            - Session ID: {}\n\
            - Knowledge Graph Entities: {}\n\
            - Knowledge Graph Relationships: {}\n\
            - Files Being Monitored: {}\n\
            - Recent Executions: {}",
            self.unified_context.conversation_history.len(),
            self.unified_context.session_id,
            self.unified_context.working_knowledge.entities.len(),
            self.unified_context.working_knowledge.relationships.len(),
            self.unified_context.fs_watcher.get_monitored_files_count(),
            self.unified_context.execution_history.get_recent_execution_count()
        )
    }

    /// Trigger context optimization and cleanup
    pub async fn optimize_context(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        // Manage context size using unified context system
        self.unified_context.manage_context_size().await?;
        
        // Process any pending file system events
        let processed_events = self.unified_context.process_file_system_events().await?;
        
        if self.config.preferences.verbose {
            println!("🔧 Context optimization completed");
        }
        
        Ok(format!(
            "Context optimization completed:\n\
            - Processed {} file system events\n\
            - Managed context size within limits",
            processed_events.len()
        ))
    }

    /// Determine if simple tasks should use the execution stack using simple rules
    fn should_use_stack_for_simple_task(&self, user_input: &str) -> bool {
        let input_lower = user_input.to_lowercase();
        
        // Stack-based keywords (actions that produce artifacts)
        let stack_keywords = [
            "create", "write", "make", "build", "generate", "implement", "add",
            "install", "run", "execute", "compile", "deploy", "save", "edit",
            "modify", "update", "delete", "remove", "move", "copy", "mkdir",
            "touch", "chmod", "git", "npm", "cargo", "python", "node"
        ];
        
        // Direct conversation keywords (questions and information requests)
        let direct_keywords = [
            "what", "why", "how", "when", "where", "who", "explain", "describe",
            "tell", "show", "list", "help", "define", "meaning", "difference",
            "compare", "analyze", "understand", "learn"
        ];
        
        // Check for stack keywords
        let has_stack_keyword = stack_keywords.iter().any(|&keyword| input_lower.contains(keyword));
        let has_direct_keyword = direct_keywords.iter().any(|&keyword| input_lower.contains(keyword));
        
        // If both or neither, default to stack for action-oriented tasks
        if has_stack_keyword && !has_direct_keyword {
            true
        } else if has_direct_keyword && !has_stack_keyword {
            false
        } else {
            // Default: use stack for sentences without question words
            !input_lower.starts_with("what") && !input_lower.starts_with("why") && 
            !input_lower.starts_with("how") && !input_lower.contains("?")
        }
    }

    /// Simple priority determination based on keywords
    fn determine_priority(&self, user_input: &str) -> u8 {
        let input_lower = user_input.to_lowercase();
        
        // High priority keywords (7-8)
        let high_priority = ["urgent", "emergency", "critical", "asap", "immediately", "now", 
                           "broken", "crash", "failure", "error", "bug", "not working", "blocking"];
        
        // Medium priority keywords (5-6)  
        let medium_priority = ["important", "priority", "needed", "required", "deadline",
                              "business", "users", "performance"];
        
        if high_priority.iter().any(|&keyword| input_lower.contains(keyword)) {
            7 // High priority
        } else if medium_priority.iter().any(|&keyword| input_lower.contains(keyword)) {
            5 // Medium priority
        } else {
            3 // Normal priority
        }
    }
}

/// File system autocomplete for semantic engine
#[derive(Clone)]
pub struct CustomTextAutocomplete {
    working_dir: String,
}

impl CustomTextAutocomplete {
    pub fn new(working_dir: String) -> Self {
        Self {
            working_dir,
        }
    }
    
    /// Get slash command completions
    fn get_slash_command_completions(&self, input: &str) -> Vec<String> {
        let slash_commands = vec![
            "/clear".to_string(),
            "/model".to_string(),
            "/models".to_string(),
            "/help".to_string(),
            "/stack".to_string(),
            "/plan".to_string(),
            "/tier".to_string(),
        ];
        
        let input_lower = input.to_lowercase();
        slash_commands.into_iter()
            .filter(|cmd| cmd.starts_with(&input_lower))
            .collect()
    }
}

impl Autocomplete for CustomTextAutocomplete {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, inquire::CustomUserError> {
        // Handle slash commands first
        if input.starts_with('/') {
            let slash_suggestions = self.get_slash_command_completions(input);
            if !slash_suggestions.is_empty() {
                return Ok(slash_suggestions);
            }
        }
        
        // Handle filesystem autocomplete if '@' is present
        if input.contains('@') {
            let last_at = input.rfind('@').unwrap();
            let before_at = &input[..last_at];
            let after_at = &input[last_at + 1..];

            // Check if this is a folder path that should show contents
            if after_at.ends_with('/') && !after_at.trim_end_matches('/').is_empty() {
                let folder_contents = self.get_folder_contents(after_at);

                if !folder_contents.is_empty() {
                    let drill_suggestions: Vec<String> = folder_contents
                        .into_iter()
                        .map(|item| format!("{}@{}{}", before_at, after_at, item))
                        .collect();

                    return Ok(drill_suggestions);
                }
            }

            // Regular filesystem autocomplete
            let suggestions = self.get_file_suggestions(after_at);

            let full_suggestions: Vec<String> = suggestions
                .into_iter()
                .map(|suggestion| format!("{}@{}", before_at, suggestion))
                .collect();

            return Ok(full_suggestions);
        }

        // No suggestions for regular text
        Ok(vec![])
    }

    fn get_completion(
        &mut self,
        _input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<inquire::autocompletion::Replacement, inquire::CustomUserError> {
        Ok(match highlighted_suggestion {
            Some(suggestion) => inquire::autocompletion::Replacement::Some(suggestion),
            None => inquire::autocompletion::Replacement::None,
        })
    }
}

impl CustomTextAutocomplete {
    fn get_folder_contents(&self, folder_path: &str) -> Vec<String> {
        let clean_path = folder_path.trim_end_matches('/');
        let full_path = Path::new(&self.working_dir).join(clean_path);
        let mut entries = Vec::new();

        if let Ok(dir_entries) = fs::read_dir(&full_path) {
            for entry in dir_entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    if name.starts_with('.') && !folder_path.contains("/.") {
                        continue;
                    }

                    if metadata.is_dir() {
                        entries.push(format!("{}/", name));
                    } else {
                        entries.push(name);
                    }
                }
            }
        }

        entries.sort_by(|a, b| {
            let a_is_dir = a.ends_with('/');
            let b_is_dir = b.ends_with('/');
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });

        entries
    }

    fn get_file_suggestions(&self, partial_path: &str) -> Vec<String> {
        if partial_path.is_empty() {
            return self.list_directory(".");
        }

        let path = Path::new(partial_path);
        let (dir_path, file_prefix) = if partial_path.ends_with('/') {
            (partial_path.trim_end_matches('/').to_string(), String::new())
        } else {
            match path.parent() {
                Some(parent) => {
                    let parent_str = parent.to_string_lossy().to_string();
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    (parent_str, file_name)
                }
                None => (".".to_string(), partial_path.to_string()),
            }
        };

        let dir_path_str = if dir_path.is_empty() { "." } else { &dir_path };
        let entries = self.list_directory(dir_path_str);

        entries
            .into_iter()
            .filter(|entry| entry.starts_with(&file_prefix))
            .collect()
    }

    fn list_directory(&self, relative_path: &str) -> Vec<String> {
        let full_path = Path::new(&self.working_dir).join(relative_path);
        let mut entries = Vec::new();

        if let Ok(dir_entries) = fs::read_dir(&full_path) {
            for entry in dir_entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    if name.starts_with('.') && !relative_path.contains("/.") {
                        continue;
                    }

                    let entry_path = if relative_path == "." {
                        if metadata.is_dir() {
                            format!("{}/", name)
                        } else {
                            name
                        }
                    } else {
                        let clean_relative_path = relative_path.trim_end_matches('/');
                        if metadata.is_dir() {
                            format!("{}/{}/", clean_relative_path, name)
                        } else {
                            format!("{}/{}", clean_relative_path, name)
                        }
                    };

                    entries.push(entry_path);
                }
            }
        }

        entries.sort_by(|a, b| {
            let a_is_dir = a.ends_with('/');
            let b_is_dir = b.ends_with('/');
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });

        entries
    }
}