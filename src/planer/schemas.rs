use serde::{Deserialize, Serialize};

/// Minimal schema for task decomposition responses from LLM
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskDecomposition {
    pub analysis: String,
    pub is_executable: bool,
    pub tasks: Option<Vec<SimpleTask>>,
}

/// Simple task structure for LLM communication
#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleTask {
    pub title: String,
    pub tool: String,
    pub target: String,
    pub operation: String,
    #[serde(default)]
    pub dependencies: Vec<usize>,
}

/// Minimal plan response from LLM
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanResponse {
    pub title: String,
    pub overview: String,
    pub phases: Vec<PlanPhase>,
}

/// Simple phase structure for LLM communication
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanPhase {
    pub name: String,
    pub emoji: String,
    #[serde(alias = "actions")]
    pub tasks: Vec<SimpleTask>,
}

/// Helper to create JSON prompts for LLM
pub fn create_json_prompt(instruction: &str) -> String {
    format!(
        "{}\n\n\
        IMPORTANT: Respond with valid JSON only, following this schema:\n\
        {{\n\
          \"title\": \"Plan title\",\n\
          \"overview\": \"Plan description\",\n\
          \"phases\": [\n\
            {{\n\
              \"name\": \"Analysis\",\n\
              \"emoji\": \"🔍\",\n\
              \"tasks\": [\n\
                {{\n\
                  \"title\": \"Task title\",\n\
                  \"tool\": \"read\",\n\
                  \"target\": \"file.txt\",\n\
                  \"operation\": \"examine content\",\n\
                  \"dependencies\": []\n\
                }}\n\
              ]\n\
            }}\n\
          ]\n\
        }}\n\n\
        Do not include any text before or after the JSON.",
        instruction
    )
}

/// Schema examples for common operations
pub mod examples {
    pub const BASIC_PLAN: &str = r#"{
  "title": "Development Plan",
  "overview": "Complete the requested development task",
  "phases": [
    {
      "name": "Analysis",
      "emoji": "🔍",
      "tasks": [
        {
          "title": "Analyze requirements",
          "tool": "read",
          "target": "project files",
          "operation": "understand the current state",
          "dependencies": []
        }
      ]
    },
    {
      "name": "Implementation", 
      "emoji": "🛠️",
      "tasks": [
        {
          "title": "Implement changes",
          "tool": "edit",
          "target": "source files",
          "operation": "make the required modifications",
          "dependencies": [1]
        }
      ]
    },
    {
      "name": "Verification",
      "emoji": "✅",
      "tasks": [
        {
          "title": "Test changes",
          "tool": "bash",
          "target": "test suite",
          "operation": "run tests to verify functionality",
          "dependencies": [2]
        }
      ]
    }
  ]
}"#;

    pub const TASK_DECOMPOSITION: &str = r#"{
  "analysis": "The task requires multiple steps to complete",
  "is_executable": false,
  "tasks": [
    {
      "title": "First step",
      "tool": "read",
      "target": "file.txt",
      "operation": "examine the file",
      "dependencies": []
    }
  ]
}"#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_response_deserialization() {
        let json = examples::BASIC_PLAN;
        let result: Result<PlanResponse, _> = serde_json::from_str(json);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.title, "Development Plan");
        assert_eq!(plan.phases.len(), 3);
        assert_eq!(plan.phases[0].name, "Analysis");
    }

    #[test]
    fn test_task_decomposition_deserialization() {
        let json = examples::TASK_DECOMPOSITION;
        let result: Result<TaskDecomposition, _> = serde_json::from_str(json);
        assert!(result.is_ok());
        
        let decomposition = result.unwrap();
        assert!(!decomposition.is_executable);
        assert!(decomposition.tasks.is_some());
    }

    #[test]
    fn test_json_prompt_creation() {
        let prompt = create_json_prompt("Create a plan for building a web app");
        assert!(prompt.contains("Create a plan for building a web app"));
        assert!(prompt.contains("valid JSON only"));
        assert!(prompt.contains("phases"));
    }
}