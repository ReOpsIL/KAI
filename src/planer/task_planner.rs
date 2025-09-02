use crate::llm::{Message, OpenRouterClient};
use crate::planer::plan::{Phase, Plan};
use crate::planer::queue::{ExecutionQueue, QueueRequest, QueueResponse};
use crate::planer::task::{Task, TaskExecution, TaskStatus, ToolCall};
use crate::prompts::PromptManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A temporary struct that mirrors the flat JSON structure produced by the LLM for a task.
#[derive(Debug, Serialize, Deserialize)]
pub struct LlmTask {
    pub id: usize,
    pub title: String,
    pub tool: String,
    pub target: String,
    pub operation: String,
    pub content: String,
    pub dependencies: Vec<usize>,
    pub status: TaskStatus,
}

/// Minimal plan response from LLM
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanResponse {
    pub title: String,
    pub overview: String,
    pub phases: Vec<PlanPhase>,
}

/// Simple phase structure for LLM communication, using the temporary LlmTask.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanPhase {
    pub name: String,
    pub emoji: String,
    pub tasks: Vec<LlmTask>,
}

#[derive(Debug, Deserialize)]
struct DecompositionResponse {
    tasks: Vec<ToolCall>, // Expect tool calls for decomposition
}

/// Advanced task planner that coordinates plan generation and execution via LLM
pub struct TaskPlanner {
    pub execution_queue: ExecutionQueue,
    pub active_plans: Vec<Plan>,
    next_plan_id: usize,
    llm_client: Option<Arc<OpenRouterClient>>,
    model: String,
}

impl Default for TaskPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskPlanner {
    pub fn new() -> Self {
        Self {
            execution_queue: ExecutionQueue::new(),
            active_plans: Vec::new(),
            next_plan_id: 1,
            llm_client: None,
            model: "openai/gpt-4o-mini".to_string(),
        }
    }

    /// Create a new task planner with OpenRouter client
    pub fn with_llm_client(llm_client: Arc<OpenRouterClient>) -> Self {
        Self {
            execution_queue: ExecutionQueue::new(),
            active_plans: Vec::new(),
            next_plan_id: 1,
            llm_client: Some(llm_client),
            model: "openai/gpt-4o-mini".to_string(),
        }
    }

    /// Set the model to use for LLM requests
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Get the LLM client if available
    pub fn get_llm_client(&self) -> Option<Arc<OpenRouterClient>> {
        self.llm_client.clone()
    }

    /// Create an advanced plan from user input using LLM with context integration
    pub async fn create_advanced_plan_with_context(
        &mut self,
        user_input: &str,
        context: &crate::context::Context,
    ) -> Result<String, String> {
        let _llm_client = self.get_llm_client_or_err()?;

        let system_prompt = PromptManager::get_enhanced_system_prompt_with_context(context);
        let user_prompt = PromptManager::create_plan_user_message_with_context(user_input, context);

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt,
            },
            Message {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        let content = self.send_llm_request(messages).await?;
        let json_content = PromptManager::extract_json_from_markdown(&content);
        let plan_response: PlanResponse = serde_json::from_str(&json_content).map_err(|e| {
            format!(
                "Failed to parse LLM response as JSON: {}. Response: {}",
                e, content
            )
        })?;

        let plan = self.convert_plan_response_to_plan(plan_response)?;
        let request_ids = self.execution_queue.push_plan_tasks(&plan);
        let plan_display = format!("{}", plan);
        self.active_plans.push(plan);

        Ok(format!(
            "{}\n\n✅ Plan created successfully with {} tasks queued for execution.",
            plan_display,
            request_ids.len()
        ))
    }

    /// Decompose a complex task into smaller, executable sub-tasks
    pub async fn decompose_task(&self, task: &Task) -> Result<Vec<Task>, String> {
        let operation_prompt = if let TaskExecution::ToolCall(tool_call) = &task.execution {
            &tool_call.operation
        } else {
            // Cannot decompose a sub-plan further in this manner
            return Err("Cannot decompose a task that is already a sub-plan.".to_string());
        };

        let prompt = PromptManager::create_task_decomposition_prompt(&task.title, operation_prompt);
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt,
        }];

        let content = self.send_llm_request(messages).await?;
        let json_content = PromptManager::extract_json_from_markdown(&content);
        let decomposition: DecompositionResponse =
            serde_json::from_str(&json_content).map_err(|e| {
                format!(
                    "Failed to parse LLM decomposition response as JSON: {}. Response: {}",
                    e, content
                )
            })?;

        // Convert ToolCalls to Tasks
        let sub_tasks = decomposition
            .tasks
            .into_iter()
            .map(|tool_call| {
                // IDs will be assigned when added to the plan
                Task::new_tool_task(
                    0,                      // Placeholder ID
                    "Sub-task".to_string(), // Placeholder title
                    tool_call.tool,
                    tool_call.target,
                    tool_call.operation,
                    tool_call.content,
                )
            })
            .collect();

        Ok(sub_tasks)
    }

    /// Convert LLM PlanResponse to internal Plan structure
    fn convert_plan_response_to_plan(
        &mut self,
        plan_response: PlanResponse,
    ) -> Result<Plan, String> {
        let mut plan = Plan::new(plan_response.title, plan_response.overview);

        for plan_phase in plan_response.phases {
            let mut phase = Phase::new(plan_phase.name, plan_phase.emoji);

            for llm_task in plan_phase.tasks {
                let task_id = plan.generate_task_id();
                let task = Task::new_tool_task(
                    task_id,
                    llm_task.title,
                    llm_task.tool,
                    llm_task.target,
                    llm_task.operation,
                    llm_task.content,
                )
                .with_dependencies(llm_task.dependencies);
                phase.add_task(task);
            }
            plan.add_phase(phase);
        }
        Ok(plan)
    }

    /// Process the next request in the queue
    pub fn process_next_request(&mut self) -> Option<QueueResponse> {
        let request = self.execution_queue.pop_request()?;
        self.execution_queue.start_processing(request.clone());

        let response = match &request {
            QueueRequest::UserPrompt { content, .. } => self.handle_user_prompt(content),
            QueueRequest::TaskExecution { task, .. } => self.handle_task_execution(task),
        };

        self.execution_queue.push_response(response.clone());
        Some(response)
    }

    /// Replaces a task in a plan with a list of sub-tasks, rewiring dependencies.
    pub fn replace_task_with_subtasks(
        &mut self,
        plan_id: &str,
        original_task_id: usize,
        sub_tasks: Vec<Task>,
    ) -> Result<(), String> {
        let plan = self
            .active_plans
            .iter_mut()
            .find(|p| p.id == plan_id)
            .ok_or_else(|| format!("Plan with ID '{}' not found", plan_id))?;

        let original_task = plan
            .find_task_by_id(original_task_id)
            .ok_or_else(|| format!("Task with ID {} not found in plan", original_task_id))?
            .clone();

        let original_dependencies = original_task.dependencies.clone();

        let task_to_decompose = plan.find_task_by_id(original_task_id).unwrap();
        task_to_decompose.set_status(TaskStatus::Decomposed);

        let mut new_task_ids = Vec::new();
        let mut last_task_id = original_task_id;

        for (i, sub_task_template) in sub_tasks.into_iter().enumerate() {
            if let TaskExecution::ToolCall(tool_call) = sub_task_template.execution {
                let new_task_id = plan.generate_task_id();
                let mut new_dependencies = sub_task_template.dependencies;

                if i == 0 {
                    new_dependencies.extend(original_dependencies.clone());
                } else {
                    new_dependencies.push(last_task_id);
                }
                new_dependencies.sort();
                new_dependencies.dedup();

                let new_task = Task::new_tool_task(
                    new_task_id,
                    sub_task_template.title,
                    tool_call.tool,
                    tool_call.target,
                    tool_call.operation,
                    tool_call.content,
                )
                .with_dependencies(new_dependencies);

                plan.add_task_to_phase(&new_task, None)?;
                new_task_ids.push(new_task_id);
                last_task_id = new_task_id;
            }
        }

        for task in plan.get_all_tasks_mut() {
            if task.dependencies.contains(&original_task_id) {
                task.dependencies.retain(|&dep| dep != original_task_id);
                task.dependencies.extend(new_task_ids.clone());
                task.dependencies.sort();
                task.dependencies.dedup();
            }
        }
        Ok(())
    }

    /// Add a high-priority user prompt
    pub fn add_user_prompt(&mut self, content: String, priority: u8) -> String {
        self.execution_queue.push_user_prompt(content, priority)
    }

    /// Check if there are pending requests
    pub fn has_pending_work(&self) -> bool {
        self.execution_queue.has_pending_requests()
    }

    /// Get status summary
    pub fn get_status(&self) -> String {
        format!(
            "Active Plans: {}, Pending Requests: {}",
            self.active_plans.len(),
            self.execution_queue.pending_count()
        )
    }

    /// Find a plan by title
    pub fn find_plan_by_title(&mut self, title: &str) -> Option<&mut Plan> {
        self.active_plans
            .iter_mut()
            .find(|p| p.title.contains(title))
    }

    /// Complete a task and update plan status
    pub fn complete_task(&mut self, task_id: usize) -> bool {
        for plan in &mut self.active_plans {
            if let Some(task) = plan.find_task_by_id(task_id) {
                task.set_status(TaskStatus::Completed);
                self.execution_queue.push_plan_tasks(plan);
                return true;
            }
        }
        false
    }

    /// Generate unique plan ID
    fn generate_plan_id(&mut self) -> String {
        let id = format!("plan_{}", self.next_plan_id);
        self.next_plan_id += 1;
        id
    }

    /// Helper to get LLM client or return an error
    fn get_llm_client_or_err(&self) -> Result<Arc<OpenRouterClient>, String> {
        self.llm_client
            .clone()
            .ok_or_else(|| "No LLM client available for AI planning".to_string())
    }

    /// Helper to send a request to the LLM and get the content
    async fn send_llm_request(&self, messages: Vec<Message>) -> Result<String, String> {
        let client = self.get_llm_client_or_err()?;
        let response = client
            .send_conversation(&self.model, messages, Some(4000), Some(0.1))
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;
        Ok(response
            .choices
            .first()
            .ok_or("No response from LLM")?
            .message
            .content
            .clone())
    }

    /// Handle user prompt processing
    fn handle_user_prompt(&mut self, content: &str) -> QueueResponse {
        let request_id = self.execution_queue.generate_id();
        QueueResponse {
            request_id,
            success: false,
            content: format!(
                "User prompt '{}' requires AI planning - no dummy processing available",
                content
            ),
            completed_task_ids: Vec::new(),
            decomposed_tasks: None,
        }
    }

    /// Handle task execution
    fn handle_task_execution(&mut self, task: &Task) -> QueueResponse {
        let request_id = self.execution_queue.generate_id();
        let result = if let TaskExecution::ToolCall(tool_call) = &task.execution {
            format!(
                "Executed task '{}' using tool '{}' on target '{}'",
                task.title, tool_call.tool, tool_call.target,
            )
        } else {
            format!("Executed sub-plan task '{}'", task.title)
        };

        QueueResponse {
            request_id,
            success: true,
            content: result,
            completed_task_ids: vec![task.id],
            decomposed_tasks: None,
        }
    }
}
