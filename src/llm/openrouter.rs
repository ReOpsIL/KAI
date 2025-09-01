use crate::tools::get_all_tools;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// OpenRouter API client for handling prompts
#[derive(Clone)]
pub struct OpenRouterClient {
    client: Client,
    api_key: String,
    base_url: String,
}

/// Request structure for OpenRouter API
#[derive(Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
}

/// Message structure for chat requests
#[derive(Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Response structure from OpenRouter API
#[derive(Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl OpenRouterClient {
    /// Create a new OpenRouter API client
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }

    /// Send a prompt to OpenRouter API and get response
    pub async fn send_prompt(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<ChatResponse, Box<dyn Error>> {
        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens,
            temperature,
            tools: None,
            tool_choice: None,
        };

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("API request failed: {}", response.status()).into());
        }

        let chat_response: ChatResponse = response.json().await?;
        Ok(chat_response)
    }

    /// Send a conversation to OpenRouter API
    pub async fn send_conversation(
        &self,
        model: &str,
        messages: Vec<Message>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<ChatResponse, Box<dyn Error>> {
        // Get file system tools and convert to JSON format for OpenRouter
        let all_tools = get_all_tools();
        let tools_json: Vec<serde_json::Value> = all_tools
            .iter()
            .map(|tool| serde_json::to_value(tool).unwrap())
            .collect();

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            max_tokens,
            temperature,
            tools: Some(tools_json),
            tool_choice: Some("auto".to_string()),
        };

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("API request failed: {}", response.status()).into());
        }

        let chat_response: ChatResponse = response.json().await?;
        Ok(chat_response)
    }

    /// Create a system message for conversation context
    pub fn create_system_message(content: &str) -> Message {
        Message {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    /// Create a user message
    pub fn create_user_message(content: &str) -> Message {
        Message {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    /// Create an assistant message
    pub fn create_assistant_message(content: &str) -> Message {
        Message {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }
}

/// Utility functions for prompt handling
pub mod utils {
    use super::Message;

    /// Build a conversation from alternating user and assistant messages
    pub fn build_conversation(messages: &[(String, String)]) -> Vec<Message> {
        let mut conversation = Vec::new();

        for (user_msg, assistant_msg) in messages {
            conversation.push(Message {
                role: "user".to_string(),
                content: user_msg.clone(),
            });

            if !assistant_msg.is_empty() {
                conversation.push(Message {
                    role: "assistant".to_string(),
                    content: assistant_msg.clone(),
                });
            }
        }

        conversation
    }

    /// Extract the response content from a ChatResponse
    pub fn extract_response_content(response: &super::ChatResponse) -> Option<String> {
        response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
    }
}
