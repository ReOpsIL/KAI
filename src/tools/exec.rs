use crate::tools::file_system::{FileSystemTool, ToolFunction, ToolParameters, ToolResult};
use serde_json::json;
use std::process::Command;

pub fn run_shell_command_tool(command: &str) -> ToolResult {
    let output = Command::new("sh").arg("-c").arg(command).output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            ToolResult {
                success: output.status.success(),
                data: Some(json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": output.status.code(),
                })),
                error: if output.status.success() {
                    None
                } else {
                    Some(stderr)
                },
            }
        }
        Err(e) => ToolResult {
            success: false,
            data: None,
            error: Some(format!("Failed to execute command '{}': {}", command, e)),
        },
    }
}

pub fn get_shell_tool() -> FileSystemTool {
    FileSystemTool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: "run_shell".to_string(),
            description: "Execute a shell command.".to_string(),
            parameters: ToolParameters {
                param_type: "object".to_string(),
                properties: json!({
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute."
                    }
                }),
                required: vec!["command".to_string()],
            },
        },
    }
}
