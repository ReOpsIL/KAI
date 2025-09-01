pub mod exec;
pub mod file_system;

use file_system::FileSystemTool;

/// Get all available tools for the LLM
pub fn get_all_tools() -> Vec<FileSystemTool> {
    let mut tools = file_system::get_file_system_tools();
    tools.push(exec::get_shell_tool());
    tools
}
