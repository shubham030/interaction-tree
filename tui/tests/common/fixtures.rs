//! Test fixtures for creating App states and mock data.

use std::path::PathBuf;

use it_tui::app::{App, AppStatus, ContentTab, Mode, WsState};
use it_tui::project::ProjectInfo;
use it_tui::ws::protocol::{MonitoringEvent, Session};

/// Create a test project info
pub fn test_project() -> ProjectInfo {
    ProjectInfo {
        path: PathBuf::from("/test/project"),
        name: "test_app".to_string(),
        is_flutter: true,
    }
}

/// Create a basic App instance for testing
pub fn test_app() -> App {
    App::new("ws://localhost:9877".to_string(), 100, test_project())
}

/// Create an App that's connected to the daemon
pub fn connected_app() -> App {
    let mut app = test_app();
    app.ws_state = WsState::Connected;
    app
}

/// Create an App with sessions loaded
pub fn app_with_sessions() -> App {
    let mut app = connected_app();
    app.set_sessions(vec![
        mock_session("sess-1", "app-one", "running"),
        mock_session("sess-2", "app-two", "not_running"),
        mock_session("sess-3", "app-three", "starting"),
    ]);
    app
}

/// Create a mock session
pub fn mock_session(id: &str, name: &str, status: &str) -> Session {
    Session {
        id: id.to_string(),
        name: name.to_string(),
        project_path: format!("/projects/{}", name),
        app_status: status.to_string(),
        vm_service_uri: if status == "running" {
            Some("ws://127.0.0.1:5678/ws".to_string())
        } else {
            None
        },
        pid: if status == "running" { Some(12345) } else { None },
        connected_clients: Vec::new(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        last_active_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

/// Create a session status changed event
pub fn status_changed_event(session_id: &str, status: &str) -> MonitoringEvent {
    MonitoringEvent {
        ts: "2024-01-01T00:00:00Z".to_string(),
        source: "session".to_string(),
        event_type: "session.status_changed".to_string(),
        payload: serde_json::json!({
            "sessionId": session_id,
            "status": status,
        }),
    }
}

/// Create a session status changed event with full running info
pub fn status_running_event(session_id: &str, pid: u32, vm_uri: &str) -> MonitoringEvent {
    MonitoringEvent {
        ts: "2024-01-01T00:00:00Z".to_string(),
        source: "session".to_string(),
        event_type: "session.status_changed".to_string(),
        payload: serde_json::json!({
            "sessionId": session_id,
            "status": "running",
            "pid": pid,
            "vmServiceUri": vm_uri,
        }),
    }
}

/// Create a session destroyed event
pub fn session_destroyed_event(session_id: &str) -> MonitoringEvent {
    MonitoringEvent {
        ts: "2024-01-01T00:00:00Z".to_string(),
        source: "session".to_string(),
        event_type: "session.destroyed".to_string(),
        payload: serde_json::json!({
            "sessionId": session_id,
        }),
    }
}

/// Create a session created event
pub fn session_created_event(session: &Session) -> MonitoringEvent {
    MonitoringEvent {
        ts: "2024-01-01T00:00:00Z".to_string(),
        source: "session".to_string(),
        event_type: "session.created".to_string(),
        payload: serde_json::json!({
            "session": session,
        }),
    }
}

/// Create a flutter log event
pub fn flutter_log_event(line: &str) -> MonitoringEvent {
    MonitoringEvent {
        ts: "2024-01-01T00:00:00Z".to_string(),
        source: "flutter".to_string(),
        event_type: "flutter.log".to_string(),
        payload: serde_json::json!({
            "line": line,
        }),
    }
}

/// Create an agent event
pub fn agent_event(event_type: &str, payload: serde_json::Value) -> MonitoringEvent {
    MonitoringEvent {
        ts: "2024-01-01T00:00:00Z".to_string(),
        source: "agent".to_string(),
        event_type: event_type.to_string(),
        payload,
    }
}
