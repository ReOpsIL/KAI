# KAI Enhanced CLI Prompter

A sophisticated terminal-based prompt CLI application that provides an interactive text input experience with special command handling, file system navigation, and advanced text editing capabilities.

## Features

### 🎯 Core Functionality
- **Dynamic Frame Rendering**: Responsive terminal UI with auto-sizing frames
- **Multi-line Text Editor**: Full text editing capabilities with cursor management
- **Command History**: Persistent command history with search functionality
- **Real-time Input Processing**: Immediate feedback and responsive interface

### ⚡ Special Key Handlers
- **`/` Command Menu**: Access built-in commands with interactive selection
- **`@` File Browser**: Navigate and select files from the filesystem
- **Ctrl+R**: Reverse search through command history

### ⌨️ Advanced Keyboard Shortcuts
- **Ctrl+A/E**: Move to beginning/end of line
- **Ctrl+D/H**: Delete character forward/backward
- **Ctrl+K**: Delete to end of line
- **Ctrl+U**: Delete entire line
- **Ctrl+W**: Delete word backward
- **Ctrl+L**: Clear screen
- **Arrow Keys**: Navigate cursor in all directions
- **Home/End**: Jump to line boundaries

### 🎨 Customization Options
- **Theme Support**: Multiple color schemes (default, dark, light, ocean, sunset)
- **Configurable Prefixes**: Customize command and file browser trigger characters
- **History Management**: Configurable history size and auto-save options

## Usage

### Basic Example

```rust
use kai::cli::CliPrompter;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut prompter = CliPrompter::new()?;
    prompter.run().await
}
```

### Running the Demo

```bash
cargo run --example cli_prompter_demo
```

## Available Commands

| Command | Description |
|---------|-------------|
| `/help` | Show available commands and keyboard shortcuts |
| `/history` | View command history |
| `/clear` | Clear the terminal screen |
| `/config` | View current configuration |
| `/theme` | Change color theme |
| `/templates` | Load prompt templates (coming soon) |
| `/export` | Export current session (coming soon) |
| `/save` | Save current session (coming soon) |
| `/load` | Load saved session (coming soon) |
| `/quit` | Exit the application |

## File Browser

- Access with `@` key
- Navigate directories with arrow keys
- Visual indicators for files (📄) and directories (📁)
- File size information display
- Support for parent directory navigation (`..`)

## Configuration

The CLI prompter supports extensive configuration through the `CliConfig` structure:

```rust
pub struct CliConfig {
    pub frame_color: String,           // Frame border color
    pub text_color: String,            // Text content color
    pub command_prefix: char,          // Command trigger (default: '/')
    pub file_browser_prefix: char,     // File browser trigger (default: '@')
    pub auto_save_history: bool,       // Auto-save command history
    pub max_history_size: usize,       // Maximum history entries
    pub theme_name: String,            // Current theme name
    // ... custom keybindings
}
```

## Architecture

### Core Components

1. **CliPrompter**: Main application controller
2. **TextEditor**: Multi-line text editing with cursor management
3. **CommandHistory**: Command history storage and search
4. **CliConfig**: Configuration management
5. **FileEntry**: File system navigation support

### Rendering System

- Built on `ratatui` for terminal UI rendering
- `crossterm` for cross-platform terminal control
- Dynamic frame sizing based on content
- Efficient text wrapping and display

### Input System

- Asynchronous input handling with `tokio`
- Special key detection and routing
- Context-aware command processing
- Real-time cursor positioning

## Integration

The CLI prompter integrates seamlessly with the KAI ecosystem:

- Access to file system tools for file operations
- Integration with OpenRouter API client
- Session management capabilities
- Context harvesting and processing

## Performance

- **Memory Efficient**: Ring buffer for command history
- **Responsive**: Non-blocking async input handling
- **Scalable**: Handles large text inputs efficiently
- **Cross-platform**: Works on Windows, macOS, and Linux

## Dependencies

- `inquire`: Interactive prompts and menus
- `crossterm`: Cross-platform terminal manipulation
- `ratatui`: Terminal UI framework
- `tokio`: Async runtime for responsive input handling
- `serde`: Configuration serialization

## Future Enhancements

- [ ] Plugin system for custom commands
- [ ] Syntax highlighting for code inputs
- [ ] Auto-completion for file paths and commands
- [ ] Session persistence and restoration
- [ ] Template system for common prompts
- [ ] Integration with external editors
- [ ] Network-based session sharing
- [ ] Advanced search and filtering