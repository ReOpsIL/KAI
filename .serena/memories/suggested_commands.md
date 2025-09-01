# Suggested Commands for KAI Development

## Build and Run Commands
```bash
# Development build and check
cargo check
cargo build

# Release build  
cargo build --release

# Run the application (requires OPENROUTER_API_KEY)
export OPENROUTER_API_KEY=your_key_here
cargo run

# Run specific examples
cargo run --example context_test
cargo run --example harvester_demo
```

## Testing Commands
```bash
# Run all tests
cargo test

# Run library tests only (skips examples)
cargo test --lib

# Run specific module tests
cargo test history
cargo test planer
cargo test cli
```

## Environment Setup
```bash
# Required: Set OpenRouter API key
export OPENROUTER_API_KEY=your_api_key_here

# Verify setup - key should start with appropriate prefix
echo $OPENROUTER_API_KEY
```

## Darwin-specific System Commands
```bash
# File operations
ls -la
find . -name "*.rs"
grep -r "pattern" src/

# Process management  
ps aux | grep kai
lsof -p <pid>

# Git operations
git status
git log --oneline
git diff
```

## Development Workflow
1. `cargo check` - Quick syntax and type check
2. `cargo test` - Run test suite  
3. `cargo build` - Full build
4. `export OPENROUTER_API_KEY=...` - Set API key
5. `cargo run` - Test the application