use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the status of a task/action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Core task/action structure with essential attributes only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: usize,
    pub title: String,
    pub tool: String,
    pub target: String,
    pub operation: String,
    pub dependencies: Vec<usize>,
    pub status: TaskStatus,
}

impl Task {
    pub fn new(id: usize, title: String, tool: String, target: String, operation: String) -> Self {
        Self {
            id,
            title,
            tool,
            target,
            operation,
            dependencies: Vec::new(),
            status: TaskStatus::Pending,
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<usize>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
    }

    pub fn is_ready_to_execute(&self, completed_tasks: &[usize]) -> bool {
        self.status == TaskStatus::Pending && 
        self.dependencies.iter().all(|dep| completed_tasks.contains(dep))
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (icon, color) = match self {
            TaskStatus::Pending => ("⏳", "\x1b[33m"),     // Yellow
            TaskStatus::InProgress => ("🔄", "\x1b[36m"),  // Cyan
            TaskStatus::Completed => ("✅", "\x1b[32m"),   // Green
            TaskStatus::Failed => ("❌", "\x1b[31m"),      // Red
        };
        write!(f, "{}{}{}\x1b[0m", color, icon, self.as_str())
    }
}

impl TaskStatus {
    fn as_str(&self) -> &str {
        match self {
            TaskStatus::Pending => " Pending",
            TaskStatus::InProgress => " In Progress",
            TaskStatus::Completed => " Completed",
            TaskStatus::Failed => " Failed",
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "   \x1b[1m▶ Task {}: {}\x1b[0m {}", self.id, self.title, self.status)?;
        writeln!(f, "     \x1b[36m🔧 Tool:\x1b[0m {}", self.tool)?;
        writeln!(f, "     \x1b[35m🎯 Target:\x1b[0m {}", self.target)?;
        writeln!(f, "     \x1b[33m⚙️  Operation:\x1b[0m {}", wrap_text(&self.operation, 60, "        "))?;
        
        if !self.dependencies.is_empty() {
            let deps = self.dependencies.iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(f, "     \x1b[31m🔗 Dependencies:\x1b[0m Tasks [{}]", deps)?;
        }
        
        Ok(())
    }
}

fn wrap_text(text: &str, width: usize, indent: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut current_line = String::new();
    
    for word in words {
        if current_line.len() + word.len() + 1 > width {
            if !current_line.is_empty() {
                lines.push(format!("{}{}", indent, current_line.trim()));
                current_line.clear();
            }
        }
        
        if !current_line.is_empty() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }
    
    if !current_line.is_empty() {
        lines.push(format!("{}{}", indent, current_line.trim()));
    }
    
    if lines.is_empty() {
        format!("{}{}", indent, text)
    } else {
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(
            1,
            "Test task".to_string(),
            "bash".to_string(),
            "file.txt".to_string(),
            "create file".to_string(),
        );
        
        assert_eq!(task.id, 1);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.dependencies.is_empty());
    }

    #[test]
    fn test_task_ready_to_execute() {
        let mut task = Task::new(
            2,
            "Dependent task".to_string(),
            "edit".to_string(),
            "file.txt".to_string(),
            "modify file".to_string(),
        ).with_dependencies(vec![1]);
        
        // Not ready without dependency
        assert!(!task.is_ready_to_execute(&[]));
        
        // Ready when dependency is completed
        assert!(task.is_ready_to_execute(&[1]));
        
        // Not ready when in progress
        task.set_status(TaskStatus::InProgress);
        assert!(!task.is_ready_to_execute(&[1]));
    }
}