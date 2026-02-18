use serde::{Deserialize, Serialize};

use super::function::ToolCall;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OllamaRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Option<Vec<super::function::Tool>>,
    pub stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct OllamaResponse {
    pub message: Message,
    pub done: bool,
    #[serde(default)]
    pub error: Option<String>,
}
