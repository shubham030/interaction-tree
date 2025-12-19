//! Chat message types and rendering for the Agent pane.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

/// A chat message in the conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub role: ChatRole,
    pub content: ChatContent,
    pub timestamp: DateTime<Utc>,
    /// Client that sent this message (e.g., "tui", "mcp")
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub enum ChatContent {
    Text(String),
    ToolCall(ToolCall),
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
    pub status: ToolStatus,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Success,
    Failed,
}

/// Streaming state during agent response.
#[derive(Debug, Clone, Default)]
pub struct StreamingState {
    pub text_buffer: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: ChatRole::User,
            content: ChatContent::Text(content.into()),
            timestamp: Utc::now(),
            client_id: Some("tui".to_string()),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: ChatRole::Assistant,
            content: ChatContent::Text(content.into()),
            timestamp: Utc::now(),
            client_id: None,
        }
    }

    pub fn assistant_tool(call: ToolCall) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: ChatRole::Assistant,
            content: ChatContent::ToolCall(call),
            timestamp: Utc::now(),
            client_id: None,
        }
    }
}

/// Daemon chat message format for deserialization
#[derive(Debug, Deserialize)]
pub struct DaemonChatMessage {
    pub role: String,
    pub content: DaemonChatContent,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonChatContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_call")]
    ToolCall {
        name: String,
        #[serde(default)]
        args: Value,
        output: Option<String>,
        status: String,
    },
}

impl TryFrom<DaemonChatMessage> for ChatMessage {
    type Error = chrono::ParseError;

    fn try_from(msg: DaemonChatMessage) -> Result<Self, Self::Error> {
        let role = match msg.role.as_str() {
            "user" => ChatRole::User,
            _ => ChatRole::Assistant,
        };
        
        let timestamp = DateTime::parse_from_rfc3339(&msg.timestamp)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let content = match msg.content {
            DaemonChatContent::Text { text } => ChatContent::Text(text),
            DaemonChatContent::ToolCall { name, args, output, status } => {
                let tool_status = match status.as_str() {
                    "running" => ToolStatus::Running,
                    "failed" => ToolStatus::Failed,
                    _ => ToolStatus::Success,
                };
                ChatContent::ToolCall(ToolCall {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    args,
                    status: tool_status,
                    output,
                })
            }
        };

        Ok(ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content,
            timestamp,
            client_id: None,
        })
    }
}

/// Parse chat history from daemon JSON
pub fn parse_chat_history(value: &Value) -> Vec<ChatMessage> {
    let Some(array) = value.as_array() else {
        return Vec::new();
    };

    array
        .iter()
        .filter_map(|v| {
            serde_json::from_value::<DaemonChatMessage>(v.clone())
                .ok()
                .and_then(|msg| ChatMessage::try_from(msg).ok())
        })
        .collect()
}
