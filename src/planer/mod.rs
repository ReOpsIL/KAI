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

// Re-export main types for convenience
pub use plan::{Phase, Plan};
pub use queue::{ExecutionQueue, QueueRequest, QueueResponse};
pub use task::{Task, TaskStatus};
pub use task_executor::TaskExecutor;
pub use task_planner::TaskPlanner;

use crate::llm::OpenRouterClient;
use std::path::Path;
use std::sync::Arc;

/// Main planner facade that combines all components
pub struct Planner {
    pub task_planner: TaskPlanner,
    pub task_executor: TaskExecutor,
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
            task_executor: TaskExecutor::new(),
        }
    }

    /// Create a new planner with LLM client for advanced planning
    pub fn with_llm_client(llm_client: Arc<OpenRouterClient>) -> Self {
        Self {
            task_planner: TaskPlanner::with_llm_client(llm_client),
            task_executor: TaskExecutor::new(),
        }
    }

    /// Set the working directory for task execution
    pub fn with_workdir<P: AsRef<Path>>(mut self, workdir: P) -> Self {
        self.task_executor = self.task_executor.with_workdir(workdir);
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.task_executor = self.task_executor.with_verbose(verbose);
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
        format!(
            "Planner Status:\n{}\nExecutor: {}",
            self.task_planner.get_status(),
            if self.task_executor.verbose {
                "Verbose"
            } else {
                "Quiet"
            }
        )
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
        assert!(planner.task_executor.verbose);
    }
}
