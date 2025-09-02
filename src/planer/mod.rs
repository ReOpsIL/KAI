//! Simplified Planner System
//!
//! A minimal implementation of the planner system focusing on essential logic and attributes.
//! This module provides core functionality for task planning, execution queue management,
//! and basic LLM integration without unnecessary complexity.

pub mod plan;
pub mod queue;

pub mod task;
pub mod task_executor;
pub mod task_planner;
pub mod task_processor;

// Re-export main types for convenience
pub use plan::{ExecutionMetadata, Phase, Plan, PlanContext, TaskResult};
pub use queue::{ExecutionQueue, QueueRequest, QueueResponse};
pub use task::{Task, TaskStatus};
pub use task_executor::TaskExecutor;
pub use task_planner::TaskPlanner;
pub use task_processor::{TaskExecutionContext, TaskProcessor};

use crate::llm::OpenRouterClient;
use std::path::Path;
use std::sync::Arc;

/// Main planner facade that combines all components with LLM-powered task processing
pub struct Planner {
    pub task_planner: TaskPlanner,
    pub task_processor: Option<TaskProcessor>,
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

impl Planner {
    pub fn new() -> Self {
        Self {
            task_planner: TaskPlanner::new(),
            task_processor: None,
        }
    }

    /// Create a new planner with LLM client for advanced planning
    pub fn with_llm_client(llm_client: Arc<OpenRouterClient>) -> Self {
        Self {
            task_planner: TaskPlanner::with_llm_client(llm_client.clone()),
            task_processor: Some(
                TaskProcessor::new(llm_client).with_task_executor(TaskExecutor::new()),
            ),
        }
    }

    /// Set the working directory for task execution
    pub fn with_workdir<P: AsRef<Path>>(mut self, workdir: P) -> Self {
        if let Some(processor) = self.task_processor.as_mut() {
            processor.task_executor = processor.task_executor.clone().with_workdir(workdir);
        }
        self
    }

    /// Execute a task with full context awareness using LLM processing
    pub async fn execute_task_with_context(
        &self,
        task: &Task,
        main_context: &crate::context::Context,
        plan: &Plan,
    ) -> Result<TaskResult, String> {
        if let Some(processor) = &self.task_processor {
            // Gather dependency results
            let dependency_results = task
                .dependencies
                .iter()
                .filter_map(|&dep_id| plan.get_task_result(dep_id))
                .map(|result| (result.task_id, result.clone()))
                .collect();

            let execution_context = TaskExecutionContext {
                main_context: main_context.clone(),
                plan_context: plan.plan_context.clone(),
                dependency_results,
                current_task: task.clone(),
            };

            processor
                .execute_task_with_context(task, execution_context)
                .await
        } else {
            // Fallback to basic execution without LLM processing
            let task_executor = TaskExecutor::new();
            let response = task_executor
                .execute_task(task, main_context, &plan.plan_context)
                .await;
            Ok(TaskResult {
                task_id: task.id,
                tool_result: response.tool_result,
                llm_processed_result: "No LLM processing available".to_string(),
                extracted_variables: std::collections::HashMap::new(),
                success: response.success,
                executed_at: chrono::Utc::now(),
            })
        }
    }

    /// Create a sub-plan with inherited context from parent plan
    pub fn create_sub_plan_with_context(
        &self,
        title: String,
        overview: String,
        parent_plan: &Plan,
    ) -> Plan {
        Plan::with_parent_context(title, overview, Some(parent_plan.plan_context.clone()))
    }

    /// Execute an entire plan with context-aware processing
    pub async fn execute_plan_with_context(
        &mut self,
        plan: &mut Plan,
        main_context: &crate::context::Context,
    ) -> Result<Vec<TaskResult>, String> {
        let mut results = Vec::new();

        for i in 0..plan.phases.len() {
            let phase = &plan.phases[i].clone();
            for task in &phase.tasks {
                // Check dependencies are satisfied
                let deps_satisfied = task
                    .dependencies
                    .iter()
                    .all(|&dep_id| plan.get_task_result(dep_id).map_or(false, |r| r.success));

                if !deps_satisfied {
                    continue;
                }

                match self
                    .execute_task_with_context(task, main_context, plan)
                    .await
                {
                    Ok(result) => {
                        plan.add_task_result(result.clone());
                        results.push(result);
                    }
                    Err(e) => {
                        let error_result = TaskResult {
                            task_id: task.id,
                            tool_result: format!("Execution failed: {}", e),
                            llm_processed_result: format!("Task failed with error: {}", e),
                            extracted_variables: std::collections::HashMap::new(),
                            success: false,
                            executed_at: chrono::Utc::now(),
                        };
                        plan.add_task_result(error_result.clone());
                        results.push(error_result);
                    }
                }
            }
        }

        Ok(results)
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        if let Some(processor) = self.task_processor.as_mut() {
            processor.task_executor = processor.task_executor.clone().with_verbose(verbose);
        }
        self
    }

    // /// Create an advanced plan using LLM and start processing
    // pub async fn create_and_execute_advanced_plan(&mut self, user_input: &str) -> Result<String, String> {
    //     let result = self.task_planner.create_advanced_plan(user_input).await?;

    //     // Process all pending requests
    //     let mut responses = Vec::new();
    //     while self.task_planner.has_pending_work() {
    //         if let Some(response) = self.task_planner.process_next_request() {
    //             responses.push(response);
    //         }
    //     }

    //     Ok(format!("{}\nProcessed {} requests", result, responses.len()))
    // }

    /// Create an advanced plan using LLM with context integration and execute it.
    pub async fn create_and_execute_advanced_plan_with_context(
        &mut self,
        user_input: &str,
        context: &crate::context::Context,
    ) -> Result<String, String> {
        self.task_planner
            .create_advanced_plan_with_context(user_input, context)
            .await
    }

    /// Get overall system status
    pub fn get_status(&self) -> String {
        format!("Planner Status:\n{}", self.task_planner.get_status(),)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_creation() {
        let planner = Planner::new();
        assert_eq!(
            planner.task_planner.get_status(),
            "Active Plans: 0, Pending Requests: 0"
        );
    }

    #[test]
    fn test_verbose_mode() {
        let planner = Planner::new().with_verbose(true);
        if let Some(processor) = planner.task_processor {
            assert!(processor.task_executor.verbose);
        }
    }
}
