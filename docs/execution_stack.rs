use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use crate::plan_display::{ActionPlan, Action, ActionStatus};

/// Represents different types of execution requests that can be stacked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StackRequest {
    /// User prompt to be processed
    UserPrompt {
        id: String,
        content: String,
        priority: u8,
    },
    /// Plan action to be executed
    PlanAction {
        id: String,
        plan_id: String,
        action: Action,
        context: String, // Additional context from the plan
    },
    /// Nested plan generation request
    NestedPlan {
        id: String,
        parent_id: String,
        request: String,
        depth: u8,
    },
}

/// Response from processing a stack request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackResponse {
    pub request_id: String,
    pub success: bool,
    pub content: String,
    pub generated_requests: Vec<StackRequest>, // New requests generated from this response
    pub completed_actions: Vec<String>, // IDs of actions that were completed
}

/// Execution context for stack processing
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub current_depth: u8,
    pub max_depth: u8,
    pub active_plan_ids: Vec<String>,
    pub completed_action_ids: Vec<String>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            current_depth: 0,
            max_depth: 5, // Prevent infinite recursion
            active_plan_ids: Vec::new(),
            completed_action_ids: Vec::new(),
        }
    }
}

/// Main execution stack for managing prompts and responses
#[derive(Debug)]
pub struct ExecutionStack {
    /// Queue of requests to be processed (FIFO for normal requests)
    request_queue: VecDeque<StackRequest>,
    /// Stack for high-priority requests (LIFO for urgent tasks)
    priority_stack: Vec<StackRequest>,
    /// History of processed requests and responses
    history: Vec<(StackRequest, StackResponse)>,
    /// Current execution context
    context: ExecutionContext,
    /// Next available ID for requests
    next_id: u64,
}

impl Default for ExecutionStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionStack {
    pub fn new() -> Self {
        Self {
            request_queue: VecDeque::new(),
            priority_stack: Vec::new(),
            history: Vec::new(),
            context: ExecutionContext::default(),
            next_id: 1,
        }
    }

    /// Generate a unique ID for a request
    pub fn generate_id(&mut self) -> String {
        let id = format!("req_{}", self.next_id);
        self.next_id += 1;
        id
    }

    /// Push a user prompt to the stack
    pub fn push_user_prompt(&mut self, content: String, priority: u8) -> String {
        let id = self.generate_id();
        let request = StackRequest::UserPrompt {
            id: id.clone(),
            content,
            priority,
        };

        if priority >= 5 {
            self.priority_stack.push(request);
        } else {
            self.request_queue.push_back(request);
        }

        println!("📥 --->>> Pushed user prompt to stack: {}", id);
        id
    }

    /// Push a plan action to the stack
    pub fn push_plan_action_item(&mut self, plan_id: String, action: Action, context: String) -> String {
        let id = self.generate_id();
        let request = StackRequest::PlanAction {
            id: id.clone(),
            plan_id,
            action,
            context,
        };

        self.request_queue.push_back(request);
        println!("📥 --->>> Pushed plan action to stack: {}", id);
        id
    }

    /// Push a nested plan request to the stack
    pub fn push_nested_plan_item(&mut self, parent_id: String, request: String, depth: u8) -> String {
        if depth > self.context.max_depth {
            println!("⚠️ Maximum depth reached, skipping nested plan: {}", request);
            return String::new();
        }

        let id = self.generate_id();
        let nested_request = StackRequest::NestedPlan {
            id: id.clone(),
            parent_id,
            request,
            depth,
        };

        // Nested plans get priority to maintain execution flow
        self.priority_stack.push(nested_request);
        println!("📥 Pushed nested plan to stack (depth {}): {}", depth, id);
        id
    }

    /// Pop the next request to process (priority stack first, then queue)
    pub fn pop_request(&mut self) -> Option<StackRequest> {
        // Check priority stack first
        if let Some(request) = self.priority_stack.pop() {
            return Some(request);
        }

        // Then check regular queue
        self.request_queue.pop_front()
    }

    /// Push a response and process any generated requests
    pub fn push_response(&mut self, response: StackResponse) {
        println!("📤 Processing response for request: {}", response.request_id);

        // Add generated requests to the stack
        for generated_request in &response.generated_requests {
            match generated_request {
                StackRequest::UserPrompt { priority, .. } => {
                    if *priority >= 5 {
                        self.priority_stack.push(generated_request.clone());
                    } else {
                        self.request_queue.push_back(generated_request.clone());
                    }
                }
                StackRequest::PlanAction { .. } => {
                    self.request_queue.push_back(generated_request.clone());
                }
                StackRequest::NestedPlan { depth, .. } => {
                    if *depth <= self.context.max_depth {
                        self.priority_stack.push(generated_request.clone());
                    }
                }
            }
        }

        // Update completed actions
        for completed_id in &response.completed_actions {
            if !self.context.completed_action_ids.contains(completed_id) {
                self.context.completed_action_ids.push(completed_id.clone());
                println!("✅ Marked action as completed: {}", completed_id);
            }
        }

        // Add to history - find the index first, then update
        let response_id = &response.request_id;
        let mut found_index = None;
        for (index, (req, _)) in self.history.iter().enumerate() {
            if self.get_request_id(req) == response_id {
                found_index = Some(index);
                break;
            }
        }
        
        if let Some(index) = found_index {
            self.history[index].1 = response;
        } else {
            // This shouldn't happen, but handle it gracefully
            println!("⚠️ No matching request found in history for response: {}", response_id);
        }
    }

    /// Convert an action plan into stack requests
    pub fn push_action_plan(&mut self, plan: ActionPlan, parent_context: Option<String>) -> Vec<String> {
        let plan_id = self.generate_id();
        let mut request_ids = Vec::new();
        
        println!("📋 Converting action plan to stack requests: {}", plan.title);

        let context = parent_context.unwrap_or_else(|| {
            format!("Plan: {}\nOverview: {}", plan.title, plan.overview)
        });

        // Push all actions from all phases
        for phase in &plan.phases {
            for action in &phase.actions {
                // Only push actions that are pending or not yet started
                if matches!(action.status, ActionStatus::Pending) {
                    let action_context = format!("{}\nPhase: {} {}\nAction: {}", 
                        context, phase.emoji, phase.name, action.title);
                    
                    let request_id = self.push_plan_action_item(
                        plan_id.clone(),
                        action.clone(),
                        action_context
                    );
                    request_ids.push(request_id);
                }
            }
        }

        self.context.active_plan_ids.push(plan_id);
        println!("📊 Added {} actions to execution stack", request_ids.len());
        request_ids
    }

    /// Check if the stack has any pending requests
    pub fn has_pending_requests(&self) -> bool {
        !self.request_queue.is_empty() || !self.priority_stack.is_empty()
    }

    /// Get the number of pending requests
    pub fn pending_count(&self) -> usize {
        self.request_queue.len() + self.priority_stack.len()
    }

    /// Get current execution context
    pub fn get_context(&self) -> &ExecutionContext {
        &self.context
    }

    /// Clear all pending requests (for emergency stops)
    pub fn clear_all(&mut self) {
        self.request_queue.clear();
        self.priority_stack.clear();
        self.context = ExecutionContext::default();
        println!("🧹 Cleared all pending requests from stack");
    }

    /// Get status summary
    pub fn get_status_summary(&self) -> String {
        format!(
            "📊 Execution Stack Status:\n\
            • Pending requests: {} (Queue: {}, Priority: {})\n\
            • Active plans: {}\n\
            • Completed actions: {}\n\
            • Current depth: {}/{}\n\
            • History entries: {}",
            self.pending_count(),
            self.request_queue.len(),
            self.priority_stack.len(),
            self.context.active_plan_ids.len(),
            self.context.completed_action_ids.len(),
            self.context.current_depth,
            self.context.max_depth,
            self.history.len()
        )
    }

    /// Helper to get request ID from any StackRequest
    fn get_request_id<'a>(&self, request: &'a StackRequest) -> &'a String {
        match request {
            StackRequest::UserPrompt { id, .. } => id,
            StackRequest::PlanAction { id, .. } => id,
            StackRequest::NestedPlan { id, .. } => id,
        }
    }

    /// Add request to history when it starts processing
    pub fn start_processing(&mut self, request: StackRequest) {
        let placeholder_response = StackResponse {
            request_id: self.get_request_id(&request).clone(),
            success: false,
            content: "Processing...".to_string(),
            generated_requests: Vec::new(),
            completed_actions: Vec::new(),
        };
        
        self.history.push((request, placeholder_response));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_display::Phase;

    #[test]
    fn test_stack_basic_operations() {
        let mut stack = ExecutionStack::new();
        
        // Test pushing user prompt
        let id1 = stack.push_user_prompt("Create a web app".to_string(), 3);
        assert_eq!(stack.pending_count(), 1);
        
        // Test pushing high priority prompt
        let id2 = stack.push_user_prompt("Emergency fix".to_string(), 8);
        assert_eq!(stack.pending_count(), 2);
        
        // High priority should be popped first
        let next = stack.pop_request().unwrap();
        if let StackRequest::UserPrompt { id, content, priority } = next {
            assert_eq!(id, id2);
            assert_eq!(content, "Emergency fix");
            assert_eq!(priority, 8);
        } else {
            panic!("Expected UserPrompt");
        }
        
        // Then normal priority
        let next = stack.pop_request().unwrap();
        if let StackRequest::UserPrompt { id, content, priority } = next {
            assert_eq!(id, id1);
            assert_eq!(content, "Create a web app");
            assert_eq!(priority, 3);
        } else {
            panic!("Expected UserPrompt");
        }
        
        assert_eq!(stack.pending_count(), 0);
    }

    #[test]
    fn test_nested_plan_depth_limit() {
        let mut stack = ExecutionStack::new();
        
        // Should allow nested plan within depth limit
        let id = stack.push_nested_plan_item("parent_1".to_string(), "subtask".to_string(), 3);
        assert!(!id.is_empty());
        assert_eq!(stack.pending_count(), 1);
        
        // Should reject nested plan exceeding depth limit
        let id = stack.push_nested_plan_item("parent_2".to_string(), "deep_task".to_string(), 10);
        assert!(id.is_empty());
        assert_eq!(stack.pending_count(), 1);
    }
}