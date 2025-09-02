use crate::planer::task::{Task, TaskStatus};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a phase in a plan with grouped tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub emoji: String,
    pub tasks: Vec<Task>,
}

impl Phase {
    pub fn new(name: String, emoji: String) -> Self {
        Self {
            name,
            emoji,
            tasks: Vec::new(),
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn get_ready_tasks(&self, completed_task_ids: &[usize]) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|task| task.is_ready_to_execute(completed_task_ids))
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        self.tasks
            .iter()
            .all(|task| task.status == TaskStatus::Completed)
    }
}

use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Plan-specific temporary context that accumulates during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanContext {
    /// Accumulated results from executed tasks, keyed by task ID
    pub task_results: HashMap<usize, TaskResult>,
    /// Plan-scoped variables and state
    pub plan_variables: HashMap<String, String>,
    /// Parent plan context for sub-plans (allows inheritance)
    pub parent_context: Option<Box<PlanContext>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Plan execution metadata
    pub execution_metadata: ExecutionMetadata,
}

/// Result of a task execution including LLM processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID that generated this result
    pub task_id: usize,
    /// Original tool execution result
    pub tool_result: String,
    /// LLM-processed result with context awareness
    pub llm_processed_result: String,
    /// Any extracted data or variables from the result
    pub extracted_variables: HashMap<String, String>,
    /// Success status
    pub success: bool,
    /// Execution timestamp
    pub executed_at: DateTime<Utc>,
}

/// Metadata about plan execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Total tasks in plan
    pub total_tasks: usize,
    /// Completed tasks count
    pub completed_tasks: usize,
    /// Current phase being executed
    pub current_phase: Option<String>,
    /// Plan start time
    pub started_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

impl PlanContext {
    /// Create new plan context
    pub fn new(parent: Option<PlanContext>) -> Self {
        let now = Utc::now();
        Self {
            task_results: HashMap::new(),
            plan_variables: HashMap::new(),
            parent_context: parent.map(Box::new),
            created_at: now,
            execution_metadata: ExecutionMetadata {
                total_tasks: 0,
                completed_tasks: 0,
                current_phase: None,
                started_at: now,
                last_activity: now,
            },
        }
    }

    /// Add task result to context
    pub fn add_task_result(&mut self, result: TaskResult) {
        self.task_results.insert(result.task_id, result);
        self.execution_metadata.completed_tasks += 1;
        self.execution_metadata.last_activity = Utc::now();
    }

    /// Get task result by ID, checking parent contexts
    pub fn get_task_result(&self, task_id: usize) -> Option<&TaskResult> {
        self.task_results
            .get(&task_id)
            .or_else(|| self.parent_context.as_ref()?.get_task_result(task_id))
    }

    /// Get all available task results including from parent contexts
    pub fn get_all_available_results(&self) -> HashMap<usize, &TaskResult> {
        let mut results = HashMap::new();

        // Add parent results first (can be overridden by current level)
        if let Some(parent) = &self.parent_context {
            results.extend(parent.get_all_available_results());
        }

        // Add current level results
        for (id, result) in &self.task_results {
            results.insert(*id, result);
        }

        results
    }

    /// Set plan variable
    pub fn set_variable(&mut self, key: String, value: String) {
        self.plan_variables.insert(key, value);
        self.execution_metadata.last_activity = Utc::now();
    }

    /// Get plan variable, checking parent contexts
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.plan_variables
            .get(key)
            .or_else(|| self.parent_context.as_ref()?.get_variable(key))
    }

    /// Get the file content from a previous read_file task
    pub fn get_file_content_from_task(&self, task_id: usize) -> Option<String> {
        let task_result = self.get_task_result(task_id)?;
        let json_result: serde_json::Value = serde_json::from_str(&task_result.tool_result).ok()?;
        json_result["data"]["content"]
            .as_str()
            .map(|s| s.to_string())
    }

    /// Format context for LLM consumption
    pub fn format_for_llm(&self, task_dependencies: &[usize]) -> String {
        let mut context_parts = Vec::new();

        // Add plan execution status
        context_parts.push(format!(
            "## Plan Execution Context\n- Progress: {}/{} tasks completed\n- Current Phase: {}\n- Started: {}",
            self.execution_metadata.completed_tasks,
            self.execution_metadata.total_tasks,
            self.execution_metadata.current_phase.as_deref().unwrap_or("Unknown"),
            self.execution_metadata.started_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Add relevant task results based on dependencies
        if !task_dependencies.is_empty() {
            let mut dependency_results = Vec::new();
            for &dep_id in task_dependencies {
                if let Some(result) = self.get_task_result(dep_id) {
                    dependency_results.push(format!(
                        "### Task {} Result\n- Success: {}\n- LLM Analysis: {}\n- Variables: {:?}",
                        dep_id,
                        result.success,
                        Self::truncate_text(&result.llm_processed_result, 200),
                        result.extracted_variables
                    ));
                }
            }

            if !dependency_results.is_empty() {
                context_parts.push(format!(
                    "## Dependency Task Results\n{}",
                    dependency_results.join("\n\n")
                ));
            }
        }

        // Add plan variables
        if !self.plan_variables.is_empty() {
            let vars: Vec<String> = self
                .plan_variables
                .iter()
                .map(|(k, v)| format!("- {}: {}", k, Self::truncate_text(v, 100)))
                .collect();
            context_parts.push(format!("## Plan Variables\n{}", vars.join("\n")));
        }

        context_parts.join("\n\n")
    }

    /// Truncate text for context display
    fn truncate_text(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[..max_len.saturating_sub(3)])
        }
    }
}

/// Core plan structure organizing tasks into phases with temporary execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub title: String,
    pub overview: String,
    pub phases: Vec<Phase>,
    pub next_task_id: usize,
    /// Temporary execution context for this plan
    pub plan_context: PlanContext,
}

impl Plan {
    pub fn new(title: String, overview: String) -> Self {
        Self::with_parent_context(title, overview, None)
    }

    /// Create new plan with optional parent context (for sub-plans)
    pub fn with_parent_context(
        title: String,
        overview: String,
        parent_context: Option<PlanContext>,
    ) -> Self {
        let mut plan_context = PlanContext::new(parent_context);
        plan_context.execution_metadata.total_tasks = 0; // Will be updated when phases are added

        Self {
            id: "".to_string(), // Will be set by the planner
            title,
            overview,
            phases: Vec::new(),
            next_task_id: 1,
            plan_context,
        }
    }

    /// Add task result to plan context
    pub fn add_task_result(&mut self, result: TaskResult) {
        self.plan_context.add_task_result(result);
    }

    /// Get task result from plan context
    pub fn get_task_result(&self, task_id: usize) -> Option<&TaskResult> {
        self.plan_context.get_task_result(task_id)
    }

    /// Set plan variable
    pub fn set_plan_variable(&mut self, key: String, value: String) {
        self.plan_context.set_variable(key, value);
    }

    /// Get plan variable
    pub fn get_plan_variable(&self, key: &str) -> Option<&String> {
        self.plan_context.get_variable(key)
    }

    /// Update total task count when phases are added
    pub fn update_task_count(&mut self) {
        let total_tasks = self.phases.iter().map(|p| p.tasks.len()).sum();
        self.plan_context.execution_metadata.total_tasks = total_tasks;
    }

    pub fn add_phase(&mut self, phase: Phase) {
        self.phases.push(phase);
        self.update_task_count();
    }

    /// Adds a task to a phase, defaulting to the last phase if none is specified.
    pub fn add_task_to_phase(
        &mut self,
        task: &Task,
        phase_name: Option<&str>,
    ) -> Result<(), String> {
        let phase = match phase_name {
            Some(name) => self
                .phases
                .iter_mut()
                .find(|p| p.name == name)
                .ok_or_else(|| format!("Phase '{}' not found in plan", name)),
            None => self
                .phases
                .last_mut()
                .ok_or_else(|| "No phases in plan to add task to".to_string()),
        }?;
        phase.add_task(task.clone());
        Ok(())
    }

    pub fn generate_task_id(&mut self) -> usize {
        let id = self.next_task_id;
        self.next_task_id += 1;
        id
    }

    pub fn get_all_tasks(&self) -> Vec<&Task> {
        self.phases.iter().flat_map(|phase| &phase.tasks).collect()
    }

    pub fn get_all_tasks_mut(&mut self) -> Vec<&mut Task> {
        self.phases
            .iter_mut()
            .flat_map(|phase| &mut phase.tasks)
            .collect()
    }

    pub fn get_completed_task_ids(&self) -> Vec<usize> {
        self.get_all_tasks()
            .iter()
            .filter(|task| task.status == TaskStatus::Completed)
            .map(|task| task.id)
            .collect()
    }

    pub fn get_next_ready_tasks(&self) -> Vec<&Task> {
        let completed_ids = self.get_completed_task_ids();
        self.phases
            .iter()
            .flat_map(|phase| phase.get_ready_tasks(&completed_ids))
            .collect()
    }

    pub fn find_task_by_id(&mut self, id: usize) -> Option<&mut Task> {
        self.get_all_tasks_mut()
            .into_iter()
            .find(|task| task.id == id)
    }

    pub fn is_complete(&self) -> bool {
        self.phases.iter().all(|phase| phase.is_complete())
    }

    pub fn get_progress(&self) -> (usize, usize) {
        let completed = self
            .get_all_tasks()
            .iter()
            .filter(|task| task.status == TaskStatus::Completed)
            .count();
        let total = self.get_all_tasks().len();
        (completed, total)
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\x1b[1m{} {}\x1b[0m", self.emoji, self.name)?;
        writeln!(f, "{}", "─".repeat(70))?;

        for task in &self.tasks {
            writeln!(f, "{}", task)?;
            writeln!(f)?;
        }

        Ok(())
    }
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\x1b[1m\x1b[34m╔══════════════════════════════════════════════════════════════════════╗\x1b[0m")?;
        writeln!(
            f,
            "\x1b[1m\x1b[34m║\x1b[0m \x1b[1m🎯 {:<64}\x1b[0m \x1b[1m\x1b[34m║\x1b[0m",
            self.title
        )?;
        writeln!(f, "\x1b[1m\x1b[34m╚══════════════════════════════════════════════════════════════════════╝\x1b[0m")?;
        writeln!(f)?;

        // Overview section
        writeln!(f, "\x1b[1m📋 Overview\x1b[0m")?;
        writeln!(f, "{}", wrap_text(&self.overview, 70, "   "))?;
        writeln!(f)?;

        // Progress summary
        let (completed, total) = self.get_progress();

        writeln!(f, "\x1b[1m📊 Progress Summary\x1b[0m")?;
        writeln!(
            f,
            "   Total Tasks: {} | Completed: {} | Remaining: {}",
            total,
            completed,
            total - completed
        )?;

        let progress_bar = create_progress_bar(completed, total, 50);
        writeln!(f, "   {}", progress_bar)?;
        writeln!(f)?;

        // Phases
        for phase in &self.phases {
            writeln!(f, "{}", phase)?;
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

fn create_progress_bar(completed: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return "█".repeat(width);
    }

    let progress = (completed as f64 / total as f64 * width as f64) as usize;
    let completed_bar = "█".repeat(progress);
    let remaining_bar = "░".repeat(width - progress);

    format!(
        "\x1b[32m{}\x1b[37m{}\x1b[0m ({}/{})",
        completed_bar, remaining_bar, completed, total
    )
}
