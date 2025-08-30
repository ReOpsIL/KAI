# Planner System Architecture Specification

## System Overview

The planner system is a hierarchical task execution engine that converts user prompts into structured action plans and manages their execution through a priority-based stack. It provides intelligent task decomposition, dependency management, and adaptive execution flow.

## Core Components

### 1. Action Structure (`Action`)

```rust
pub struct Action {
    pub id: usize,
    pub title: String,
    pub tool: String,
    pub target: String,
    pub operation: String,
    pub purpose: String,
    pub success_criteria: String,
    pub dependencies: Vec<usize>,
    pub status: ActionStatus,
}

pub enum ActionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}
```

**Purpose**: Represents atomic, executable tasks with clear success criteria and dependency tracking.

### 2. Execution Stack System

#### Stack Request Types
```rust
pub enum StackRequest {
    UserPrompt {
        id: String,
        content: String,
    },
    PlanAction {
        id: String,
        plan_id: String,
        action: Action,
        context: String,
    },
    NestedPlan {
        id: String,
        parent_id: String,
        request: String,
        depth: u8,
    },
}
```

#### Execution Stack (`ExecutionStack`)
- **Dual Queue System**: Priority stack (LIFO) + Request queue (FIFO)
- **Context Tracking**: Maintains execution depth, active plans, completed actions
- **Automatic ID Generation**: Unique request identifiers
- **Depth Limiting**: Prevents infinite recursion (max depth: 5)

**Key Methods**:
- `push_user_prompt()`: Queue user requests as first item to process  (Vec for user + VecDeque for actions)
- `push_plan_action_item()`: Queue individual actions from plans
- `push_nested_plan_item()`: Handle sub-planning requests
- `pop_request()`: Get next request (first the user if forced pushed using push_user_prompt to Vec, then FIFO from VecDeque)
- `push_action_plan()`: Convert structured plans to executable requests

### 3. Plan Generation System

#### LLM Schema Integration
The system uses structured JSON schemas for LLM communication:

```rust
// Task decomposition analysis
pub struct TaskDecompositionResponse {
    pub analysis: String,
    pub is_executable: bool,
    pub executable_action: Option<ExecutableAction>,
    pub sub_tasks: Option<Vec<SubTask>>,
    pub reasoning: String,
}

// Plan structure
pub struct DetailedPlan {
    pub phases: Vec<PlanPhase>,
    pub dependencies: Vec<PhaseDependency>,
    pub risk_factors: Vec<String>,
}
```

#### Prompt Management (`PromptManager`)
- **Unified System Prompt**: Consistent across all LLM interactions
- **Action Plan Template**: Embedded prompt for hierarchical planning
- **JSON Schema Enforcement**: Ensures structured, parseable responses

**Core Planning Prompt Structure**:
1. Analysis Phase (understanding requirements)
2. Discovery Actions (examination and investigation)
3. Implementation Actions (actual work)
4. Verification Phase (testing and validation)

### 4. Semantic Engine Integration

#### Conversation State Management
```rust
pub enum ConversationState {
    Conversational,   // General discussion
    Planning,        // Strategy and design
    Implementing,    // Active development
    Troubleshooting, // Problem solving
    Exploring,       // Code investigation
}
```

#### Intelligent Task Routing
The system analyzes user input to determine execution strategy:
- **Multi-step Planning**: Complex tasks requiring hierarchical decomposition
- **Direct Execution**: Simple tasks handled immediately
- **Stack-based Execution**: Priority management with queuing

#### Context Integration
- **Unified Context System**: Semantic message relevance and context management
- **Working Memory**: Task insights and execution context
- **File System Monitoring**: Automatic context updates from file changes

### 5. Execution Flow

#### Stack Processing Loop
1. **Request Analysis**: Determine task complexity and execution strategy
2. **Plan Generation**: Create structured action plans via LLM
3. **Stack Population**: Convert plans to executable requests
4. **Execution**: Process requests with (user priority first handling)
5. **Context Updates**: Maintain execution state and results

#### Priority Management
- Priority 7-8: Critical/urgent tasks
- Priority 5-6: Important business tasks
- Priority 3: Normal operations
- Priority stack processes high-priority items first

#### Dependency Resolution
- Action dependency tracking via ID references
- Topological sorting for execution order
- Circular dependency detection and handling

## Implementation Requirements

### 1. Core Data Structures

**Action System**:
- Implement Action struct with all fields
- ActionStatus enum with state transitions
- Dependency validation and resolution
- Success criteria evaluation

**Execution Stack**:
- Dual-queue priority system (Vec + VecDeque)
- Request ID generation and tracking
- Depth limiting with configurable max depth
- Request history and response tracking

### 2. LLM Integration

**Schema Definitions**:
- JSON response structures for all LLM interactions
- Error handling for malformed responses
- Schema validation and parsing

**Prompt System**:
- System prompt for consistent behavior
- Action plan template with phase structure
- JSON format enforcement instructions

### 3. Planning Engine

**Plan Generation**:
- Convert user prompts to structured plans
- Phase-based organization (Analysis → Implementation → Verification)
- Tool selection and operation specification
- Success criteria definition

**Task Analysis**:
- Complexity assessment (simple vs. multi-step)
- Conversation state detection
- Priority determination from input analysis

### 4. Execution Management

**Stack Processing**:
- Request popping with priority handling
- Execution loop with error recovery
- Progress reporting and status updates
- Context preservation between requests

**Tool Integration**:
- Tool call execution and result handling
- File system change detection
- Context updates from tool results

### 5. Error Handling and Recovery

**Resilience Features**:
- Malformed JSON response recovery
- Execution failure handling and retry logic
- Stack corruption prevention
- Context state validation

## Key Architectural Decisions

### 1. Hierarchical Planning
- Break complex tasks into phases and atomic actions
- Maintain clear dependency relationships
- Enable parallel execution where possible

### 2. Priority-Based Execution
- Critical tasks bypass normal queue
- Nested plans get priority to maintain flow
- User control over execution priority

### 3. Context-Aware Processing
- Semantic message relevance for context selection
- File system monitoring for automatic updates
- Execution history preservation

### 4. LLM Integration Strategy
- Structured JSON responses for reliability
- Multi-tier model selection based on task complexity
- Error recovery with fallback prompting

## Expected Behaviors

### Plan Generation
- User prompt → LLM analysis → Structured action plan
- Automatic tool selection based on task requirements
- Success criteria generation for validation

### Stack Execution
- Priority-first processing with FIFO fallback
- Automatic plan decomposition to actions
- Progress tracking and status reporting

### Context Management
- Conversation state adaptation
- Semantic context selection for relevance
- Working memory updates from execution results

### Error Recovery
- Graceful handling of LLM response errors
- Execution failure recovery with context preservation
- Stack state validation and correction

This architecture provides a robust, scalable system for intelligent task planning and execution with hierarchical decomposition, priority management, and context-aware processing.