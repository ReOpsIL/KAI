use crate::llm::{Message, OpenRouterClient};
use crate::planer::plan::{Phase, Plan};
use crate::planer::queue::{ExecutionQueue, QueueRequest, QueueResponse};
use crate::planer::schemas::{PlanPhase, PlanResponse, SimpleTask};
use crate::planer::task::{Task, TaskStatus};
use crate::prompts::PromptManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Advanced task planner that coordinates plan generation and execution via LLM
pub struct TaskPlanner {
    execution_queue: ExecutionQueue,
    active_plans: Vec<Plan>,
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

    /// Create an advanced plan from user input using LLM
    pub async fn create_advanced_plan(&mut self, user_input: &str) -> Result<String, String> {
        self.create_advanced_plan_with_context(user_input, &crate::context::Context::new())
            .await
    }

    /// Create an advanced plan from user input using LLM with context integration
    pub async fn create_advanced_plan_with_context(
        &mut self,
        user_input: &str,
        context: &crate::context::Context,
    ) -> Result<String, String> {
        let llm_client = match &self.llm_client {
            Some(client) => client.clone(),
            None => return Err("No LLM client available for AI planning".to_string()),
        };

        // Generate enhanced LLM prompt with context
        let system_prompt = PromptManager::get_enhanced_system_prompt_with_context(context);
        let user_prompt = PromptManager::create_plan_user_message(user_input);

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

        // Send request to LLM
        let response = llm_client
            .send_conversation(&self.model, messages, Some(4000), Some(0.1))
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;

        // Extract content from response
        let content = response
            .choices
            .first()
            .ok_or("No response from LLM")?
            .message
            .content
            .clone();

        // Parse the JSON response
        let plan_response: PlanResponse = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse LLM response as JSON: {}. Response: {}",
                e, content
            )
        })?;

        // Convert LLM response to internal plan structure
        let plan = self.convert_plan_response_to_plan(plan_response)?;

        // Add plan to active plans and queue its tasks
        let request_ids = self.execution_queue.push_plan_tasks(&plan);
        let plan_display = format!("{}", plan);
        self.active_plans.push(plan);

        Ok(format!(
            "{}\n\n✅ Plan created successfully with {} tasks queued for execution.",
            plan_display,
            request_ids.len()
        ))
    }

    /// Convert LLM PlanResponse to internal Plan structure
    fn convert_plan_response_to_plan(
        &mut self,
        plan_response: PlanResponse,
    ) -> Result<Plan, String> {
        let mut plan = Plan::new(plan_response.title, plan_response.overview);

        for plan_phase in plan_response.phases {
            let mut phase = Phase::new(plan_phase.name, plan_phase.emoji);

            for simple_task in plan_phase.tasks {
                let task_id = plan.generate_task_id();
                let task = Task::new(
                    task_id,
                    simple_task.title,
                    simple_task.tool,
                    simple_task.target,
                    simple_task.operation,
                )
                .with_dependencies(simple_task.dependencies);

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

        // Update history with response
        self.execution_queue.push_response(response.clone());
        Some(response)
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

                // Queue next ready tasks
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

    /// Handle user prompt processing
    fn handle_user_prompt(&mut self, content: &str) -> QueueResponse {
        let request_id = self.execution_queue.generate_id();

        // User prompts should be handled through AI planning only
        QueueResponse {
            request_id,
            success: false,
            content: format!(
                "User prompt '{}' requires AI planning - no dummy processing available",
                content
            ),
            completed_task_ids: Vec::new(),
        }
    }

    /// Handle task execution
    fn handle_task_execution(&mut self, task: &Task) -> QueueResponse {
        let request_id = self.execution_queue.generate_id();

        // Simulate task execution
        let result = format!(
            "Executed task '{}' using tool '{}' on target '{}'",
            task.title, task.tool, task.target
        );

        QueueResponse {
            request_id,
            success: true,
            content: result,
            completed_task_ids: vec![task.id],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_planner_creation() {
        let planner = TaskPlanner::new();
        assert_eq!(planner.active_plans.len(), 0);
        assert!(!planner.has_pending_work());
    }

    #[test]
    fn test_request_processing() {
        let mut planner = TaskPlanner::new();
        planner.add_user_prompt("Test prompt".to_string(), 3);

        assert!(planner.has_pending_work());

        let response = planner.process_next_request().unwrap();
        assert!(!response.success); // Should fail without LLM client
        assert!(response.content.contains("requires AI planning"));
    }
}
