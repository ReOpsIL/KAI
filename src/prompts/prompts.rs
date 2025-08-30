/// Centralized prompt management to ensure consistency across all LLM interactions
pub struct PromptManager;

impl PromptManager {
    /// Unified system prompt that aligns with action_plan_prompt.md philosophy
    /// This should be used consistently across all LLM interactions
    pub fn get_system_prompt() -> String {
        "You are an expert coding assistant that creates detailed, step-by-step action plans for coding tasks. \
        You work with filesystem and command execution tools to complete user requests. \
        You always respond with valid JSON when JSON format is requested, and you follow the exact format specified in user prompts. \
        You create comprehensive plans that break down complex requests into specific, executable actions.".to_string()
    }

    /// Enhanced system prompt with context integration
    pub fn get_enhanced_system_prompt_with_context(context: &crate::context::Context) -> String {
        let base_prompt = Self::get_system_prompt();
        
        // Generate context information
        let context_info = Self::format_context_for_prompt(context);
        
        format!("{}\n\n{}", base_prompt, context_info)
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

        // Add recent conversation history if available
        let recent_interactions = context.get_user_interactions_in_timeframe(1); // Last 1 day
        if !recent_interactions.is_empty() {
            context_parts.push("## Recent Context".to_string());
            
            // Limit to last 3 interactions to avoid token bloat
            let recent_interactions: Vec<_> = recent_interactions.into_iter().take(3).collect();
            
            for (prompt, response, _timestamp) in recent_interactions {
                context_parts.push(format!("- User asked: {}", 
                    Self::truncate_text(&prompt, 100)));
                
                if let Some(resp) = response {
                    context_parts.push(format!("- Assistant responded: {}", 
                        Self::truncate_text(&resp, 150)));
                }
            }
        }

        // Add file change information if any
        if context.initialized {
            context_parts.push("## Session State\n- Context initialized and tracking file changes".to_string());
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

    /// Get the main action plan prompt template (embedded)
    pub fn get_action_plan_template() -> &'static str {
        r#"# LLM Action Plan Generation Prompt

## Core Instruction

You are a coding assistant cli application that needs to create a highly granulated plan for handling user request. When given a user request, you must create a comprehensive, step-by-step action plan that breaks down the user request into specific, executable actions. Each action should specify exactly which tools to use and what operations to perform.

## Plan Structure Requirements

Your plan must include:

### 1. **Analysis Phase**
- Understanding the request scope and requirements
- Identifying the project structure and technology stack
- Locating relevant files and dependencies
- Assessing current implementation state

### 2. **Discovery Actions**
For each discovery step, specify:
- **Tool**: Which tool to use (Read, Grep, Glob, LS, Bash, etc.)
- **Target**: Specific files, directories, or patterns to examine
- **Purpose**: What information you're seeking
- **Expected Output**: What you expect to find

### 3. **Implementation Actions**
For each implementation step, specify:
- **Tool**: Which tool to use (Edit, MultiEdit, Write, Bash, etc.)
- **Operation**: Exact changes to make
- **Files**: Specific file paths involved
- **Dependencies**: Prerequisites from previous steps
- **Validation**: How to verify the step succeeded

### 4. **Verification Phase**
- Testing procedures
- Quality checks (lint, typecheck, build)
- Integration verification
- Documentation updates

## Action Item Format

Each action item must follow this structure:

```
### Action [N]: [Brief Description]
**Tool**: [Tool Name]
**Target**: [Specific file/directory/pattern]
**Operation**: [Detailed description of what to do]
**Purpose**: [Why this step is necessary]
**Success Criteria**: [How to know it worked]
**Dependencies**: [Previous actions this depends on]
```

## Required Output Format

**IMPORTANT**: Your response MUST be in valid JSON format that can be parsed and processed programmatically. Use the following structure:

```json
{
  "title": "Brief task summary",
  "overview": "Detailed description of the overall task and approach",
  "phases": [
    {
      "name": "Analysis Phase",
      "emoji": "🔍",
      "actions": [
        {
          "id": 1,
          "title": "Action title",
          "tool": "Tool name",
          "target": "Specific file/directory/pattern",
          "operation": "Detailed description of what to do",
          "purpose": "Why this step is necessary",
          "success_criteria": "How to know it worked",
          "dependencies": [2, 3],
          "status": "pending"
        }
      ]
    },
    {
      "name": "Implementation Phase", 
      "emoji": "🛠️",
      "actions": [...]
    },
    {
      "name": "Verification Phase",
      "emoji": "✅", 
      "actions": [...]
    }
  ],
  "expected_outcome": "Description of the final state after all actions complete"
}
```

**Critical Requirements**:
- Response must be valid JSON only - no markdown, no explanations, no additional text
- All string values must be properly escaped for JSON
- Action IDs must be unique integers starting from 1
- Dependencies array contains action IDs that must complete first
- Status must be "pending" for all initial actions
- Include all phases: Analysis, Implementation, Verification (minimum)

## Quality Guidelines

1. **Specificity**: Each action must be specific enough to execute without ambiguity
2. **Atomicity**: Each action should accomplish one clear objective
3. **Dependency Management**: Clearly specify which actions depend on others
4. **Error Handling**: Consider what could go wrong and how to handle it
5. **Validation**: Include verification steps for critical changes

## Tool Selection Guide

- **Read**: For examining specific files
- **Grep**: For searching content across files
- **Glob**: For finding files by pattern
- **LS**: For directory exploration
- **Edit/MultiEdit**: For modifying existing files
- **Write**: For creating new files (use sparingly)
- **Bash**: For running commands, tests, builds
- **Task**: For complex multi-step operations

## Response Requirements

1. Always start with a clear overview
2. Number actions sequentially
3. Group related actions logically
4. Include emoji headers for visual organization
5. End with expected outcome summary
6. Ensure the plan is comprehensive but not overly verbose

Remember: The goal is to create a plan so detailed that any LLM could execute it step-by-step without additional decision-making."#
    }

    /// Create a user message for plan generation requests
    pub fn create_plan_user_message(user_request: &str) -> String {
        let base_prompt = Self::get_action_plan_template();
        
        format!(
            "{}\n\n---\n\n## User Request\n\n{}\n\nPlease create a detailed action plan for this request following the format specified above. Remember to respond with valid JSON only.",
            base_prompt,
            user_request
        )
    }
}