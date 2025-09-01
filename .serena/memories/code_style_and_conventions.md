# Code Style and Conventions for KAI

## General Rust Conventions
- Follow standard Rust naming conventions (snake_case for functions/variables, PascalCase for types)
- Use `#[derive(Debug, Clone)]` for appropriate structs and enums
- Comprehensive `Result<T, E>` error handling throughout
- No mock, dummy, stub, or fallback implementations (per CLAUDE.md)
- Every function must have full business logic and error handling

## Module Organization
- `src/main.rs` - Entry point with dependency injection
- `src/cli/` - All CLI-related functionality
- `src/planer/` - AI planning system components
- `src/tools/` - File system and utility tools
- `src/context/` - Context management and persistence
- `src/llm/` - LLM integration (OpenRouter)

## Error Handling Patterns
- Use `io::Result<()>` for I/O operations
- User-friendly error messages with clear setup instructions
- Graceful exit patterns rather than silent failures
- No fallback modes - application exits if AI is unavailable

## Async Patterns
- Tokio-based runtime for non-blocking operations
- `async fn` for LLM API calls and I/O operations
- Proper `await` handling in loops and error cases

## Documentation
- Use `///` for public API documentation
- Use `//!` for module-level documentation
- Clear, concise comments explaining business logic
- No TODO comments (implement fully instead)

## Terminal UI Patterns
- Use crossterm for cross-platform terminal control
- ANSI escape sequences for cursor movement and formatting
- Proper terminal state management (raw mode on/off)
- Unicode emoji prefixes for message types

## Dependencies and Integration
- No circular dependencies
- Clean separation of concerns between modules
- Dependency injection pattern for component integration
- Well-defined interfaces between major systems