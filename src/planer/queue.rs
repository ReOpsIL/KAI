use std::collections::VecDeque;
use crate::planer::task::{Task, TaskStatus};
use crate::planer::plan::Plan;

/// Different types of requests that can be queued
#[derive(Debug, Clone)]
pub enum QueueRequest {
    /// User prompt to be processed
    UserPrompt {
        id: String,
        content: String,
        priority: u8,
    },
    /// Task execution request
    TaskExecution {
        id: String,
        plan_id: String,
        task: Task,
    },
}

/// Response from processing a queue request
#[derive(Debug, Clone)]
pub struct QueueResponse {
    pub request_id: String,
    pub success: bool,
    pub content: String,
    pub completed_task_ids: Vec<usize>,
}

/// Simple execution queue with priority handling
#[derive(Debug)]
pub struct ExecutionQueue {
    /// High priority requests (LIFO stack)
    priority_queue: Vec<QueueRequest>,
    /// Normal priority requests (FIFO queue)
    normal_queue: VecDeque<QueueRequest>,
    /// History of processed requests
    history: Vec<(QueueRequest, QueueResponse)>,
    /// Next available request ID
    next_id: u64,
}

impl Default for ExecutionQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionQueue {
    pub fn new() -> Self {
        Self {
            priority_queue: Vec::new(),
            normal_queue: VecDeque::new(),
            history: Vec::new(),
            next_id: 1,
        }
    }

    /// Generate a unique request ID
    pub fn generate_id(&mut self) -> String {
        let id = format!("req_{}", self.next_id);
        self.next_id += 1;
        id
    }

    /// Add a user prompt to the queue
    pub fn push_user_prompt(&mut self, content: String, priority: u8) -> String {
        let id = self.generate_id();
        let request = QueueRequest::UserPrompt {
            id: id.clone(),
            content,
            priority,
        };

        if priority >= 5 {
            self.priority_queue.push(request);
        } else {
            self.normal_queue.push_back(request);
        }

        id
    }

    /// Add a task execution request to the queue
    pub fn push_task_execution(&mut self, plan_id: String, task: Task) -> String {
        let id = self.generate_id();
        let request = QueueRequest::TaskExecution {
            id: id.clone(),
            plan_id,
            task,
        };

        self.normal_queue.push_back(request);
        id
    }

    /// Add all ready tasks from a plan to the queue
    pub fn push_plan_tasks(&mut self, plan: &Plan) -> Vec<String> {
        let ready_tasks = plan.get_next_ready_tasks();
        let mut request_ids = Vec::new();

        for task in ready_tasks {
            let request_id = self.push_task_execution(
                format!("plan_{}", plan.title),
                task.clone(),
            );
            request_ids.push(request_id);
        }

        request_ids
    }

    /// Get the next request to process (priority first, then FIFO)
    pub fn pop_request(&mut self) -> Option<QueueRequest> {
        // Check priority queue first
        if let Some(request) = self.priority_queue.pop() {
            return Some(request);
        }

        // Then check normal queue
        self.normal_queue.pop_front()
    }

    /// Add a response to history
    pub fn push_response(&mut self, response: QueueResponse) {
        // Find matching request in history and update response
        for (req, resp) in &mut self.history {
            if *Self::get_request_id_static(req) == response.request_id {
                *resp = response.clone();
                return;
            }
        }
    }

    /// Start processing a request (add to history)
    pub fn start_processing(&mut self, request: QueueRequest) {
        let placeholder_response = QueueResponse {
            request_id: Self::get_request_id_static(&request).clone(),
            success: false,
            content: "Processing...".to_string(),
            completed_task_ids: Vec::new(),
        };
        
        self.history.push((request, placeholder_response));
    }

    /// Check if there are pending requests
    pub fn has_pending_requests(&self) -> bool {
        !self.priority_queue.is_empty() || !self.normal_queue.is_empty()
    }

    /// Get the number of pending requests
    pub fn pending_count(&self) -> usize {
        self.priority_queue.len() + self.normal_queue.len()
    }

    /// Clear all pending requests
    pub fn clear_all(&mut self) {
        self.priority_queue.clear();
        self.normal_queue.clear();
    }

    /// Get request ID from any QueueRequest (static version)
    fn get_request_id_static(request: &QueueRequest) -> &String {
        match request {
            QueueRequest::UserPrompt { id, .. } => id,
            QueueRequest::TaskExecution { id, .. } => id,
        }
    }

    /// Get request ID from any QueueRequest
    fn get_request_id<'a>(&self, request: &'a QueueRequest) -> &'a String {
        Self::get_request_id_static(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_basic_operations() {
        let mut queue = ExecutionQueue::new();
        
        // Test normal priority prompt
        let id1 = queue.push_user_prompt("Normal task".to_string(), 3);
        assert_eq!(queue.pending_count(), 1);
        
        // Test high priority prompt
        let id2 = queue.push_user_prompt("Urgent task".to_string(), 7);
        assert_eq!(queue.pending_count(), 2);
        
        // High priority should be popped first
        let next = queue.pop_request().unwrap();
        match next {
            QueueRequest::UserPrompt { id, priority, .. } => {
                assert_eq!(id, id2);
                assert_eq!(priority, 7);
            }
            _ => panic!("Expected UserPrompt"),
        }
        
        // Then normal priority
        let next = queue.pop_request().unwrap();
        match next {
            QueueRequest::UserPrompt { id, priority, .. } => {
                assert_eq!(id, id1);
                assert_eq!(priority, 3);
            }
            _ => panic!("Expected UserPrompt"),
        }
        
        assert_eq!(queue.pending_count(), 0);
    }

    #[test]
    fn test_task_execution_queue() {
        let mut queue = ExecutionQueue::new();
        let task = Task::new(
            1,
            "Test task".to_string(),
            "bash".to_string(),
            "test.sh".to_string(),
            "run test".to_string(),
        );
        
        let id = queue.push_task_execution("plan_1".to_string(), task);
        assert_eq!(queue.pending_count(), 1);
        
        let request = queue.pop_request().unwrap();
        match request {
            QueueRequest::TaskExecution { id: req_id, plan_id, .. } => {
                assert_eq!(req_id, id);
                assert_eq!(plan_id, "plan_1");
            }
            _ => panic!("Expected TaskExecution"),
        }
    }
}