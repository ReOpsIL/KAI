use serde::{Deserialize, Serialize};
use crate::planer::task::{Task, TaskStatus};
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
        self.tasks.iter()
            .filter(|task| task.is_ready_to_execute(completed_task_ids))
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        self.tasks.iter().all(|task| task.status == TaskStatus::Completed)
    }
}

/// Core plan structure organizing tasks into phases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub title: String,
    pub overview: String,
    pub phases: Vec<Phase>,
    pub next_task_id: usize,
}

impl Plan {
    pub fn new(title: String, overview: String) -> Self {
        Self {
            title,
            overview,
            phases: Vec::new(),
            next_task_id: 1,
        }
    }

    pub fn add_phase(&mut self, phase: Phase) {
        self.phases.push(phase);
    }

    pub fn generate_task_id(&mut self) -> usize {
        let id = self.next_task_id;
        self.next_task_id += 1;
        id
    }

    pub fn get_all_tasks(&self) -> Vec<&Task> {
        self.phases.iter()
            .flat_map(|phase| &phase.tasks)
            .collect()
    }

    pub fn get_all_tasks_mut(&mut self) -> Vec<&mut Task> {
        self.phases.iter_mut()
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
        self.phases.iter()
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
        let completed = self.get_all_tasks()
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
        writeln!(f, "\x1b[1m\x1b[34m║\x1b[0m \x1b[1m🎯 {:<62}\x1b[0m \x1b[1m\x1b[34m║\x1b[0m", self.title)?;
        writeln!(f, "\x1b[1m\x1b[34m╚══════════════════════════════════════════════════════════════════════╝\x1b[0m")?;
        writeln!(f)?;
        
        // Overview section
        writeln!(f, "\x1b[1m📋 Overview\x1b[0m")?;
        writeln!(f, "{}", wrap_text(&self.overview, 70, "   "))?;
        writeln!(f)?;

        // Progress summary
        let (completed, total) = self.get_progress();
        
        writeln!(f, "\x1b[1m📊 Progress Summary\x1b[0m")?;
        writeln!(f, "   Total Tasks: {} | Completed: {} | Remaining: {}", 
                 total, completed, total - completed)?;
        
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
    
    format!("\x1b[32m{}\x1b[37m{}\x1b[0m ({}/{})", 
            completed_bar, remaining_bar, completed, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_creation() {
        let plan = Plan::new(
            "Test Plan".to_string(),
            "A simple test plan".to_string(),
        );
        
        assert_eq!(plan.title, "Test Plan");
        assert_eq!(plan.next_task_id, 1);
        assert!(plan.phases.is_empty());
    }

    #[test]
    fn test_phase_with_tasks() {
        let mut phase = Phase::new("Analysis".to_string(), "🔍".to_string());
        let task = Task::new(
            1,
            "Analyze code".to_string(),
            "read".to_string(),
            "src/main.rs".to_string(),
            "examine structure".to_string(),
        );
        
        phase.add_task(task);
        assert_eq!(phase.tasks.len(), 1);
        assert!(!phase.is_complete());
    }

    #[test]
    fn test_plan_task_management() {
        let mut plan = Plan::new(
            "Development Plan".to_string(),
            "Build a feature".to_string(),
        );
        
        let id1 = plan.generate_task_id();
        let id2 = plan.generate_task_id();
        
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(plan.next_task_id, 3);
    }
}