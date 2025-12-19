use serde::{Deserialize, Serialize};

// Client hello handshake message
#[derive(Debug, Clone, Serialize)]
pub struct ClientHello {
    #[serde(rename = "type")]
    pub msg_type: String, // "hello"
    #[serde(rename = "clientType")]
    pub client_type: String, // "tui"
    #[serde(rename = "clientId")]
    pub client_id: String,
    pub version: String,
}

impl ClientHello {
    pub fn new(client_id: String) -> Self {
        Self {
            msg_type: "hello".to_string(),
            client_type: "tui".to_string(),
            client_id,
            version: "0.1.0".to_string(),
        }
    }
}

// Server hello response
#[derive(Debug, Clone, Deserialize)]
pub struct ServerHello {
    #[serde(rename = "daemonVersion")]
    pub daemon_version: String,
    pub sessions: Vec<SessionSummary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub name: String,
    #[serde(rename = "projectPath")]
    pub project_path: String,
    #[serde(rename = "appStatus")]
    pub app_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringEvent {
    pub ts: String,
    pub source: String,
    /// Event type - daemon sends as "eventType", some legacy sources use "type"
    #[serde(alias = "type", rename = "eventType")]
    pub event_type: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Session info matching SessionInfo from the server.
/// Represents a session that binds a project directory to a dedicated agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    #[serde(rename = "projectPath")]
    pub project_path: String,
    /// App status: "not_running", "starting", "running", "stopped", "error"
    #[serde(rename = "appStatus")]
    pub app_status: String,
    #[serde(rename = "vmServiceUri", default)]
    pub vm_service_uri: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(rename = "connectedClients", default)]
    pub connected_clients: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "lastActiveAt")]
    pub last_active_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutgoingMessage {
    Command {
        id: String,
        #[serde(rename = "clientId")]
        client_id: String,
        action: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncomingMessage {
    CommandResponse(CommandResponse),
    AgentResponse(AgentResponse),
    /// Monitoring events from daemon (type: "event")
    Event(MonitoringEvent),
    /// Streaming agent events (type: "agent_stream")
    AgentStream(AgentStreamEvent),
}

/// Streaming event from agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStreamEvent {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub event: AgentEventKind,
}

/// Types of streaming events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEventKind {
    TextDelta { text: String },
    ToolCallStart { 
        #[serde(rename = "toolName")]
        tool_name: String, 
        #[serde(rename = "toolCallId")]
        tool_call_id: String 
    },
    ToolCallEnd { 
        #[serde(rename = "toolName")]
        tool_name: String, 
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        result: Option<String>,
    },
    MessageComplete,  // Streaming finished for current message (immediate feedback)
    TaskComplete { summary: String },  // Full task/turn complete
    Error { message: String },
    /// User message from another client (e.g., MCP/Amp)
    UserMessage {
        text: String,
        #[serde(rename = "clientId")]
        client_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub id: String,
    pub success: bool,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub status: AgentStatus,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub question: Option<String>,
    /// Claude SDK session ID (managed by daemon, not used by TUI)
    #[serde(rename = "sdkSessionId", default)]
    pub sdk_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Success,
    NeedsContext,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_response() {
        let json = r#"{"type":"command_response","id":"123","success":true,"data":{"session":{"id":"sess-1","name":"test","projectPath":"/path","appStatus":"not_running","createdAt":"2024-01-01T00:00:00Z","lastActiveAt":"2024-01-01T00:00:00Z"}}}"#;
        let msg: IncomingMessage = serde_json::from_str(json).expect("parse failed");
        match msg {
            IncomingMessage::CommandResponse(resp) => {
                assert!(resp.success);
                assert_eq!(resp.id, "123");
                assert!(resp.data.get("session").is_some());
            }
            _ => panic!("Expected CommandResponse, got {:?}", msg),
        }
    }

    #[test]
    fn test_parse_session_from_response() {
        let json = r#"{"type":"command_response","id":"123","success":true,"data":{"session":{"id":"sess-1","name":"test","projectPath":"/path","appStatus":"not_running","createdAt":"2024-01-01T00:00:00Z","lastActiveAt":"2024-01-01T00:00:00Z"}}}"#;
        let msg: IncomingMessage = serde_json::from_str(json).expect("parse failed");
        if let IncomingMessage::CommandResponse(resp) = msg {
            let session_value = resp.data.get("session").expect("no session");
            let session: Session = serde_json::from_value(session_value.clone()).expect("parse session failed");
            assert_eq!(session.id, "sess-1");
            assert_eq!(session.name, "test");
            assert_eq!(session.project_path, "/path");
            assert_eq!(session.app_status, "not_running");
        }
    }

    #[test]
    fn test_parse_sessions_list() {
        let json = r#"{"type":"command_response","id":"123","success":true,"data":{"sessions":[{"id":"sess-1","name":"test","projectPath":"/path","appStatus":"running","vmServiceUri":"ws://127.0.0.1:5678","pid":1234,"createdAt":"2024-01-01T00:00:00Z","lastActiveAt":"2024-01-01T00:00:00Z"}]}}"#;
        let msg: IncomingMessage = serde_json::from_str(json).expect("parse failed");
        if let IncomingMessage::CommandResponse(resp) = msg {
            let sessions_arr = resp.data.get("sessions").and_then(|s| s.as_array()).expect("no sessions");
            let sessions: Vec<Session> = serde_json::from_value(serde_json::Value::Array(sessions_arr.clone())).expect("parse sessions failed");
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].id, "sess-1");
            assert_eq!(sessions[0].pid, Some(1234));
        }
    }

    #[test]
    fn test_serialize_create_session_command() {
        let msg = OutgoingMessage::Command {
            id: "123".to_string(),
            client_id: "client-456".to_string(),
            action: "create_session".to_string(),
            key: None,
            data: Some(serde_json::json!({
                "name": "my-session",
                "projectPath": "/path/to/project",
            })),
        };
        let json = serde_json::to_string(&msg).expect("serialize failed");
        assert!(json.contains(r#""type":"command""#));
        assert!(json.contains(r#""clientId":"client-456""#));
        assert!(json.contains(r#""action":"create_session""#));
        assert!(json.contains(r#""name":"my-session""#));
        assert!(json.contains(r#""projectPath":"/path/to/project""#));
    }

    #[test]
    fn test_monitoring_event_fallback() {
        // Events with eventType field should parse as Event
        let json = r#"{"ts":"2024-01-01T00:00:00Z","source":"flutter","type":"event","eventType":"flutter.log","payload":{"line":"hello"}}"#;
        let msg: IncomingMessage = serde_json::from_str(json).expect("parse failed");
        match msg {
            IncomingMessage::Event(event) => {
                assert_eq!(event.source, "flutter");
                assert_eq!(event.event_type, "flutter.log");
            }
            _ => panic!("Expected Event, got {:?}", msg),
        }
    }

    #[test]
    fn test_session_destroyed_event() {
        let json = r#"{"ts":"2024-01-01T00:00:00Z","source":"session","type":"event","eventType":"session.destroyed","payload":{"sessionId":"sess-123"}}"#;
        let msg: IncomingMessage = serde_json::from_str(json).expect("parse failed");
        match msg {
            IncomingMessage::Event(event) => {
                assert_eq!(event.source, "session");
                assert_eq!(event.event_type, "session.destroyed");
                assert_eq!(event.payload["sessionId"], "sess-123");
            }
            _ => panic!("Expected Event, got {:?}", msg),
        }
    }
}

impl AgentResponse {
    pub fn to_monitoring_event(&self) -> MonitoringEvent {
        let (event_type, payload) = match self.status {
            AgentStatus::Success => (
                "agent_success".to_string(),
                serde_json::json!({
                    "summary": self.summary,
                }),
            ),
            AgentStatus::NeedsContext => (
                "agent_needs_context".to_string(),
                serde_json::json!({
                    "question": self.question,
                }),
            ),
            AgentStatus::Error => (
                "agent_error".to_string(),
                serde_json::json!({
                    "error": self.summary,
                }),
            ),
        };

        MonitoringEvent {
            ts: chrono::Utc::now().to_rfc3339(),
            source: "agent".to_string(),
            event_type,
            payload,
        }
    }
}
