use crate::cli::config::OpenRouterConfig;
use crate::context::context::Context;
use crate::llm::openrouter::OpenRouterClient;
use crate::planer::plan::{Plan, PlanContext, TaskResult};
use crate::planer::task::{Task, TaskExecution, TaskStatus, ToolCall};
use crate::tools::{exec, file_system};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Task executor that handles tool calls, sub-plans, and LLM processing of results.
#[derive(Debug, Clone)]
pub struct TaskExecutor {
    pub verbose: bool,
    pub workdir: PathBuf,
    pub openrouter_client: Option<OpenRouterClient>,
    pub midrange_model: String,
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskExecutor {
    pub fn new() -> Self {
        let workdir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("workdir");
        Self {
            verbose: false,
            workdir,
            openrouter_client: None,
            midrange_model: OpenRouterConfig::default().midrange_model,
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn with_workdir<P: AsRef<Path>>(mut self, workdir: P) -> Self {
        self.workdir = workdir.as_ref().to_path_buf();
        self
    }

    pub fn with_openrouter_client(mut self, client: OpenRouterClient) -> Self {
        self.openrouter_client = Some(client);
        self
    }

    /// Resolve a path relative to the working directory
    fn resolve_path(&self, path: &str) -> PathBuf {
        let path_buf = PathBuf::from(path);
        if path_buf.is_absolute() {
            path_buf
        } else {
            self.workdir.join(path_buf)
        }
    }

    /// Convert a resolved path back to a string for tool operations
    fn path_to_string(&self, path: &Path) -> String {
        path.to_string_lossy().to_string()
    }

    /// Main entry point to execute a plan
    pub async fn execute_plan(&self, plan: &mut Plan, global_context: &Context) {
        let mut completed_ids = Vec::new();

        while !plan.is_complete() {
            let ready_tasks: Vec<usize> = plan
                .get_all_tasks()
                .iter()
                .filter(|t| t.is_ready_to_execute(&completed_ids))
                .map(|t| t.id)
                .collect();

            if ready_tasks.is_empty() {
                if self.verbose {
                    println!("No more ready tasks, but plan is not complete. Exiting.");
                }
                break;
            }

            for task_id in ready_tasks {
                let task = plan.find_task_by_id(task_id).unwrap().clone();
                let result = self
                    .execute_task(&task, global_context, &plan.plan_context)
                    .await;
                plan.add_task_result(result);
                completed_ids.push(task.id);
                plan.find_task_by_id(task_id).unwrap().status = TaskStatus::Completed;
            }
        }
    }

    /// Execute a single task and return the result
    pub async fn execute_task(
        &self,
        task: &Task,
        global_context: &Context,
        plan_context: &PlanContext,
    ) -> TaskResult {
        if self.verbose {
            println!("Executing task: {}", task.title);
        }

        match &task.execution {
            TaskExecution::ToolCall(tool_call) => {
                self.execute_tool_call(task, tool_call, global_context, plan_context)
                    .await
            }
            TaskExecution::SubPlan(sub_plan) => {
                Box::pin(self.execute_sub_plan(
                    task,
                    sub_plan.clone(),
                    global_context,
                    plan_context,
                ))
                .await
            }
        }
    }

    /// Execute a tool call and process its result with an LLM
    async fn execute_tool_call(
        &self,
        task: &Task,
        tool_call: &ToolCall,
        global_context: &Context,
        plan_context: &PlanContext,
    ) -> TaskResult {
        // Prepare the tool call with LLM if necessary
        let prepared_tool_call = self
            .prepare_tool_call_with_llm(task, tool_call, plan_context)
            .await
            .unwrap_or_else(|e| {
                if self.verbose {
                    println!("LLM preparation failed: {}", e);
                }
                tool_call.clone() // Fallback to original tool_call
            });

        let tool_result = self.dispatch_tool(&prepared_tool_call).await;

        let llm_processed_result = self
            .process_result_with_llm(&tool_result, global_context, plan_context)
            .await
            .unwrap_or_else(|e| format!("LLM processing failed: {}", e));

        TaskResult {
            task_id: task.id,
            tool_result,
            llm_processed_result,
            extracted_variables: HashMap::new(), // Placeholder
            success: true,                       // Placeholder
            executed_at: Utc::now(),
        }
    }

    /// Pre-process a tool call with an LLM to generate dynamic content.
    async fn prepare_tool_call_with_llm(
        &self,
        task: &Task,
        tool_call: &ToolCall,
        plan_context: &PlanContext,
    ) -> Result<ToolCall, String> {
        let client = self
            .openrouter_client
            .as_ref()
            .ok_or("OpenRouter client not configured")?;

        println!(
            "[LLM_DEBUG_INPUT] Prompt for prepare_tool_call_with_llm:\n{}",
            tool_call.tool
        );

        match tool_call.tool.as_str() {
            "write_file" => {
                // Find the dependency that read the file
                let read_task_dep = task
                    .dependencies
                    .first()
                    .ok_or("write_file task must have a dependency")?;
                let file_content = plan_context
                    .get_file_content_from_task(*read_task_dep)
                    .ok_or(format!(
                        "Could not retrieve file content from task {}",
                        read_task_dep
                    ))?;

                let prompt = format!(
                    "## Task: Generate File Content
                    You are an AI assistant modifying a file.
                    Based on the original content and the modification instructions, generate the new, complete content of the file.
                    ### Modification Instructions
                    {}
                    ### Original File Content
                    ```
                    {}
                    ```
                    ### Your Task
                    Generate the full, updated content for the file `{}`. Do not add any extra explanations or markdown formatting.
                    The output should be only the raw file content.",
                    tool_call.operation, file_content, tool_call.target
                );

                println!("[LLM_DEBUG_INPUT] Prompt for write_file:\n{}", prompt);

                let response = client
                    .send_prompt(&self.midrange_model, &prompt, Some(2000), Some(0.1))
                    .await
                    .map_err(|e| e.to_string())?;
                let new_content = response
                    .choices
                    .first()
                    .map_or("".to_string(), |c| c.message.content.clone());

                println!(
                    "[LLM_DEBUG_OUTPUT] New content for write_file:\n{}",
                    new_content
                );

                let mut new_tool_call = tool_call.clone();
                new_tool_call.content = new_content;
                Ok(new_tool_call)
            }
            "bash" => {
                let prompt = format!(
                    "## Task: Generate Shell Command
                    You are an AI assistant generating a shell command.
                    Based on the high-level operation description, generate the precise, executable shell command.
                    ### Operation Description
                    {}
                    ### Your Task
                    Generate the shell command to be executed. Do not add any extra explanations or markdown formatting.
                    The output should be only the raw command.",
                    tool_call.operation
                );

                println!("[LLM_DEBUG_INPUT] Prompt for bash:\n{}", prompt);

                let response = client
                    .send_prompt(&self.midrange_model, &prompt, Some(200), Some(0.1))
                    .await
                    .map_err(|e| e.to_string())?;
                let new_target = response
                    .choices
                    .first()
                    .map_or("".to_string(), |c| c.message.content.clone());

                println!("[LLM_DEBUG_OUTPUT] New target for bash:\n{}", new_target);

                let mut new_tool_call = tool_call.clone();
                new_tool_call.target = new_target;
                Ok(new_tool_call)
            }
            _ => Ok(tool_call.clone()), // No preparation needed for other tools
        }
    }

    /// Execute a sub-plan recursively
    async fn execute_sub_plan(
        &self,
        task: &Task,
        mut sub_plan: Plan,
        global_context: &Context,
        _parent_plan_context: &PlanContext,
    ) -> TaskResult {
        if self.verbose {
            println!("Executing sub-plan: {}", sub_plan.title);
        }

        // The sub-plan's context already inherits from the parent context
        self.execute_plan(&mut sub_plan, global_context).await;

        let final_results = sub_plan.plan_context.get_all_available_results();
        let summary = format!(
            "Sub-plan '{}' completed. {} tasks executed.",
            sub_plan.title,
            final_results.len()
        );

        TaskResult {
            task_id: task.id,
            tool_result: summary.clone(),
            llm_processed_result: summary,
            extracted_variables: HashMap::new(),
            success: sub_plan.is_complete(),
            executed_at: Utc::now(),
        }
    }

    /// Dispatch a tool call to the appropriate handler
    pub async fn dispatch_tool(&self, tool_call: &ToolCall) -> String {
        println!(
            "[LLM_DEBUG_INPUT] Prompt for dispatch_tool:\n{}",
            tool_call.tool
        );
        let result = match tool_call.tool.as_str() {
            "read" | "read_file" => {
                let resolved_path = self.resolve_path(&tool_call.target);
                file_system::FileSystemOperations::read_file(&self.path_to_string(&resolved_path))
            }
            "write" | "write_file" => {
                let resolved_path = self.resolve_path(&tool_call.target);
                file_system::FileSystemOperations::write_file(
                    &self.path_to_string(&resolved_path),
                    &tool_call.content,
                    None,
                )
            }
            "bash" | "run_shell" => {
                let command_with_cd =
                    format!("cd {} && {}", self.workdir.display(), tool_call.target);
                exec::run_shell_command_tool(&command_with_cd)
            }
            "ls" | "list_directory" => {
                let resolved_path = if tool_call.target.is_empty()
                    || tool_call.target == "."
                    || tool_call.target == "./"
                {
                    self.workdir.clone()
                } else {
                    self.resolve_path(&tool_call.target)
                };
                file_system::FileSystemOperations::list_directory(
                    &self.path_to_string(&resolved_path),
                    None,
                    None,
                )
            }
            _ => {
                return format!("Unknown tool: {}", tool_call.tool);
            }
        };

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Failed to serialize result: {}", e))
    }

    /// Process a tool's raw result with the LLM for contextual analysis
    async fn process_result_with_llm(
        &self,
        tool_result: &str,
        global_context: &Context,
        plan_context: &PlanContext,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let client = self
            .openrouter_client
            .as_ref()
            .ok_or("OpenRouter client not configured")?;

        let plan_context_summary = plan_context.format_for_llm(&[]); // Simplified for now
        let global_context_summary = global_context.query_story_timeframe(1); // Example summary

        let prompt = format!(
            "## Task: Analyze Tool Result in Context\n            You are an AI assistant analyzing the output of a command-line tool. Your goal is to interpret the result, summarize it, and identify any important information or variables that should be passed to subsequent tasks.\n\n            ### Global Context\n\n            Recent user interactions:\n\n            {}\n\n            ### Plan Execution Context\n\n            {}\n\n            ### Tool Execution Result\n\n            ```json            {}\n\n            ```\n\n            ### Your Analysis\n\n            Based on the context, provide a concise summary of the tool's output. If the tool reported an error, explain the likely cause. If it was successful, describe the outcome. Extract any key variables or data that might be useful for the next steps in the plan.",
            global_context_summary,
            plan_context_summary,
            tool_result
        );

        println!(
            "[LLM_DEBUG_INPUT] Prompt for process_result_with_llm:\n{}",
            prompt
        );

        let response = client
            .send_prompt(&self.midrange_model, &prompt, None, None)
            .await?;
        let content = response
            .choices
            .first()
            .map_or("".to_string(), |c| c.message.content.clone());

        println!(
            "[LLM_DEBUG_OUTPUT] Response from process_result_with_llm:\n{}",
            content
        );

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planer::task::Task;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_execute_write_and_read() {
        let dir = tempdir().unwrap();
        let executor = TaskExecutor::new().with_workdir(dir.path());
        let global_context = Context::new();
        let plan_context = PlanContext::new(None);

        // Test write
        let write_task = Task::new_tool_task(
            1,
            "Write file".to_string(),
            "write".to_string(),
            "test.txt".to_string(),
            "Write content".to_string(),
            "Hello, world!".to_string(),
        );
        let write_response = executor
            .execute_task(&write_task, &global_context, &plan_context)
            .await;
        assert!(write_response.success);

        // Test read
        let read_task = Task::new_tool_task(
            2,
            "Read file".to_string(),
            "read".to_string(),
            "test.txt".to_string(),
            "Read content".to_string(),
            "".to_string(),
        );
        let read_response = executor
            .execute_task(&read_task, &global_context, &plan_context)
            .await;
        assert!(read_response.success);
        assert!(read_response.tool_result.contains("Hello, world!"));
    }
}
