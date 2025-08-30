# CLAUDE.md

## Core Development Principles

### 1. NO MOCK/STUB FUNCTIONALITY
- **NEVER** generate placeholder, mock, or stub code
- Every function, class, or module must be fully implemented
- If implementation requires external dependencies or complex logic, provide the complete working solution
- Replace TODO comments with actual implementation
- Avoid `throw new Error("Not implemented")` or similar patterns

### 2. NO ORPHAN CODE
- Every piece of code must have a clear purpose and integration path
- All functions must be called or exported for use
- All classes must be instantiated and utilized
- All modules must be imported and integrated into the larger system
- Remove unused imports, variables, and functions
- Ensure every component has clear entry and exit points

### 3. ARCHITECTURAL INTEGRITY
- **THINK BEFORE CODING**: Always understand the full system architecture before implementing
- Map out relationships between all components before writing code
- Consider data flow, dependency graphs, and integration points
- Avoid creating overly complex mechanisms without full integration understanding
- Design with the complete system perspective in mind
- Document component relationships and interactions

## Implementation Strategy

### 4. INCREMENTAL DEVELOPMENT
- **GO SLOW**: Implement one small, complete feature at a time
- Start with the simplest possible working version
- Add complexity gradually and deliberately
- Each step should be fully functional before moving to the next
- Avoid jumping multiple abstraction levels in a single implementation

### 5. STEP-BY-STEP PROGRESSION
```
Phase 1: Analysis & Planning
Phase 2: Core Foundation (minimal viable implementation)
Phase 3: Incremental Feature Addition
Phase 4: Integration & Testing
Phase 5: Optimization & Refinement
```

### 6. DETAILED PLANNING REQUIREMENT
Before ANY implementation, provide:

#### A. System Architecture Overview
- High-level component diagram
- Data flow patterns
- Key interfaces and contracts
- Integration points with existing systems

#### B. Implementation Plan
- Break down into specific, measurable steps
- Define clear success criteria for each step
- Identify dependencies and prerequisites
- Estimate complexity and potential challenges

#### C. Module Relationship Map
- How each new component connects to existing code
- What interfaces it exposes and consumes
- How it fits into the overall application lifecycle
- What it depends on and what depends on it

## Code Quality Standards

### 7. COMPLETE IMPLEMENTATIONS
- Every function must have full business logic
- Error handling must be comprehensive
- Edge cases must be addressed
- Input validation must be thorough
- Return values must be meaningful and complete

### 8. INTEGRATION REQUIREMENTS
- New code must integrate seamlessly with existing codebase
- Follow established patterns and conventions
- Maintain consistency in error handling, logging, and configuration
- Respect existing architectural decisions
- Update documentation and tests accordingly

## Before Starting Any Task

### Required Analysis Steps:
1. **Understand the Request**: What is the actual business requirement?
2. **Assess Current State**: What exists already? What can be reused?
3. **Design the Solution**: How will this fit into the existing architecture?
4. **Plan the Steps**: What is the minimal viable first step?
5. **Identify Risks**: What could go wrong? What are the dependencies?

### Required Deliverables Before Coding:
1. **Architecture Diagram**: Visual representation of components and relationships
2. **Implementation Roadmap**: Ordered list of development phases
3. **Integration Points**: How new code connects to existing systems
4. **Success Metrics**: How to validate each phase works correctly
5. **Rollback Plan**: How to safely undo changes if issues arise

## Development Process

### Phase 1: Analysis (Required)
- Map existing codebase structure
- Identify integration points
- Document current patterns and conventions
- Understand data models and business logic

### Phase 2: Foundation (Minimal Implementation)
- Create the simplest possible working version
- Focus on core functionality only
- Ensure full integration with existing systems
- No shortcuts or temporary solutions

### Phase 3: Incremental Enhancement
- Add one feature at a time
- Test integration after each addition
- Maintain backward compatibility
- Keep complexity manageable

### Phase 4: Validation & Refinement
- Verify all integration points work correctly
- Test edge cases and error conditions
- Optimize performance if needed
- Update documentation

## Forbidden Practices

❌ **DO NOT**:
- Generate TODO or placeholder comments
- Create interfaces without implementations
- Write abstract classes without concrete implementations
- Use mock objects in production code
- Leave unused imports or variables
- Implement partial solutions
- Skip error handling
- Create circular dependencies
- Ignore existing conventions
- Jump multiple architectural layers at once

✅ **DO**:
- Implement complete, working solutions
- Think through full integration before coding
- Start simple and build incrementally
- Document decisions and rationale
- Follow established patterns
- Test integration continuously
- Plan before implementing
- Consider the full system impact

## Success Criteria

Every implementation must:
1. **Work Completely**: No placeholders or stubs
2. **Integrate Fully**: Connect properly to existing systems
3. **Follow Patterns**: Maintain consistency with codebase
4. **Be Testable**: Include proper error handling and validation
5. **Be Maintainable**: Clear structure and documentation
6. **Be Reversible**: Can be safely removed if needed

## Remember

> "Perfect is the enemy of good, but incomplete is the enemy of functional."

Build working solutions incrementally. Each step should be complete and functional, even if simple. Complexity comes through careful addition, not through ambitious initial design.

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

KAI is a sophisticated AI-powered terminal CLI application that combines advanced interactive prompting with intelligent task planning and execution. The system integrates OpenRouter's LLM capabilities with a comprehensive file system toolset to provide an enhanced development assistant experience.

## Development Commands

### Build and Run
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

### Testing
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

## Core Architecture

### Entry Point and Initialization (`src/main.rs`)
- **Critical Pattern**: Application REQUIRES OpenRouter API key or exits immediately - NO fallback modes
- Initializes OpenRouter client → Planner → CLI Prompter pipeline
- Uses dependency injection pattern for component integration

### AI Planning System (`src/planer/`)
The heart of KAI's intelligence:

**Key Flow**: User Input → LLM → Structured JSON Plan → Task Queue → Execution
- **`task_planner.rs`**: Orchestrates LLM-powered plan generation using structured prompts
- **`plan.rs`**: Multi-phase organization (Analysis → Implementation → Verification) with Display formatting
- **`task.rs`**: Core task structure with dependency management and status tracking
- **`queue.rs`**: Priority-based execution queue with request/response handling
- **`schemas.rs`**: JSON schema definitions for reliable LLM communication

**Important**: Default model is `openai/gpt-4o-mini` - configurable via `with_model()` method

### CLI Interface System (`src/cli/`)
Modern terminal interface with ratatui:

**Architecture**: All UI components coordinate through `prompter.rs` main orchestrator
- **`editor.rs`**: Multi-line text editor with advanced cursor management
- **`history.rs`**: Persistent command history with smart navigation 
- **`commands.rs`**: Command system (`/` commands, `@` file browser)
- **`config.rs`**: Theme and configuration management with 5 built-in themes
- **`file_browser.rs`**: Interactive file navigation with metadata display

**Key Pattern**: CLI prompter takes ownership of planner via `with_planner()` constructor

### File System Tools (`src/tools/`)
Comprehensive toolkit designed for LLM integration:
- **Purpose-built for AI**: Each tool provides structured output suitable for LLM consumption
- **Operations**: read_file, write_file, list_directory, grep_files, search_replace, create_path, delete_path
- **Wildcard Support**: Pattern matching and bulk operations

### Context Management (`src/context/`)
Intelligent contextual awareness:
- **`context.rs`**: File timestamp tracking and change detection
- **`harvesters.rs`**: Automated codebase analysis and metadata extraction
- **`story.rs`**: Conversation history with prompt-response pairing
- **Integration**: Links with session management for persistent state

### LLM Integration (`src/llm/openrouter.rs`)
Complete OpenRouter API client:
- **Features**: Tool calling support, streaming responses, error handling
- **Authentication**: Environment variable `OPENROUTER_API_KEY` (required)
- **Models**: Supports any OpenRouter model with tool capabilities

## Key Design Patterns

### No Fallback Policy
**Critical**: The application follows a strict "AI or exit" policy:
- No mock, dummy, stub, or fallback implementations
- If OpenRouter fails during planning, application exits with error
- If no API key is provided, application exits during startup

### Result-Based Error Handling
- Comprehensive `Result<T, E>` usage throughout
- User-friendly error messages with clear setup instructions
- Graceful exit patterns rather than silent failures

### Modular Architecture
- Clean separation of concerns across modules
- Dependency injection for component integration
- Well-defined interfaces between major systems

### Async-First Design
- Tokio-based runtime for non-blocking operations
- LLM API calls are async by default
- File system operations support concurrent execution

## Data Flow Architecture

```
User Input → CLI Prompter (process_input) 
    ↓
Task Planner (create_and_execute_advanced_plan)
    ↓  
OpenRouter LLM → JSON Plan Response
    ↓
Plan Conversion → Task Queue → File System Tools
    ↓
Results → Display → Context Update
```

## Configuration Requirements

### Environment Variables
- **`OPENROUTER_API_KEY`**: Required for all AI functionality (no fallback)
- Must start with appropriate prefix for OpenRouter

### Terminal Requirements  
- ANSI color support
- UTF-8 encoding
- Minimum size for proper UI rendering

### Dependencies
- Rust 1.70+
- Modern terminal emulator
- Internet connection for OpenRouter API

## Integration Points

### CLI ↔ Planner Integration
- CLI prompter processes all user input through AI planner
- Uses `take()` pattern to avoid borrowing conflicts during async operations
- Planner instance injected via constructor dependency

### Context ↔ Session Management
- Bidirectional state persistence
- File change tracking integrated with session lifecycle
- Generic key-value storage for flexible data persistence

### Tools ↔ Task Execution
- File system tools designed for structured LLM consumption
- Task executor coordinates tool invocation based on plan tasks
- Results formatted for both human and AI readability

## Development Patterns

### Adding New Commands
- Extend `CliCommand` enum in `src/cli/commands.rs`
- Implement command logic in `execute_command` method of `src/cli/prompter.rs`
- Follow existing pattern for help text and validation

### Adding New File System Tools
- Implement in `src/tools/file_system.rs`
- Follow structured return format for LLM consumption
- Add comprehensive error handling with user-friendly messages

### Extending AI Planning
- Modify schemas in `src/planer/schemas.rs` for new plan structures
- Update prompt templates in `src/prompts/prompts.rs`
- Ensure JSON schema validation for reliable LLM communication

### Theme Development
- Add theme definitions to `src/cli/config.rs`
- Follow existing color mapping patterns
- Update available themes list for user selection

## Testing Strategy

### Unit Tests
- Each module has comprehensive test coverage
- Mock dependencies for isolated testing
- Focus on edge cases and error conditions

### Integration Tests
- Test component interaction patterns
- Verify async operation handling
- Validate configuration and theme switching

### Example Applications
- `examples/` directory contains demo applications
- Use for feature validation and architectural examples
- Note: Some examples may reference missing files (build will skip them)