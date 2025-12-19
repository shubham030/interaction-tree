//! Mock daemon WebSocket server for testing.
//!
//! This provides a fake daemon that the TUI can connect to for integration testing
//! without needing the real daemon running.

use std::sync::Arc;
use tokio::sync::Mutex;
use tungstenite::Message;
use ws_mock::matchers::{Any, StringExact};
use ws_mock::ws_mock_server::{WsMock, WsMockServer};

use super::fixtures::mock_session;

/// A mock daemon for testing TUI WebSocket interactions.
pub struct MockDaemon {
    server: WsMockServer,
    /// Track received commands for assertions
    pub received_commands: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl MockDaemon {
    /// Start a new mock daemon on a random port.
    pub async fn start() -> Self {
        let server = WsMockServer::start().await;
        Self {
            server,
            received_commands: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get the WebSocket URL to connect to.
    pub async fn url(&self) -> String {
        self.server.uri().await
    }

    /// Mount a mock that responds to hello with hello_ack.
    pub async fn mock_hello(&self) {
        let hello_ack = serde_json::json!({
            "type": "hello_ack",
            "daemonVersion": "0.1.0-test",
            "sessions": []
        });

        WsMock::new()
            .matcher(Any::new())
            .respond_with(Message::text(hello_ack.to_string()))
            .expect(1)
            .mount(&self.server)
            .await;
    }

    /// Mount a mock that responds to hello with existing sessions.
    pub async fn mock_hello_with_sessions(&self, sessions: Vec<serde_json::Value>) {
        let hello_ack = serde_json::json!({
            "type": "hello_ack",
            "daemonVersion": "0.1.0-test",
            "sessions": sessions
        });

        WsMock::new()
            .matcher(Any::new())
            .respond_with(Message::text(hello_ack.to_string()))
            .expect(1)
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for list_sessions command.
    pub async fn mock_list_sessions(&self, sessions: Vec<serde_json::Value>) {
        let response = serde_json::json!({
            "type": "command_response",
            "id": "mock-id",
            "success": true,
            "data": {
                "sessions": sessions
            }
        });

        WsMock::new()
            .matcher(StringExact::new(r#""action":"list_sessions""#))
            .respond_with(Message::text(response.to_string()))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for create_session command.
    pub async fn mock_create_session(&self, session_id: &str, session_name: &str) {
        let session = mock_session(session_id, session_name, "not_running");
        let response = serde_json::json!({
            "type": "command_response",
            "id": "mock-id",
            "success": true,
            "data": {
                "session": session
            }
        });

        WsMock::new()
            .matcher(StringExact::new(r#""action":"create_session""#))
            .respond_with(Message::text(response.to_string()))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for get_status command.
    pub async fn mock_get_status(&self, session: Option<serde_json::Value>) {
        let response = serde_json::json!({
            "type": "command_response",
            "id": "mock-id",
            "success": true,
            "data": {
                "daemon": {
                    "version": "0.1.0-test",
                    "uptime": 1000
                },
                "currentSession": session
            }
        });

        WsMock::new()
            .matcher(StringExact::new(r#""action":"get_status""#))
            .respond_with(Message::text(response.to_string()))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for run_app command.
    pub async fn mock_run_app(&self, pid: u32) {
        let response = serde_json::json!({
            "type": "command_response",
            "id": "mock-id",
            "success": true,
            "data": {
                "pid": pid
            }
        });

        WsMock::new()
            .matcher(StringExact::new(r#""action":"run_app""#))
            .respond_with(Message::text(response.to_string()))
            .mount(&self.server)
            .await;
    }

    /// Mount a generic success response for any command.
    pub async fn mock_success_response(&self) {
        let response = serde_json::json!({
            "type": "command_response",
            "id": "mock-id",
            "success": true,
            "data": {}
        });

        WsMock::new()
            .matcher(Any::new())
            .respond_with(Message::text(response.to_string()))
            .mount(&self.server)
            .await;
    }

    /// Verify all mock expectations were met.
    pub async fn verify(&self) {
        self.server.verify().await;
    }
}

/// Builder for creating mock daemon responses.
pub struct MockResponseBuilder {
    msg_type: String,
    id: String,
    success: bool,
    data: serde_json::Value,
    error: Option<String>,
}

impl MockResponseBuilder {
    pub fn command_response() -> Self {
        Self {
            msg_type: "command_response".to_string(),
            id: "mock-id".to_string(),
            success: true,
            data: serde_json::json!({}),
            error: None,
        }
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    pub fn error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    pub fn build(self) -> serde_json::Value {
        let mut resp = serde_json::json!({
            "type": self.msg_type,
            "id": self.id,
            "success": self.success,
            "data": self.data
        });

        if let Some(err) = self.error {
            resp["error"] = serde_json::Value::String(err);
        }

        resp
    }
}

/// Builder for creating mock daemon events.
pub struct MockEventBuilder {
    ts: String,
    source: String,
    event_type: String,
    payload: serde_json::Value,
}

impl MockEventBuilder {
    pub fn new(event_type: &str) -> Self {
        Self {
            ts: "2024-01-01T00:00:00Z".to_string(),
            source: "daemon".to_string(),
            event_type: event_type.to_string(),
            payload: serde_json::json!({}),
        }
    }

    pub fn session_status_changed(session_id: &str, status: &str) -> Self {
        Self::new("session.status_changed")
            .source("session")
            .payload(serde_json::json!({
                "sessionId": session_id,
                "status": status
            }))
    }

    pub fn session_status_running(session_id: &str, pid: u32, vm_uri: &str) -> Self {
        Self::new("session.status_changed")
            .source("session")
            .payload(serde_json::json!({
                "sessionId": session_id,
                "status": "running",
                "pid": pid,
                "vmServiceUri": vm_uri
            }))
    }

    pub fn flutter_log(line: &str) -> Self {
        Self::new("flutter.log")
            .source("flutter")
            .payload(serde_json::json!({ "line": line }))
    }

    pub fn source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }

    pub fn payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn build(self) -> serde_json::Value {
        serde_json::json!({
            "type": "event",
            "ts": self.ts,
            "source": self.source,
            "eventType": self.event_type,
            "payload": self.payload
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_daemon_starts() {
        let daemon = MockDaemon::start().await;
        let url = daemon.url().await;
        assert!(url.starts_with("ws://"));
    }

    #[test]
    fn test_response_builder() {
        let resp = MockResponseBuilder::command_response()
            .id("test-123")
            .data(serde_json::json!({"key": "value"}))
            .build();

        assert_eq!(resp["type"], "command_response");
        assert_eq!(resp["id"], "test-123");
        assert_eq!(resp["success"], true);
        assert_eq!(resp["data"]["key"], "value");
    }

    #[test]
    fn test_event_builder_status_changed() {
        let event = MockEventBuilder::session_status_changed("sess-1", "running").build();

        assert_eq!(event["type"], "event");
        assert_eq!(event["source"], "session");
        assert_eq!(event["eventType"], "session.status_changed");
        assert_eq!(event["payload"]["sessionId"], "sess-1");
        assert_eq!(event["payload"]["status"], "running");
    }

    #[test]
    fn test_event_builder_flutter_log() {
        let event = MockEventBuilder::flutter_log("flutter: Hello world").build();

        assert_eq!(event["eventType"], "flutter.log");
        assert_eq!(event["source"], "flutter");
        assert_eq!(event["payload"]["line"], "flutter: Hello world");
    }
}
