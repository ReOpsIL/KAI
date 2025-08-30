use crate::planer::task::{Task, TaskStatus};
use crate::planer::queue::QueueResponse;

/// Simple task executor that simulates task execution
#[derive(Debug)]
pub struct TaskExecutor {
    pub verbose: bool,
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskExecutor {
    pub fn new() -> Self {
        Self {
            verbose: false,
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Execute a task and return the result
    pub fn execute_task(&self, task: &Task) -> QueueResponse {
        if self.verbose {
            println!("Executing task: {} ({})", task.title, task.tool);
        }

        let result = match task.tool.as_str() {
            "read" => self.execute_read_task(task),
            "edit" => self.execute_edit_task(task),
            "bash" => self.execute_bash_task(task),
            "write" => self.execute_write_task(task),
            _ => format!("Unknown tool '{}' for task '{}'", task.tool, task.title),
        };

        QueueResponse {
            request_id: format!("task_{}", task.id),
            success: true,
            content: result,
            completed_task_ids: vec![task.id],
        }
    }

    /// Simulate reading a file or examining content
    fn execute_read_task(&self, task: &Task) -> String {
        format!(
            "Read operation completed:\n\
            - Target: {}\n\
            - Operation: {}\n\
            - Status: Successfully examined content",
            task.target,
            task.operation
        )
    }

    /// Simulate editing files
    fn execute_edit_task(&self, task: &Task) -> String {
        format!(
            "Edit operation completed:\n\
            - Target: {}\n\
            - Operation: {}\n\
            - Status: Successfully modified content",
            task.target,
            task.operation
        )
    }

    /// Simulate bash command execution
    fn execute_bash_task(&self, task: &Task) -> String {
        format!(
            "Bash operation completed:\n\
            - Target: {}\n\
            - Operation: {}\n\
            - Status: Command executed successfully",
            task.target,
            task.operation
        )
    }

    /// Simulate writing new files
    fn execute_write_task(&self, task: &Task) -> String {
        format!(
            "Write operation completed:\n\
            - Target: {}\n\
            - Operation: {}\n\
            - Status: Successfully created new content",
            task.target,
            task.operation
        )
    }

    /// Check if a task can be executed (dependencies met)
    pub fn can_execute(&self, task: &Task, completed_task_ids: &[usize]) -> bool {
        task.status == TaskStatus::Pending && 
        task.dependencies.iter().all(|dep| completed_task_ids.contains(dep))
    }

    /// Execute multiple tasks in dependency order
    pub fn execute_batch(&self, tasks: &[Task]) -> Vec<QueueResponse> {
        let mut responses = Vec::new();
        let mut completed_ids = Vec::new();

        // Keep executing until all tasks are processed or no progress can be made
        let mut remaining_tasks: Vec<_> = tasks.iter().collect();
        
        while !remaining_tasks.is_empty() {
            let mut progress_made = false;
            let mut new_remaining = Vec::new();

            for task in remaining_tasks {
                if self.can_execute(task, &completed_ids) {
                    let response = self.execute_task(task);
                    completed_ids.extend(&response.completed_task_ids);
                    responses.push(response);
                    progress_made = true;
                } else {
                    new_remaining.push(task);
                }
            }

            remaining_tasks = new_remaining;

            // Prevent infinite loops if dependencies can't be satisfied
            if !progress_made && !remaining_tasks.is_empty() {
                if self.verbose {
                    println!("Warning: Some tasks have unsatisfied dependencies");
                }
                break;
            }
        }

        responses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_executor_creation() {
        let executor = TaskExecutor::new();
        assert!(!executor.verbose);
    }

    #[test]
    fn test_task_execution() {
        let executor = TaskExecutor::new();
        let task = Task::new(
            1,
            "Read file".to_string(),
            "read".to_string(),
            "test.txt".to_string(),
            "examine content".to_string(),
        );

        let response = executor.execute_task(&task);
        assert!(response.success);
        assert!(response.content.contains("Read operation completed"));
        assert_eq!(response.completed_task_ids, vec![1]);
    }

    #[test]
    fn test_dependency_checking() {
        let executor = TaskExecutor::new();
        let task = Task::new(
            2,
            "Edit file".to_string(),
            "edit".to_string(),
            "test.txt".to_string(),
            "modify content".to_string(),
        ).with_dependencies(vec![1]);

        // Should not be able to execute without dependency
        assert!(!executor.can_execute(&task, &[]));
        
        // Should be able to execute with dependency satisfied
        assert!(executor.can_execute(&task, &[1]));
    }

    #[test]
    fn test_batch_execution() {
        let executor = TaskExecutor::new();
        let tasks = vec![
            Task::new(
                1,
                "First task".to_string(),
                "read".to_string(),
                "file1.txt".to_string(),
                "read file".to_string(),
            ),
            Task::new(
                2,
                "Second task".to_string(),
                "edit".to_string(),
                "file1.txt".to_string(),
                "modify file".to_string(),
            ).with_dependencies(vec![1]),
        ];

        let responses = executor.execute_batch(&tasks);
        assert_eq!(responses.len(), 2);
        
        // First response should be for task 1
        assert_eq!(responses[0].completed_task_ids, vec![1]);
        // Second response should be for task 2
        assert_eq!(responses[1].completed_task_ids, vec![2]);
    }
}