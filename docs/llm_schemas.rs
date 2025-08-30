use serde::{Deserialize, Serialize};

/// JSON schema for LLM task decomposition responses
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskDecompositionResponse {
    pub analysis: String,
    pub is_executable: bool,
    pub executable_action: Option<ExecutableAction>,
    pub sub_tasks: Option<Vec<SubTask>>,
    pub reasoning: String,
}

/// JSON schema for executable actions
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutableAction {
    pub tool: String,
    pub operation: String,
    pub target: String,
    pub parameters: Option<serde_json::Value>,
    pub expected_outcome: String,
}

/// JSON schema for sub-tasks that need further decomposition
#[derive(Debug, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: u8,
    pub dependencies: Vec<String>,
    pub estimated_complexity: u8, // 1-10 scale
}

/// JSON schema for plan action decomposition responses
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanActionDecompositionResponse {
    pub analysis: String,
    pub is_executable: bool,
    pub executable_steps: Option<Vec<ExecutableStep>>,
    pub sub_actions: Option<Vec<SubAction>>,
    pub context_needed: Vec<String>,
}

/// JSON schema for executable steps within a plan action
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutableStep {
    pub step_id: String,
    pub tool: String,
    pub operation: String,
    pub target: String,
    pub parameters: Option<serde_json::Value>,
    pub validation: String,
    pub rollback_action: Option<String>,
}

/// JSON schema for sub-actions that need further breakdown
#[derive(Debug, Serialize, Deserialize)]
pub struct SubAction {
    pub id: String,
    pub title: String,
    pub description: String,
    pub context: String,
    pub tool_category: String,
    pub complexity: u8,
}

/// JSON schema for nested plan generation responses
#[derive(Debug, Serialize, Deserialize)]
pub struct NestedPlanResponse {
    pub plan_title: String,
    pub overview: String,
    pub is_simple_task: bool,
    pub direct_execution: Option<ExecutableAction>,
    pub detailed_plan: Option<DetailedPlan>,
}

/// JSON schema for detailed nested plans
#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedPlan {
    pub phases: Vec<PlanPhase>,
    pub dependencies: Vec<PhaseDependency>,
    pub estimated_duration: String,
    pub risk_factors: Vec<String>,
}

/// JSON schema for plan phases
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanPhase {
    pub phase_id: String,
    pub name: String,
    pub description: String,
    pub actions: Vec<PhaseAction>,
    pub success_criteria: Vec<String>,
}

/// JSON schema for actions within a phase
#[derive(Debug, Serialize, Deserialize)]
pub struct PhaseAction {
    pub action_id: String,
    pub title: String,
    pub tool: String,
    pub target: String,
    pub operation: String,
    pub parameters: Option<serde_json::Value>,
    pub validation: String,
    pub dependencies: Vec<String>,
}

/// JSON schema for phase dependencies
#[derive(Debug, Serialize, Deserialize)]
pub struct PhaseDependency {
    pub phase_id: String,
    pub depends_on: Vec<String>,
    pub dependency_type: String, // "sequential", "parallel", "conditional"
}

/// Helper function to create prompts that request JSON responses
pub fn create_json_prompt(instruction: &str, schema_example: &str) -> String {
    format!(
        "{}\n\n\
        IMPORTANT: Respond with valid JSON only, following this exact schema:\n\
        {}\n\n\
        Do not include any text before or after the JSON. \
        Do not wrap the JSON in code blocks or markdown. \
        Ensure all strings are properly quoted and escaped.",
        instruction,
        schema_example
    )
}

/// Schema examples for prompt generation
pub mod schema_examples {
    pub const TASK_DECOMPOSITION: &str = r#"{
  "analysis": "Brief analysis of the task complexity and requirements",
  "is_executable": false,
  "executable_action": null,
  "sub_tasks": [
    {
      "id": "task_1",
      "title": "First sub-task",
      "description": "Detailed description of what needs to be done",
      "priority": 5,
      "dependencies": [],
      "estimated_complexity": 3
    }
  ],
  "reasoning": "Explanation of why this decomposition approach was chosen"
}"#;

    pub const PLAN_ACTION_DECOMPOSITION: &str = r#"{
  "analysis": "Analysis of the action and its requirements",
  "is_executable": true,
  "executable_steps": [
    {
      "step_id": "step_1",
      "tool": "bash",
      "operation": "run_command",
      "target": "ls -la",
      "parameters": {"working_dir": "./"},
      "validation": "Command should list directory contents",
      "rollback_action": null
    }
  ],
  "sub_actions": null,
  "context_needed": ["current_directory", "permissions"]
}"#;

    pub const NESTED_PLAN: &str = r#"{
  "plan_title": "Title of the nested plan",
  "overview": "Brief overview of what this plan will accomplish",
  "is_simple_task": false,
  "direct_execution": null,
  "detailed_plan": {
    "phases": [
      {
        "phase_id": "phase_1",
        "name": "Preparation",
        "description": "Set up requirements",
        "actions": [
          {
            "action_id": "action_1",
            "title": "Check prerequisites",
            "tool": "bash",
            "target": "system",
            "operation": "validate_environment",
            "parameters": null,
            "validation": "All prerequisites are met",
            "dependencies": []
          }
        ],
        "success_criteria": ["Environment is ready"]
      }
    ],
    "dependencies": [],
    "estimated_duration": "5-10 minutes",
    "risk_factors": ["Missing dependencies"]
  }
}"#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_decomposition_deserialization() {
        let json = r#"{
            "analysis": "Test analysis",
            "is_executable": false,
            "executable_action": null,
            "sub_tasks": [
                {
                    "id": "task_1",
                    "title": "Test task",
                    "description": "Test description",
                    "priority": 5,
                    "dependencies": [],
                    "estimated_complexity": 3
                }
            ],
            "reasoning": "Test reasoning"
        }"#;

        let result: Result<TaskDecompositionResponse, _> = serde_json::from_str(json);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.analysis, "Test analysis");
        assert!(!response.is_executable);
        assert!(response.sub_tasks.is_some());
        assert_eq!(response.sub_tasks.unwrap().len(), 1);
    }

    #[test]
    fn test_executable_action_deserialization() {
        let json = r#"{
            "analysis": "Simple executable task",
            "is_executable": true,
            "executable_action": {
                "tool": "bash",
                "operation": "run_command",
                "target": "echo 'hello'",
                "parameters": null,
                "expected_outcome": "Output 'hello'"
            },
            "sub_tasks": null,
            "reasoning": "Task is simple enough to execute directly"
        }"#;

        let result: Result<TaskDecompositionResponse, _> = serde_json::from_str(json);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response.is_executable);
        assert!(response.executable_action.is_some());
    }
}