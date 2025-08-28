# KAI - Usage Guide

## Installation and Running

### Development Mode

1. **Clone and Build**
   ```bash
   git clone <repository-url>
   cd KAI
   cargo build --release
   ```

2. **Run KAI CLI Prompter**
   ```bash
   # Development build
   cargo run --bin kai
   
   # Or run the binary directly
   ./target/debug/kai
   ```

3. **Install Locally**
   ```bash
   cargo install --path .
   kai  # Now available in your PATH
   ```

### Usage Examples

#### Main CLI Application
```bash
# Start the enhanced CLI prompter
kai

# The application will show:
# ╭─────────────────────────────────────────────────╮
# │  🤖 KAI - Enhanced CLI Prompter               │
# │  Advanced terminal interface for AI prompting  │
# ╰─────────────────────────────────────────────────╯
```

#### Example Demos
```bash
# File system tools demo
cargo run --example file_system_tools_demo

# Session manager demo
cargo run --example session_manager_demo

# Harvester demo
cargo run --example harvester_demo

# CLI prompter demo
cargo run --example cli_prompter_demo
```

## CLI Prompter Features

### Quick Start
1. **Launch KAI**: `cargo run --bin kai`
2. **Command Menu**: Type `/` to access commands
3. **File Browser**: Type `@` to browse files
4. **Help**: Type `/help` for detailed help
5. **Exit**: Press `Ctrl+C` or use `/quit`

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `/` | Open command menu |
| `@` | Open file browser |
| `Ctrl+C` | Exit application |
| `Ctrl+R` | Search command history |
| `Ctrl+A` | Move to beginning of line |
| `Ctrl+E` | Move to end of line |
| `Ctrl+D` | Delete character forward |
| `Ctrl+H` | Delete character backward |
| `Ctrl+K` | Delete to end of line |
| `Ctrl+U` | Delete entire line |
| `Ctrl+W` | Delete word backward |
| `Ctrl+L` | Clear screen |
| `↑↓←→` | Navigate cursor |
| `Home/End` | Jump to line boundaries |
| `Enter` | New line or submit |
| `Esc` | Clear current input |

### Available Commands

| Command | Description |
|---------|-------------|
| `/help` | Show help and keyboard shortcuts |
| `/history` | View command history |
| `/clear` | Clear terminal screen |
| `/config` | View current configuration |
| `/theme` | Change color theme |
| `/templates` | Load prompt templates |
| `/export` | Export current session |
| `/save` | Save current session |
| `/load` | Load saved session |
| `/quit` | Exit application |

### Themes

Available color themes:
- **default**: Blue frame, white text
- **dark**: Black frame, green text  
- **light**: Gray frame, black text
- **ocean**: Cyan frame, white text
- **sunset**: Magenta frame, yellow text

Access themes with `/theme` command.

### File Browser

1. Type `@` to open file browser
2. Navigate with arrow keys
3. Enter directories or select files
4. `..` navigates to parent directory
5. File paths are automatically inserted

Visual indicators:
- 📁 Directories
- 📄 Files
- File sizes displayed

## Troubleshooting

### Common Issues

**Terminal Not Supported**
- Try a different terminal (iTerm2, Terminal.app, etc.)
- Ensure terminal supports ANSI colors

**Permission Issues**
- Check terminal permissions
- Try running with appropriate privileges

**Terminal Too Small**
- Resize terminal window
- Minimum recommended size: 80x24

**Build Issues**
```bash
# Clean and rebuild
cargo clean
cargo build --release

# Check dependencies
cargo check
```

### System Requirements

- **Rust**: 1.70+ (2021 edition)
- **Terminal**: ANSI color support
- **Platform**: Windows, macOS, Linux
- **Memory**: 16MB+ available RAM

### Dependencies

Core dependencies automatically handled by Cargo:
- `tokio`: Async runtime
- `ratatui`: Terminal UI framework
- `crossterm`: Cross-platform terminal control
- `inquire`: Interactive prompts
- `serde`: Configuration serialization

## API Integration

### Using as Library

```rust
use kai::cli::CliPrompter;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut prompter = CliPrompter::new()?;
    prompter.run().await
}
```

### Configuration

```rust
use kai::cli::CliConfig;

let config = CliConfig {
    frame_color: "Blue".to_string(),
    text_color: "White".to_string(),
    command_prefix: '/',
    file_browser_prefix: '@',
    auto_save_history: true,
    max_history_size: 1000,
    theme_name: "default".to_string(),
    // ...
};
```

## Advanced Usage

### Custom Integration

The KAI CLI can be integrated into larger applications:

```rust
// Initialize with custom config
let prompter = CliPrompter::with_config(custom_config)?;

// Run with custom handlers
prompter.run_with_handlers(custom_handlers).await?;
```

### Plugin Development

Future plugin system will support:
- Custom commands
- External integrations  
- Theme extensions
- File format handlers

## Support

For issues, feature requests, or contributions:
1. Check existing documentation
2. Review troubleshooting section
3. File issues with detailed reproduction steps
4. Include system information and terminal type