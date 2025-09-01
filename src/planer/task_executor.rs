use crate::planer::queue::QueueResponse;
use crate::planer::task::{Task, TaskStatus};
use crate::tools::{exec, file_system};
use std::path::{Path, PathBuf};

/// Simple task executor that simulates task execution
#[derive(Debug)]
pub struct TaskExecutor {
    pub verbose: bool,
    pub workdir: PathBuf,
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

    /// Execute a task and return the result
    pub async fn execute_task(&self, task: &Task) -> QueueResponse {
        if self.verbose {
            println!("Executing task: {} ({})", task.title, task.tool);
        }

        let result = match task.tool.as_str() {
            "read" | "read_file" => {
                let resolved_path = self.resolve_path(&task.target);
                file_system::FileSystemOperations::read_file(&self.path_to_string(&resolved_path))
            }
            "write" | "write_file" => {
                let resolved_path = self.resolve_path(&task.target);
                file_system::FileSystemOperations::write_file(
                    &self.path_to_string(&resolved_path),
                    &task.content,
                    None,
                )
            }
            "bash" | "run_shell" => {
                // For shell commands, change to workdir first
                let command_with_cd = format!("cd {} && {}", self.workdir.display(), task.content);
                exec::run_shell_command_tool(&command_with_cd)
            }
            "ls" | "list_directory" => {
                let resolved_path = if task.content.is_empty() || task.content == "." {
                    self.workdir.clone()
                } else {
                    self.resolve_path(&task.target)
                };
                file_system::FileSystemOperations::list_directory(
                    &self.path_to_string(&resolved_path),
                    None,
                    None,
                )
            }
            "grep" | "grep_files" => {
                let resolved_path = if task.target.is_empty() || task.target == "." {
                    self.workdir.clone()
                } else {
                    self.resolve_path(&task.target)
                };
                file_system::FileSystemOperations::grep_files(
                    &task.content,
                    &self.path_to_string(&resolved_path),
                    None,
                    None,
                    None,
                )
            }
            "find" | "find_files" => {
                let resolved_path = if task.target.is_empty() || task.target == "." {
                    self.workdir.clone()
                } else {
                    self.resolve_path(&task.target)
                };
                file_system::FileSystemOperations::find_files(
                    &self.path_to_string(&resolved_path),
                    None,
                    None,
                )
            }
            _ => {
                // This indicates a task that needs decomposition
                return QueueResponse {
                    request_id: format!("task_{}", task.id),
                    success: false,
                    content: format!("Decomposition needed for tool '{}'", task.tool),
                    completed_task_ids: vec![],
                    decomposed_tasks: None,
                };
            }
        };

        QueueResponse {
            request_id: format!("task_{}", task.id),
            success: result.success,
            content: serde_json::to_string_pretty(&result)
                .unwrap_or_else(|e| format!("Failed to serialize result: {}", e)),
            completed_task_ids: if result.success {
                vec![task.id]
            } else {
                vec![]
            },
            decomposed_tasks: None,
        }
    }

    /// Check if a task can be executed (dependencies met)
    pub fn can_execute(&self, task: &Task, completed_task_ids: &[usize]) -> bool {
        task.status == TaskStatus::Pending
            && task
                .dependencies
                .iter()
                .all(|dep| completed_task_ids.contains(dep))
    }

    /// Execute multiple tasks in dependency order
    pub async fn execute_batch(&self, tasks: &[Task]) -> Vec<QueueResponse> {
        let mut responses = Vec::new();
        let mut completed_ids = Vec::new();

        // Keep executing until all tasks are processed or no progress can be made
        let mut remaining_tasks: Vec<_> = tasks.iter().collect();

        while !remaining_tasks.is_empty() {
            let mut progress_made = false;
            let mut new_remaining = Vec::new();

            for task in remaining_tasks {
                if self.can_execute(task, &completed_ids) {
                    let response = self.execute_task(task).await;
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
