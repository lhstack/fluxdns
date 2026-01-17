// LLM Client - Unified OpenAI-Compatible Client
//
// This client works with any LLM provider that implements the OpenAI API specification.

use anyhow::{Context, Result};
use reqwest::Client;
use std::sync::Arc;

use super::config::LlmConfig;
use super::functions::FunctionRegistry;
use super::types::*;

/// Unified LLM client for OpenAI-compatible APIs
pub struct LlmClient {
    http_client: Client,
    config: Arc<LlmConfig>,
    function_registry: Arc<FunctionRegistry>,
}

impl LlmClient {
    /// Create a new LLM client with the given configuration
    pub fn new(config: LlmConfig, function_registry: Arc<FunctionRegistry>) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            config: Arc::new(config),
            function_registry,
        }
    }

    /// Update the configuration (e.g., when user changes settings)
    #[allow(dead_code)]
    pub fn update_config(&mut self, config: LlmConfig) {
        self.config = Arc::new(config);
    }

    /// Get the current configuration
    #[allow(dead_code)]
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// Send a chat completion request
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatCompletionResponse> {
        let tools = self.function_registry.get_tool_definitions();
        
        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            tool_choice: None,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: Some(false),
        };

        self.send_request(request).await
    }

    /// Send a chat completion request with custom parameters
    #[allow(dead_code)]
    pub async fn chat_with_options(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatCompletionResponse> {
        let tools = self.function_registry.get_tool_definitions();
        
        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            tool_choice: None,
            temperature,
            max_tokens,
            stream: Some(false),
        };

        self.send_request(request).await
    }

    /// Send the actual HTTP request to the LLM API
    /// Send the actual HTTP request to the LLM API
    async fn send_request(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.config.api_base_url.trim_end_matches('/'));
        
        // Log request details
        tracing::info!("Sending LLM Request to: {}", url);
        tracing::info!("Model: {}", &request.model);
        
        let request_body = serde_json::to_string(&request).unwrap_or_default();
        tracing::debug!("Request Body: {}", request_body);

        let response = self.http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to LLM API")?;

        let status = response.status();
        tracing::info!("LLM Response Status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("LLM API Error Body: {}", error_text);
            anyhow::bail!("LLM API error ({}): {}", status, error_text);
        }

        let response_text = response.text().await.context("Failed to read response body")?;
        tracing::debug!("Response Body: {}", response_text);

        let completion: ChatCompletionResponse = serde_json::from_str(&response_text)
            .context(format!("Failed to parse LLM response: {}", response_text))?;

        Ok(completion)
    }

    /// Process a user message, handling function calls automatically
    pub async fn process_message(
        &self,
        messages: &mut Vec<ChatMessage>,
        user_message: String,
    ) -> Result<String> {
        // Add user message
        messages.push(ChatMessage {
            role: Role::User,
            content: Some(user_message),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        // Loop to handle function calls
        loop {
            let response = self.chat(messages.clone()).await?;
            
            let choice = response.choices.first()
                .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;
            
            let assistant_message = choice.message.clone();
            messages.push(assistant_message.clone());

            // Check if there are tool calls to execute
            if let Some(tool_calls) = &assistant_message.tool_calls {
                if tool_calls.is_empty() {
                    // No more tool calls, return the content
                    return Ok(assistant_message.content.unwrap_or_default());
                }

                // Execute each tool call
                for tool_call in tool_calls {
                    let result = self.function_registry
                        .execute(&tool_call.function.name, &tool_call.function.arguments)
                        .await;

                    let result_json = serde_json::to_string(&result)?;

                    // Add the function result as a new message
                    messages.push(ChatMessage {
                        role: Role::Tool,
                        content: Some(result_json),
                        name: Some(tool_call.function.name.clone()),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
                // Continue the loop to get the next response
            } else {
                // No tool calls, return the content
                return Ok(assistant_message.content.unwrap_or_default());
            }

            // Safety check to prevent infinite loops
            if messages.len() > 50 {
                anyhow::bail!("Too many messages in conversation, possible infinite loop");
            }
        }
    }

    /// Test the connection to the LLM API
    pub async fn test_connection(&self) -> Result<bool> {
        let messages = vec![ChatMessage {
            role: Role::User,
            content: Some("Hello, please respond with 'OK' if you can hear me.".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            tools: None,
            tool_choice: None,
            temperature: Some(0.0),
            max_tokens: Some(10),
            stream: Some(false),
        };

        match self.send_request(request).await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::warn!("LLM connection test failed: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: Role::User,
            content: Some("Hello".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }
}
