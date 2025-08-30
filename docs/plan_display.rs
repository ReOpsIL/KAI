use std::fmt;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionPlan {
    pub title: String,
    pub overview: String,
    pub phases: Vec<Phase>,
    pub expected_outcome: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Phase {
    pub name: String,
    pub emoji: String,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Action {
    pub id: usize,
    pub title: String,
    pub tool: String,
    pub target: String,
    pub operation: String,
    pub purpose: String,
    pub success_criteria: String,
    pub dependencies: Vec<usize>,
    pub status: ActionStatus,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ActionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (icon, color) = match self {
            ActionStatus::Pending => ("⏳", "\x1b[33m"),     // Yellow
            ActionStatus::InProgress => ("🔄", "\x1b[36m"),  // Cyan
            ActionStatus::Completed => ("✅", "\x1b[32m"),   // Green
            ActionStatus::Failed => ("❌", "\x1b[31m"),      // Red
        };
        write!(f, "{}{}{}\x1b[0m", color, icon, self.as_str())
    }
}

impl ActionStatus {
    fn as_str(&self) -> &str {
        match self {
            ActionStatus::Pending => " Pending",
            ActionStatus::InProgress => " In Progress",
            ActionStatus::Completed => " Completed",
            ActionStatus::Failed => " Failed",
        }
    }
}

impl fmt::Display for ActionPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\x1b[1m\x1b[34m╔══════════════════════════════════════════════════════════════════════╗\x1b[0m")?;
        writeln!(f, "\x1b[1m\x1b[34m║\x1b[0m \x1b[1m🎯 {:<62}\x1b[0m \x1b[1m\x1b[34m║\x1b[0m", self.title)?;
        writeln!(f, "\x1b[1m\x1b[34m╚══════════════════════════════════════════════════════════════════════╝\x1b[0m")?;
        writeln!(f)?;
        
        // Overview section
        writeln!(f, "\x1b[1m📋 Overview\x1b[0m")?;
        writeln!(f, "{}", wrap_text(&self.overview, 210, "   "))?;
        writeln!(f)?;

        // Progress summary
        let total_actions: usize = self.phases.iter().map(|p| p.actions.len()).sum();
        let completed_actions: usize = self.phases.iter()
            .flat_map(|p| &p.actions)
            .filter(|a| a.status == ActionStatus::Completed)
            .count();
        
        writeln!(f, "\x1b[1m📊 Progress Summary\x1b[0m")?;
        writeln!(f, "   Total Actions: {} | Completed: {} | Remaining: {}", 
                 total_actions, completed_actions, total_actions - completed_actions)?;
        
        let progress_bar = create_progress_bar(completed_actions, total_actions, 50);
        writeln!(f, "   {}", progress_bar)?;
        writeln!(f)?;

        // Phases
        for phase in &self.phases {
            writeln!(f, "\x1b[1m{} {}\x1b[0m", phase.emoji, phase.name)?;
            writeln!(f, "{}", "─".repeat(210))?;
            
            for action in &phase.actions {
                writeln!(f, "{}", action)?;
                writeln!(f)?;
            }
        }

        // Expected outcome
        writeln!(f, "\x1b[1m📈 Expected Outcome\x1b[0m")?;
        writeln!(f, "{}", wrap_text(&self.expected_outcome, 210, "   "))?;
        
        Ok(())
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "   \x1b[1m▶ Action {}: {}\x1b[0m {}", self.id, self.title, self.status)?;
        writeln!(f, "     \x1b[36m🔧 Tool:\x1b[0m {}", self.tool)?;
        writeln!(f, "     \x1b[35m🎯 Target:\x1b[0m {}", self.target)?;
        writeln!(f, "     \x1b[33m⚙️  Operation:\x1b[0m {}", wrap_text(&self.operation, 60, "        "))?;
        writeln!(f, "     \x1b[32m💡 Purpose:\x1b[0m {}", wrap_text(&self.purpose, 60, "        "))?;
        writeln!(f, "     \x1b[34m✓ Success Criteria:\x1b[0m {}", wrap_text(&self.success_criteria, 60, "        "))?;
        
        if !self.dependencies.is_empty() {
            let deps = self.dependencies.iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(f, "     \x1b[31m🔗 Dependencies:\x1b[0m Actions [{}]", deps)?;
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
    fn test_action_status_display() {
        assert_eq!(format!("{}", ActionStatus::Pending), "\x1b[33m⏳ Pending\x1b[0m");
        assert_eq!(format!("{}", ActionStatus::Completed), "\x1b[32m✅ Completed\x1b[0m");
    }

    #[test]
    fn test_progress_bar() {
        let bar = create_progress_bar(3, 10, 20);
        assert!(bar.contains("██████"));
        assert!(bar.contains("░"));
        assert!(bar.contains("(3/10)"));
    }
}