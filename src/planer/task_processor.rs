use crate::context::Context;
use crate::llm::{Message, OpenRouterClient};
use crate::planer::plan::{PlanContext, TaskResult};
use crate::planer::task::{Task, TaskExecution, ToolCall};
use crate::planer::task_executor::TaskExecutor;
use crate::prompts::PromptManager;
use crate::tools::{exec, file_system};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

/// LLM-powered task processor that executes tasks with context awareness
pub struct TaskProcessor {
    llm_client: Arc<OpenRouterClient>,
    model: String,
    verbose: bool,
    pub task_executor: TaskExecutor,
}

/// Context for a single task execution
#[derive(Debug, Clone)]
pub struct TaskExecutionContext {
    /// Main application context (read-only)
    pub main_context: Context,
    /// Plan context with accumulated results (read-only for task)
    pub plan_context: PlanContext,
    /// Dependencies results for this specific task
    pub dependency_results: HashMap<usize, TaskResult>,
    /// Current task being executed
    pub current_task: Task,
}

/// LLM response for task execution analysis
#[derive(Debug, Deserialize)]
struct TaskExecutionResponse {
    /// Analysis of the task and its context
    analysis: String,
    /// Recommended approach for execution
    approach: String,
    /// Expected outcome
    expected_outcome: String,
    /// Key variables to extract from results
    variables_to_extract: Vec<String>,
    /// Whether to proceed with execution
    should_execute: bool,
}

impl TaskProcessor {
    /// Create new task processor
    pub fn new(llm_client: Arc<OpenRouterClient>) -> Self {
        Self {
            llm_client,
            model: "openai/gpt-4o-mini".to_string(),
            verbose: false,
            task_executor: TaskExecutor::new(),
        }
    }

    /// Set the task executor
    pub fn with_task_executor(mut self, task_executor: TaskExecutor) -> Self {
        self.task_executor = task_executor;
        self
    }

    /// Set the model to use
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Enable verbose logging
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Execute a task with full context awareness and LLM processing
    pub async fn execute_task_with_context(
        &self,
        task: &Task,
        execution_context: TaskExecutionContext,
    ) -> Result<TaskResult, String> {
        if self.verbose {
            println!("🧠 Processing task {} with LLM: {}", task.id, task.title);
        }

        let tool_call = match &task.execution {
            TaskExecution::ToolCall(tc) => tc,
            TaskExecution::SubPlan(_) => {
                return Ok(TaskResult {
                    task_id: task.id,
                    tool_result: "Sub-plan execution not handled by this processor".to_string(),
                    llm_processed_result: "Sub-plan execution should be handled by TaskExecutor"
                        .to_string(),
                    extracted_variables: HashMap::new(),
                    success: false,
                    executed_at: Utc::now(),
                });
            }
        };

        // Step 1: LLM analyzes the task with full context
        let analysis = self
            .analyze_task_with_context(task, tool_call, &execution_context)
            .await?;

        if !analysis.should_execute {
            return Ok(TaskResult {
                task_id: task.id,
                tool_result: "Task skipped based on LLM analysis".to_string(),
                llm_processed_result: analysis.analysis,
                extracted_variables: HashMap::new(),
                success: false,
                executed_at: Utc::now(),
            });
        }

        // Step 2: Execute the actual tool operation
        let tool_result = self.execute_tool_operation(tool_call).await?;

        // Step 3: LLM processes the result with context awareness
        let processed_result = self
            .process_result_with_context(
                task,
                tool_call,
                &tool_result,
                &execution_context,
                &analysis,
            )
            .await?;

        // Step 4: Extract variables as suggested by LLM analysis
        let extracted_variables = self
            .extract_variables_from_result(
                &tool_result,
                &processed_result,
                &analysis.variables_to_extract,
            )
            .await?;

        Ok(TaskResult {
            task_id: task.id,
            tool_result: tool_result.clone(),
            llm_processed_result: processed_result,
            extracted_variables,
            success: true,
            executed_at: Utc::now(),
        })
    }

    /// LLM analyzes task with full context before execution
    async fn analyze_task_with_context(
        &self,
        task: &Task,
        tool_call: &ToolCall,
        context: &TaskExecutionContext,
    ) -> Result<TaskExecutionResponse, String> {
        let prompt = self.create_task_analysis_prompt(task, tool_call, context);

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are an expert task analyst that evaluates coding tasks with context awareness. Analyze the task and determine the best execution approach.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let response = self
            .llm_client
            .send_conversation(&self.model, messages, Some(1000), Some(0.3))
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;

        let content = response
            .choices
            .first()
            .ok_or("No response from LLM")?
            .message
            .content
            .clone();
        let json_content = PromptManager::extract_json_from_markdown(&content);
        serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse LLM analysis: {}. Content: {}", e, content))
    }

    /// Create prompt for task analysis
    fn create_task_analysis_prompt(
        &self,
        task: &Task,
        tool_call: &ToolCall,
        context: &TaskExecutionContext,
    ) -> String {
        let dependency_context = if !context.dependency_results.is_empty() {
            let deps: Vec<String> = context
                .dependency_results
                .iter()
                .map(|(id, result)| {
                    format!(
                        "Task {}: {} (Success: {})",
                        id,
                        Self::truncate_text(&result.llm_processed_result, 100),
                        result.success
                    )
                })
                .collect();
            format!(
                "## Dependency Results
{}",
                deps.join("\n")
            )
        } else {
            "## Dependency Results
No dependencies"
                .to_string()
        };

        let plan_context = context.plan_context.format_for_llm(&task.dependencies);

        format!(
            r###"Analyze this coding task with full context awareness and determine execution approach.

## Task Details
- **ID**: {}
- **Title**: {}
- **Tool**: {}
- **Target**: {}
- **Operation**: {}
- **Dependencies**: {:?}

{}

{}

## Main Project Context
- Working Directory: {}
- Files Tracked: {}
- Context Initialized: {}

## Analysis Required

Provide a JSON response with this structure:

```json
{{
  "analysis": "Detailed analysis of the task in context of previous results and dependencies",
  "approach": "Recommended execution approach based on context",
  "expected_outcome": "What you expect this task to accomplish",
  "variables_to_extract": ["key1", "key2"],
  "should_execute": true
}}
```

Consider:
1. How previous task results affect this task
2. Whether the task is still relevant given the context
3. What specific information should be extracted from the result
4. Any modifications needed to the execution approach"###,
            task.id,
            task.title,
            tool_call.tool,
            tool_call.target,
            tool_call.operation,
            task.dependencies,
            dependency_context,
            plan_context,
            context.main_context.root_path.display(),
            context.main_context.tracked_files_count(),
            context.main_context.initialized
        )
    }

    /// Execute the actual tool operation
    async fn execute_tool_operation(&self, tool_call: &ToolCall) -> Result<String, String> {
        Ok(self.task_executor.dispatch_tool(tool_call).await)
    }

    /// LLM processes the tool result with context awareness
    async fn process_result_with_context(
        &self,
        task: &Task,
        tool_call: &ToolCall,
        tool_result: &str,
        context: &TaskExecutionContext,
        analysis: &TaskExecutionResponse,
    ) -> Result<String, String> {
        let prompt = format!(
            r###"Process this task execution result with full context awareness.

## Task Executed
- **Title**: {}
- **Tool**: {}
- **Analysis**: {}
- **Expected Outcome**: {}

## Execution Result
```
{}
```

## Context
{}

## Processing Instructions

Analyze the result and provide a comprehensive summary that:
1. Explains what was accomplished
2. Relates the result to the overall plan context
3. Identifies any issues or unexpected outcomes
4. Suggests next steps or implications for future tasks
5. Extracts key information that might be useful for dependent tasks

Respond with a clear, structured analysis that will be useful for subsequent tasks."###,
            task.title,
            tool_call.tool,
            analysis.analysis,
            analysis.expected_outcome,
            Self::truncate_text(tool_result, 1000),
            context.plan_context.format_for_llm(&task.dependencies)
        );

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are an expert at analyzing task execution results in context. Provide structured, actionable analysis.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let response = self
            .llm_client
            .send_conversation(&self.model, messages, Some(800), Some(0.3))
            .await
            .map_err(|e| format!("LLM result processing failed: {}", e))?;

        Ok(response
            .choices
            .first()
            .ok_or("No response from LLM")?
            .message
            .content
            .clone())
    }

    /// Extract variables from result based on LLM suggestions
    async fn extract_variables_from_result(
        &self,
        tool_result: &str,
        processed_result: &str,
        variables_to_extract: &[String],
    ) -> Result<HashMap<String, String>, String> {
        if variables_to_extract.is_empty() {
            return Ok(HashMap::new());
        }

        let prompt = format!(
            r#"Extract specific variables from this task execution result.

## Tool Result
```
{}
```

## Processed Analysis
{}

## Variables to Extract
{}

Please extract the requested variables and return them in JSON format:

```json
{{
  "variable1": "extracted_value1",
  "variable2": "extracted_value2"
}}
```

If a variable cannot be found or extracted, omit it from the response."#,
            Self::truncate_text(tool_result, 800),
            Self::truncate_text(processed_result, 300),
            variables_to_extract.join(", ")
        );

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are an expert at extracting structured data from text. Extract only the requested variables in valid JSON format.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let response = self
            .llm_client
            .send_conversation(&self.model, messages, Some(400), Some(0.2))
            .await
            .map_err(|e| format!("LLM variable extraction failed: {}", e))?;

        let content = response
            .choices
            .first()
            .ok_or("No response from LLM")?
            .message
            .content
            .clone();
        let json_content = PromptManager::extract_json_from_markdown(&content);

        let parsed_json: serde_json::Value = serde_json::from_str(&json_content).map_err(|e| {
            format!(
                "Failed to parse extracted variables: {}. Content: {}",
                e, content
            )
        })?;

        let mut extracted_variables = HashMap::new();
        if let serde_json::Value::Object(map) = parsed_json {
            for (key, value) in map {
                if variables_to_extract.contains(&key) {
                    extracted_variables.insert(key, value.to_string());
                }
            }
        }

        Ok(extracted_variables)
    }

    /// Truncate text for prompt inclusion
    fn truncate_text(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[..max_len.saturating_sub(3)])
        }
    }
}
