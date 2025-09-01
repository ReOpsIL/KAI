use crate::planer::plan::Plan;
use crate::planer::task::Task;
use std::collections::VecDeque;

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
    pub decomposed_tasks: Option<Vec<Task>>,
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
            let request_id = self.push_task_execution(format!("plan_{}", plan.title), task.clone());
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
            decomposed_tasks: None,
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
