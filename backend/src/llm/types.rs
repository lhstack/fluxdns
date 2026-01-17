// LLM Type Definitions
// OpenAI-compatible request/response structures

use serde::{Deserialize, Serialize};

/// Chat message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Function,
    Tool,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Tool call from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Function definition for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool definition (wrapper for function)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

impl ToolDefinition {
    pub fn new(function: FunctionDefinition) -> Self {
        Self {
            tool_type: "function".to_string(),
            function,
        }
    }
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Tool choice specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Auto(String),
    None(String),
    Required(String),
    Specific { r#type: String, function: ToolChoiceFunction },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoiceFunction {
    pub name: String,
}

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Response choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Function execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl FunctionResult {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// LLM provider preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPreset {
    pub name: String,
    pub display_name: String,
    pub api_base_url: String,
    pub models: Vec<String>,
}

/// Get all supported provider presets
pub fn get_provider_presets() -> Vec<ProviderPreset> {
    vec![
        ProviderPreset {
            name: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            api_base_url: "https://api.openai.com/v1".to_string(),
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
        },
        ProviderPreset {
            name: "deepseek".to_string(),
            display_name: "DeepSeek".to_string(),
            api_base_url: "https://api.deepseek.com/v1".to_string(),
            models: vec![
                "deepseek-chat".to_string(),
                "deepseek-reasoner".to_string(),
            ],
        },
        ProviderPreset {
            name: "qwen".to_string(),
            display_name: "通义千问".to_string(),
            api_base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
            models: vec![
                "qwen-max".to_string(),
                "qwen-plus".to_string(),
                "qwen-turbo".to_string(),
            ],
        },
        ProviderPreset {
            name: "zhipu".to_string(),
            display_name: "智谱清言".to_string(),
            api_base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
            models: vec![
                "glm-4".to_string(),
                "glm-4-flash".to_string(),
            ],
        },
        ProviderPreset {
            name: "doubao".to_string(),
            display_name: "豆包".to_string(),
            api_base_url: "https://ark.cn-beijing.volces.com/api/v3".to_string(),
            models: vec![
                "doubao-pro-32k".to_string(),
                "doubao-lite-32k".to_string(),
            ],
        },
        ProviderPreset {
            name: "wenxin".to_string(),
            display_name: "文心一言".to_string(),
            api_base_url: "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop".to_string(),
            models: vec![
                "ernie-4.0".to_string(),
                "ernie-3.5".to_string(),
            ],
        },
        ProviderPreset {
            name: "moonshot".to_string(),
            display_name: "月之暗面".to_string(),
            api_base_url: "https://api.moonshot.cn/v1".to_string(),
            models: vec![
                "moonshot-v1-8k".to_string(),
                "moonshot-v1-32k".to_string(),
                "moonshot-v1-128k".to_string(),
            ],
        },
        ProviderPreset {
            name: "yi".to_string(),
            display_name: "零一万物".to_string(),
            api_base_url: "https://api.lingyiwanwu.com/v1".to_string(),
            models: vec![
                "yi-large".to_string(),
                "yi-medium".to_string(),
            ],
        },
    ]
}
