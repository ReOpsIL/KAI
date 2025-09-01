# Task Completion Workflow for KAI

## Required Steps After Making Changes

### 1. Code Quality Checks
```bash
# Always run these in order:
cargo check          # Quick syntax/type check
cargo clippy          # Linting (if available)
cargo fmt            # Code formatting (if available)
```

### 2. Testing
```bash
# Run comprehensive tests:
cargo test --lib      # Library tests
cargo test            # All tests including examples
```

### 3. Build Verification
```bash
# Verify builds succeed:
cargo build           # Debug build
cargo build --release # Release build
```

### 4. Runtime Testing
```bash
# Set required environment variable:
export OPENROUTER_API_KEY=your_key_here

# Test basic functionality:
cargo run

# Test examples (if applicable):
cargo run --example context_test
```

### 5. Integration Verification
- Ensure AI integration works (requires valid OpenRouter key)
- Test CLI interactions (input, commands, file browser)
- Verify terminal display formatting
- Check that no broken functionality remains

## Critical Requirements
- **NO FALLBACKS**: Application must work with AI or exit cleanly
- **COMPLETE IMPLEMENTATIONS**: No placeholder or TODO code
- **ERROR HANDLING**: All error paths must be handled gracefully
- **TERMINAL COMPATIBILITY**: Must work in various terminal emulators

## Before Committing
1. All tests pass
2. Code builds without warnings
3. Application runs successfully with valid API key
4. No regression in existing functionality
5. Terminal display works correctly across different environments

## What NOT to Do
- Do not commit broken code
- Do not leave TODO comments
- Do not implement partial solutions
- Do not skip error handling
- Do not create mock/stub functionality