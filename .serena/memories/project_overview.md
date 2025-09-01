# KAI Project Overview

KAI is a sophisticated AI-powered terminal CLI application that combines advanced interactive prompting with intelligent task planning and execution. The system integrates OpenRouter's LLM capabilities with a comprehensive file system toolset to provide an enhanced development assistant experience.

## Tech Stack
- **Language**: Rust (edition 2021)
- **Terminal UI**: crossterm for terminal control
- **HTTP Client**: reqwest with JSON support  
- **Async Runtime**: Tokio with full features
- **CLI Interactions**: inquire for menus and prompts
- **Serialization**: serde and serde_json
- **File Operations**: walkdir, glob
- **Other**: regex, rand, chrono

## Key Features
- AI-powered task planning using OpenRouter LLMs
- Interactive terminal interface with advanced text editing
- File system integration and browsing
- Command history with smart navigation
- Multiple color themes
- Context-aware conversations
- No fallback modes - requires working AI integration

## Architecture
- Entry Point: `src/main.rs` - requires OpenRouter API key
- AI Planning: `src/planer/` - orchestrates LLM-powered plan generation  
- CLI Interface: `src/cli/` - modern terminal interface
- File Tools: `src/tools/` - comprehensive toolkit for LLM integration
- Context Management: `src/context/` - intelligent contextual awareness
- LLM Integration: `src/llm/openrouter.rs` - complete OpenRouter API client