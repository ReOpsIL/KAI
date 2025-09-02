use crate::planer::plan::Plan;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the status of a task
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Decomposed,
}

/// Represents a tool call with its arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,      // Name of tool to execute
    pub target: String,    // target file or directory to write / delete / list / search / grep etc
    pub operation: String, // Operation description
    pub content: String,   //File content for writing / replacing
}

/// Defines what a task executes: either a direct tool call or a sub-plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskExecution {
    ToolCall(ToolCall),
    SubPlan(Plan),
}

/// Core task structure with essential attributes only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: usize,
    pub title: String,
    pub execution: TaskExecution,
    pub dependencies: Vec<usize>,
    pub status: TaskStatus,
}

impl Task {
    pub fn new_tool_task(
        id: usize,
        title: String,
        tool: String,
        target: String,
        operation: String,
        content: String,
    ) -> Self {
        Self {
            id,
            title,
            execution: TaskExecution::ToolCall(ToolCall {
                tool,
                target,
                operation,
                content,
            }),
            dependencies: Vec::new(),
            status: TaskStatus::Pending,
        }
    }

    pub fn new_sub_plan_task(id: usize, title: String, plan: Plan) -> Self {
        Self {
            id,
            title,
            execution: TaskExecution::SubPlan(plan),
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
        self.status == TaskStatus::Pending
            && self
                .dependencies
                .iter()
                .all(|dep| completed_tasks.contains(dep))
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (icon, color) = match self {
            TaskStatus::Pending => ("⏳", "\x1b[33m"),    // Yellow
            TaskStatus::InProgress => ("🔄", "\x1b[36m"), // Cyan
            TaskStatus::Completed => ("✅", "\x1b[32m"),  // Green
            TaskStatus::Failed => ("❌", "\x1b[31m"),     // Red
            TaskStatus::Decomposed => ("🧬", "\x1b[35m"), // Magenta
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
            TaskStatus::Decomposed => " Decomposed",
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "   \x1b[1m▶ Task {}: {}\x1b[0m {}",
            self.id, self.title, self.status
        )?;

        match &self.execution {
            TaskExecution::ToolCall(tool_call) => {
                writeln!(f, "     \x1b[36m🔧 Tool:\x1b[0m {}", tool_call.tool)?;
                writeln!(f, "     \x1b[35m🎯 Target:\x1b[0m {}", tool_call.target)?;
                writeln!(
                    f,
                    "     \x1b[33m🚜  Operation:\x1b[0m {}",
                    wrap_text(&tool_call.operation, 60, "        ")
                )?;
                writeln!(
                    f,
                    "     \x1b[33m🏃  Content:\x1b[0m {}",
                    wrap_text(&tool_call.content, 60, "        ")
                )?;
            }
            TaskExecution::SubPlan(plan) => {
                writeln!(f, "     \x1b[36m📋 Sub-Plan:\x1b[0m {}", plan.title)?;
                let (completed, total) = plan.get_progress();
                writeln!(
                    f,
                    "        Progress: {}/{}\x1b[0m tasks completed",
                    completed, total
                )?;
            }
        }

        if !self.dependencies.is_empty() {
            let deps = self
                .dependencies
                .iter()
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
