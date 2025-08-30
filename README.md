# KAI - Enhanced CLI Prompter

🤖 **KAI** is a sophisticated terminal-based CLI application that provides an advanced interactive prompt interface with intelligent text editing, command history, file browsing, and modern keyboard shortcuts.

## ✨ Features

### 🎯 **Advanced Text Editing**
- Multi-line text input with cursor management
- Word-based operations (Ctrl+W for word deletion)
- Smart text wrapping and display
- Real-time cursor positioning

### 📜 **Intelligent Command History**
- Persistent command history with efficient storage
- **Smart navigation**: Type text → Press Up to browse history → Press Down to restore original text
- History search with filtering capabilities
- Automatic duplicate prevention

### 🎨 **Modern Interface**
- Dynamic frame rendering positioned at screen bottom
- Multiple color themes (default, dark, light, solarized, dracula)
- Clean, responsive terminal UI
- Progress indicators and visual feedback

### ⌨️ **Enhanced Keyboard Shortcuts**

#### **Navigation & History**
- `↑` / `↓` - Navigate command history with current line preservation
- `←` / `→` - Move cursor within text
- `Home` / `End` - Jump to start/end of line
- `Ctrl+R` - Search command history

#### **Text Operations**
- `Ctrl+A` / `Ctrl+E` - Move to start/end of line
- `Ctrl+K` - Delete to end of line
- `Ctrl+U` - Delete entire line  
- `Ctrl+W` - Delete word backward
- `Ctrl+D` / `Delete` - Delete character forward
- `Ctrl+H` / `Backspace` - Delete character backward

#### **Copy & Paste**
- `Ctrl+C` (1x) - Copy current text to clipboard
- `Ctrl+V` - Paste from clipboard with multi-line support

#### **System Controls**
- `Escape` (3x) - Clear prompt frame completely
- `Ctrl+C` (3x) - Exit application
- `Ctrl+L` - Clear screen

### 🚀 **Interactive Menus**
- **Command Menu**: Type `/` for available commands
- **File Browser**: Type `@` for interactive file system navigation
- **Theme Selector**: Switch between color themes instantly
- **Configuration**: View and modify settings

### 📁 **File System Integration**
- Interactive file browser with metadata display
- File type detection and formatting
- Directory navigation with history
- File size formatting (B, KB, MB, GB, TB)

### 🤖 **AI Planning System**
- **Advanced Task Planning**: Uses OpenRouter LLMs to create detailed action plans
- **Structured Prompts**: Employs sophisticated prompt engineering for reliable results
- **JSON Schema Parsing**: Converts AI responses to executable task structures  
- **Phase-Based Organization**: Analysis → Implementation → Verification workflow
- **Dependency Management**: Handles task dependencies and execution order
- **Fallback Support**: Graceful degradation when AI is unavailable

## 🛠️ Installation

### Prerequisites
- Rust 1.70+ 
- A terminal that supports ANSI colors and UTF-8
- OpenRouter API key (required for AI planning features)

### OpenRouter Setup
1. **Get API Key**: Visit [OpenRouter.ai](https://openrouter.ai) and create an account
2. **Get API Key**: Generate an API key from your OpenRouter dashboard
3. **Set Environment Variable**:
   ```bash
   export OPENROUTER_API_KEY=your_api_key_here
   ```
4. **Verify Setup**: The key should start with `sk-or-` for OpenRouter

### Build from Source
```bash
git clone <repository-url>
cd KAI
cargo build --release
```

### Run
```bash
# Make sure your API key is set
export OPENROUTER_API_KEY=your_api_key_here

# Run the application
cargo run
# or
./target/release/kai
```

### First Run
When you start KAI for the first time with a valid API key, you'll see:
```
✅ OpenRouter client initialized successfully
✅ CLI prompter initialized successfully  
🧠 AI Planning system initialized with OpenRouter
```

If the API key is missing or invalid, KAI will show setup instructions and exit.

## 🎮 Usage

### Getting Started
1. **Launch KAI**: Run `cargo run` or `./kai`
2. **Start typing**: Enter your prompts in the interactive frame
3. **Use shortcuts**: Access commands with `/` or files with `@`
4. **Navigate history**: Use arrow keys to browse previous commands

### Example Workflow
```bash
# Type some text
Hello, this is my prompt

# Press ↑ to save current text and browse history
# Navigate through previous commands
# Press ↓ to restore "Hello, this is my prompt"

# Use special commands
/help          # Show help menu
@              # Open file browser
/theme         # Change color theme
/history       # View command history
```

### Keyboard Shortcuts Quick Reference

| Shortcut | Action |
|----------|--------|
| `↑` / `↓` | Navigate history (preserves current text) |
| `←` / `→` | Move cursor |
| `/` | Command menu |
| `@` | File browser |
| `Ctrl+C` (1x) | Copy text |
| `Ctrl+V` | Paste text |
| `Ctrl+C` (3x) | Exit |
| `Escape` (3x) | Clear prompt |
| `Ctrl+R` | Search history |
| `Enter` | Submit prompt |

## 🏗️ Architecture

KAI features a modular architecture with clean separation of concerns:

### Core Modules
```
src/cli/
├── prompter.rs      # Main orchestration and UI
├── config.rs        # Configuration and themes
├── editor.rs        # Text editing engine
├── history.rs       # Command history management
├── commands.rs      # Command system
├── file_browser.rs  # File system navigation
└── utils.rs         # Common utilities
```

### Key Design Principles
- **Modular**: Each component has a single responsibility
- **Testable**: Comprehensive unit test coverage
- **Extensible**: Easy to add new features and commands
- **Performant**: Efficient data structures and algorithms

## 🧪 Testing

Run the test suite:
```bash
# All tests
cargo test

# Specific module tests
cargo test history
cargo test editor
cargo test config
```

## 🎨 Themes

KAI supports multiple built-in themes:
- **Default** - Clean blue/white theme
- **Dark** - High contrast dark theme
- **Light** - Minimal light theme  
- **Solarized** - Popular solarized color scheme
- **Dracula** - Modern purple/pink theme

Switch themes with `/theme` command or modify configuration.

## ⚙️ Configuration

Customize KAI through the configuration system:
- Color themes and styling
- History size limits
- Keyboard shortcuts
- File browser settings

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines
- Follow existing code style and patterns
- Add tests for new functionality
- Update documentation as needed
- Ensure all tests pass before submitting

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- Terminal UI powered by [ratatui](https://github.com/ratatui-org/ratatui)
- Interactive prompts using [inquire](https://github.com/mikaelmello/inquire)
- Cross-platform terminal handling with [crossterm](https://github.com/crossterm-rs/crossterm)

---

**KAI** - Where intelligent prompting meets modern terminal experience! 🚀