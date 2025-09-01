/// Centralized prompt management to ensure consistency across all LLM interactions
pub struct PromptManager;

impl PromptManager {
    /// Unified system prompt that aligns with context-aware task planning philosophy
    /// This should be used consistently across all LLM interactions
    pub fn get_system_prompt() -> String {
        "You are an expert coding assistant that creates detailed, step-by-step task plans for coding tasks. \
        You work with filesystem and command execution tools to complete user requests. \
        You always respond with valid JSON when JSON format is requested, and you follow the exact format specified in user prompts. \
        You create comprehensive plans that break down complex requests into specific, executable tasks. \
        When context information is provided, you leverage it to create more targeted and efficient plans without redundant discovery phases.".to_string()
    }

    /// Enhanced system prompt with context integration
    pub fn get_enhanced_system_prompt_with_context(context: &crate::context::Context) -> String {
        let base_prompt = Self::get_system_prompt();

        // Determine if we have rich file context available
        let has_file_context = context.initialized && !context.file_timestamps.is_empty();

        // Add context-specific guidance
        let context_guidance = if has_file_context {
            "\n\nIMPORTANT CONTEXT MODE:\n\
            - The project file structure and context are provided below\n\
            - All files have been pre-analyzed with detailed descriptions\n\
            - Skip redundant discovery tasks and focus on relevant files\n\
            - Use the provided file paths and context information to create targeted plans\n\
            - Prioritize analysis of existing implementations before proposing changes"
        } else {
            "\n\nDISCOVERY MODE:\n\
            - Project context is limited - start with filesystem discovery\n\
            - Use list_directory and find_files to understand project structure\n\
            - Build comprehensive understanding before implementation"
        };

        // Generate context information
        let context_info = Self::format_context_for_prompt(context);

        format!("{}{}\n\n{}", base_prompt, context_guidance, context_info)
    }

    /// Format context information for inclusion in system prompt
    fn format_context_for_prompt(context: &crate::context::Context) -> String {
        let mut context_parts = Vec::new();

        // Add project context
        context_parts.push(format!(
            "## Project Context\n- Working directory: {}\n- Tracking {} files",
            context.root_path.display(),
            context.tracked_files_count()
        ));

        // Add project file structure if context is initialized and has files
        if context.initialized && !context.file_timestamps.is_empty() {
            let mut file_paths: Vec<String> = context
                .file_timestamps
                .keys()
                .filter_map(|path| {
                    path.strip_prefix(&context.root_path)
                        .ok()
                        .map(|p| format!("- {}", p.display()))
                })
                .collect();

            // Sort for consistent output
            file_paths.sort();

            if !file_paths.is_empty() {
                context_parts.push(format!(
                    "## Project File Structure\nAll project files are known and tracked:\n{}",
                    file_paths.join("\n")
                ));
            }
        }

        // Add recent conversation history if available
        let recent_interactions = context.get_user_interactions_in_timeframe(1); // Last 1 day
        if !recent_interactions.is_empty() {
            context_parts.push("## Recent Context".to_string());

            // Limit to last 3 interactions to avoid token bloat
            let recent_interactions: Vec<_> = recent_interactions.into_iter().take(3).collect();

            for (prompt, response, _timestamp) in recent_interactions {
                context_parts.push(format!(
                    "- User asked: {}",
                    Self::truncate_text(&prompt, 100)
                ));

                if let Some(resp) = response {
                    context_parts.push(format!(
                        "- Assistant responded: {}",
                        Self::truncate_text(&resp, 150)
                    ));
                }
            }
        }

        // Add file change information if any
        if context.initialized {
            context_parts.push(
                "## Session State\n- Context initialized and tracking file changes".to_string(),
            );
        }

        if context_parts.is_empty() {
            return "## Context\nNo specific context available for this session.".to_string();
        }

        context_parts.join("\n\n")
    }

    /// Truncate text to specified length with ellipsis
    fn truncate_text(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[..max_len.saturating_sub(3)])
        }
    }

    /// Get the main task plan prompt template (embedded)
    pub fn get_action_plan_template(has_file_context: bool) -> String {
        let context_block: &str = r#"Context information above should help you understand the user's request and provide a more accurate task plan.
            Context information includes details about the user's requests, history, the project's structure, and any relevant files or directories.
            Each file in project was processed by the LLM to generate the context information.
            'Analysis Phase' and 'Discovery Phase' should be based on context information, no need to search all files in the project,
            only files that are relevant to the user's request according to the context information."#;

        let context_block = if has_file_context { context_block } else { "" };

        let main_block: String = format!(
            r#"# LLM Action Plan Generation Prompt

## Core Instruction

You are a coding assistant cli application that needs to create a highly granulated plan for handling user request. When given a user request, you must create a comprehensive, step-by-step task plan that breaks down the user request into specific, executable tasks. Each task should specify exactly which tools to use and what operations to perform.

## CRITICAL PATH REQUIREMENTS

**NEVER USE PLACEHOLDER PATHS**: You must NEVER use placeholder paths like "path/to/main/application/file" or "path/to/documentation/file". Instead:

1. **ALWAYS START WITH DISCOVERY**: Your first tasks must ALWAYS be discovery tasks to find actual files in the project
2. **USE ACTUAL FILE PATHS**: Only reference files that actually exist in the working directory
3. **BUILD FROM CONCRETE INFORMATION**: Each task must build on concrete information from previous discovery steps

{}

## Plan Structure Requirements

Your plan must include:

### 1. **Analysis Phase** (MANDATORY DISCOVERY)
- **MUST START** with listing the working directory contents (`list_directory` with target ".")
- Understanding the request scope and requirements
- Identifying the project structure and technology stack
- Locating relevant files through actual filesystem exploration
- Assessing current implementation state using real file paths
- Refer to context block and understand the description of each file or module

### 2. **Discovery Actions** (REQUIRED BEFORE IMPLEMENTATION)
For each discovery step, specify:
- **Tool**: Use `list_directory`, `read_file`, `grep_files`, or `find_files`
- **Target**: Use "." for current directory or actual discovered paths
- **Purpose**: What information you're seeking
- **Expected Output**: What you expect to find

**Discovery Examples**:
- First task: `list_directory` with target "." to see project structure
- Follow-up: `find_files` or `grep_files` to locate specific file types
- Then: `read_file` on discovered files to understand file content

### 3. **Implementation Actions** (ONLY AFTER DISCOVERY)
For each implementation step, specify:
- **Tool**: Which tool to use (write_file, bash, etc.)
- **Operation**: Description of the exact changes to make
- **Target**: Use for specific file paths discovered in Analysis phase or Discovery phase. The tool should execute the operation on the target (read  / write / delete Etc).
    Incase of command execution - linux bash command ( could include linux command/s or script/s to run as one line with `&&` operator).
    Refer to the following tool types and relevant value in "target":
    tool - list_directory: directory path to list
    tool - read_file: file path to read - "content" will be hold the result of reading the file
    tool - grep_files: pattern to search in files
    tool - find_files: pattern to locate files
    tool - write_file: file path to write the "content" into
    tool - bash: shell command to execute
- **Content**: The exact content to be written/modified incase of write_file or read_file, could be source code or document content.
- **Files**: ONLY use file paths discovered in Analysis phase
- **Dependencies**: Must depend on discovery tasks that found the files
- **Validation**: How to verify the step succeeded

### 4. **Verification Phase**
- Testing procedures using actual discovered files
- Quality checks (lint, typecheck, build)
- Integration verification
- Documentation updates on real documentation files

## Required Output Format

**IMPORTANT**: Your response MUST be a single, valid JSON object. Do not include any text, markdown, or explanations before or after the JSON object.
The JSON must be able to be parsed programmatically without any cleaning or modification. Use the following structure (content is example):

```
{{
  "title": "Brief task summary",
  "overview": "Detailed description of the overall task and approach",
  "phases": [
    {{
      "name": "Analysis Phase",
      "emoji": "🔍",
      "tasks": [
        {{
          "id": 1,
          "title": "List all files in the working directory",
          "tool": "list_directory",
          "target": ".",
          "operation": "List all files in the working directory to find the main application file that needs modification.",
          "content": "./",
          "purpose": "Discover the project structure and identify relevant files",
          "success_criteria": "Successfully list directory contents and identify file structure",
          "dependencies": [],
          "status": "Pending"
        }}
      ]
    }},
    {{
      "name": "Implementation Phase",
      "emoji": "🛠️",
      "tasks": [...]
    }},
    {{
      "name": "Verification Phase",
      "emoji": "✅",
      "tasks": [...]
    }}
  ],
  "expected_outcome": "Description of the final state after all tasks complete"
}}
```

**Critical Requirements**:
- The entire response must be a single, valid JSON object.
- Do not use comments (e.g., // or /* */) inside the JSON.
- Do not include any markdown, explanations, or any other text outside of the main JSON object.
- All string values must be properly escaped for JSON.
- Action IDs must be unique integers starting from 1.
- The `dependencies` array must only contain integer task IDs that must complete before the task can start.
- The `status` must be "Pending" for all initial tasks.
- Include all phases: Analysis, Implementation, and Verification (minimum).
- **NEVER use placeholder file paths** - only use "." for current directory or paths discovered through filesystem exploration

## Forbidden Patterns

**DO NOT DO THESE**:
- ❌ "target": "path/to/main/application/file"
- ❌ "target": "path/to/documentation/file"
- ❌ "target": "src/main.py" (unless you discovered this file exists)
- ❌ Any path that wasn't discovered through filesystem exploration

**DO THESE INSTEAD**:
- ✅ "target": "." (for listing current directory)
- ✅ "target": "*.py" (for finding Python files)
- ✅ "target": "README.md" (only after discovering it exists)
- ✅ Build implementation tasks on concrete discoveries

## Tool Selection Guide

- **list_directory**: For exploring directory structure (start with "." target)
- **read_file**: For examining discovered files
- **grep_files**: For searching content across discovered files
- **find_files**: For locating files by pattern
- **write_file**: For modifying discovered files
- **bash**: For shell commands - operation field must contain the actual command (e.g., "cargo build", "npm test", "python script.py")

## Response Requirements

1. Always start with directory exploration
2. Number tasks sequentially with proper dependencies
3. Group related tasks logically
4. Include emoji headers for visual organization
5. End with expected outcome summary
6. Ensure all file references are based on actual discovery

Remember: The goal is to create a plan so detailed that any LLM could execute it step-by-step using only real files discovered through filesystem exploration."#,
            context_block
        );

        main_block
    }

    /// Create a context-aware user message for plan generation requests
    pub fn create_plan_user_message_with_context(
        user_request: &str,
        context: &crate::context::Context,
    ) -> String {
        let has_file_context = context.initialized && !context.file_timestamps.is_empty();
        let base_prompt = Self::get_action_plan_template(has_file_context);

        format!(
            "{}\n\n---\n\n## User Request\n\n{}\n\nPlease create a detailed task plan for this request following the format specified above. Remember to respond with valid JSON only.",
            base_prompt,
            user_request
        )
    }
    /// Create a prompt for decomposing a complex task into smaller, tool-executable tasks
    pub fn create_task_decomposition_prompt(task_title: &str, task_operation: &str) -> String {
        format!(
            r#"You are an expert at decomposing complex tasks into a series of simple, executable steps.
Break down the following high-level task into a sequence of atomic tasks that can be executed using the available tools.

## High-Level Task
- **Title**: {}
- **Operation**: {}

## Available Tools
- `read_file(path)`
- `write_file(path, content)`
- `list_directory(path)`
- `grep_files(pattern, file_pattern)`
- `find_files(name_pattern)`
- `bash(command)`

## Decomposition Requirements
- Each decomposed task must map directly to one of the available tools.
- The `tool` field in the output must be one of the exact tool names listed above.
- The `target` and `operation` fields must contain the exact parameters for the chosen tool.
- Maintain dependencies between the new sub-tasks.

## Required Output Format
**IMPORTANT**: Your response MUST be a single, valid JSON object containing a list of tasks. Do not include any other text or explanations.

```json
{{
  "tasks": [
    {{
      "id": 1,
      "title": "First sub-task",
      "tool": "read_file",
      "target": "path/to/file.txt",
      "operation": "Read the file content",
      "dependencies": []
    }},
    {{
      "id": 2,
      "title": "Second sub-task",
      "tool": "bash",
      "target": "N/A",
      "operation": "echo 'hello' > new_file.txt",
      "dependencies": [1]
    }}
  ]
}}
```
"#,
            task_title, task_operation
        )
    }

    /// Extract JSON content from markdown-wrapped responses
    pub fn extract_json_from_markdown(content: &str) -> String {
        let content = content.trim();
        if content.starts_with("```json") && content.ends_with("```") {
            let start = content.find("```json").unwrap() + 7;
            let end = content.rfind("```").unwrap();
            return content[start..end].trim().to_string();
        }
        if content.starts_with("```") && content.ends_with("```") {
            let start = content.find("```").unwrap() + 3;
            let end = content.rfind("```").unwrap();
            return content[start..end].trim().to_string();
        }
        content.to_string()
    }
}
