//! Unit tests for TUI event handling logic.
//!
//! These tests verify that the App correctly handles various events
//! without needing a real terminal or WebSocket connection.
//!
//! Run with: cargo test --test event_handling_tests

mod common;

use it_tui::app::{App, AppStatus, ContentTab, Mode, Toast, WsState};
use it_tui::ws::protocol::{
    AgentResponse, AgentStatus, CommandResponse, MonitoringEvent, Session,
};

use common::fixtures::*;

// =============================================================================
// Session Status Event Tests
// =============================================================================

#[test]
fn test_status_starting_updates_app_status() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.app_status = AppStatus::Stopped;

    let event = status_changed_event("sess-1", "starting");
    app.push_event(event);

    assert!(matches!(app.app_status, AppStatus::Starting));
}

#[test]
fn test_status_running_updates_with_pid_and_uri() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.app_status = AppStatus::Starting;

    let event = status_running_event("sess-1", 54321, "ws://127.0.0.1:9999/ws");
    app.push_event(event);

    match app.app_status {
        AppStatus::Running { pid, uri } => {
            assert_eq!(pid, 54321);
            assert_eq!(uri, "ws://127.0.0.1:9999/ws");
        }
        _ => panic!("Expected Running status, got {:?}", app.app_status),
    }
}

#[test]
fn test_status_stopped_updates_app_status() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.app_status = AppStatus::Running {
        pid: 12345,
        uri: "ws://127.0.0.1:5678/ws".to_string(),
    };

    let event = status_changed_event("sess-1", "stopped");
    app.push_event(event);

    assert!(matches!(app.app_status, AppStatus::Stopped));
}

#[test]
fn test_status_event_for_different_session_ignored() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.app_status = AppStatus::Stopped;

    // Event for sess-2, but we're connected to sess-1
    let event = status_changed_event("sess-2", "running");
    app.push_event(event);

    // Should still be stopped since event was for different session
    assert!(matches!(app.app_status, AppStatus::Stopped));
}

#[test]
fn test_status_transitions_in_correct_order() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.app_status = AppStatus::Stopped;

    // Simulate: stopped -> starting -> running
    app.push_event(status_changed_event("sess-1", "starting"));
    assert!(matches!(app.app_status, AppStatus::Starting));

    app.push_event(status_running_event("sess-1", 12345, "ws://localhost:5678/ws"));
    assert!(matches!(app.app_status, AppStatus::Running { .. }));
}

// =============================================================================
// Session Lifecycle Event Tests
// =============================================================================

#[test]
fn test_session_created_adds_to_list() {
    let mut app = connected_app();
    assert!(app.sessions.is_empty());

    let new_session = mock_session("new-sess", "new-app", "not_running");
    let event = session_created_event(&new_session);
    app.push_event(event);

    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.sessions[0].id, "new-sess");
    assert_eq!(app.sessions[0].name, "new-app");
}

#[test]
fn test_session_created_does_not_duplicate() {
    let mut app = app_with_sessions();
    let initial_count = app.sessions.len();

    // Try to add existing session
    let existing = mock_session("sess-1", "app-one", "running");
    let event = session_created_event(&existing);
    app.push_event(event);

    // Should not add duplicate
    assert_eq!(app.sessions.len(), initial_count);
}

#[test]
fn test_session_destroyed_removes_from_list() {
    let mut app = app_with_sessions();
    assert_eq!(app.sessions.len(), 3);

    let event = session_destroyed_event("sess-2");
    app.push_event(event);

    assert_eq!(app.sessions.len(), 2);
    assert!(!app.sessions.iter().any(|s| s.id == "sess-2"));
}

#[test]
fn test_session_destroyed_clears_selected_if_current() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    assert_eq!(app.selected_session, Some("sess-1".to_string()));

    let event = session_destroyed_event("sess-1");
    app.push_event(event);

    assert_eq!(app.selected_session, None);
    assert!(matches!(app.app_status, AppStatus::Stopped));
}

#[test]
fn test_session_destroyed_adjusts_picker_index() {
    let mut app = app_with_sessions();
    app.session_picker_index = 2; // Last session

    let event = session_destroyed_event("sess-3");
    app.push_event(event);

    // Picker index should adjust to stay valid
    assert!(app.session_picker_index < app.sessions.len());
}

// =============================================================================
// Command Response Tests
// =============================================================================

#[test]
fn test_command_response_running_status() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());

    let resp = CommandResponse {
        id: "1".to_string(),
        success: true,
        data: serde_json::json!({
            "status": "running",
            "pid": 99999,
            "uri": "ws://127.0.0.1:8888"
        }),
        error: None,
    };

    app.handle_command_response(resp);

    match app.app_status {
        AppStatus::Running { pid, uri } => {
            assert_eq!(pid, 99999);
            assert_eq!(uri, "ws://127.0.0.1:8888");
        }
        _ => panic!("Expected Running status"),
    }
}

#[test]
fn test_command_response_sessions_list() {
    let mut app = connected_app();
    assert!(app.sessions.is_empty());

    let resp = CommandResponse {
        id: "1".to_string(),
        success: true,
        data: serde_json::json!({
            "sessions": [
                {
                    "id": "sess-a",
                    "name": "app-a",
                    "projectPath": "/path/a",
                    "appStatus": "running",
                    "createdAt": "2024-01-01T00:00:00Z",
                    "lastActiveAt": "2024-01-01T00:00:00Z"
                },
                {
                    "id": "sess-b",
                    "name": "app-b",
                    "projectPath": "/path/b",
                    "appStatus": "stopped",
                    "createdAt": "2024-01-01T00:00:00Z",
                    "lastActiveAt": "2024-01-01T00:00:00Z"
                }
            ]
        }),
        error: None,
    };

    app.handle_command_response(resp);

    assert_eq!(app.sessions.len(), 2);
    assert_eq!(app.sessions[0].id, "sess-a");
    assert_eq!(app.sessions[1].id, "sess-b");
}

#[test]
fn test_command_response_create_session() {
    let mut app = connected_app();
    app.mode = Mode::SessionPicker;

    let resp = CommandResponse {
        id: "1".to_string(),
        success: true,
        data: serde_json::json!({
            "session": {
                "id": "new-sess",
                "name": "new-app",
                "projectPath": "/new/path",
                "appStatus": "not_running",
                "createdAt": "2024-01-01T00:00:00Z",
                "lastActiveAt": "2024-01-01T00:00:00Z"
            }
        }),
        error: None,
    };

    app.handle_command_response(resp);

    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.selected_session, Some("new-sess".to_string()));
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_command_response_error_creates_toast() {
    let mut app = connected_app();
    assert!(app.toasts.is_empty());

    let resp = CommandResponse {
        id: "1".to_string(),
        success: false,
        data: serde_json::json!({}),
        error: Some("Something failed".to_string()),
    };

    app.handle_command_response(resp);

    assert_eq!(app.toasts.len(), 1);
}

// =============================================================================
// Agent Response Tests
// =============================================================================

#[test]
fn test_agent_response_success_clears_pending() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.session.pending_response = true;

    let resp = AgentResponse {
        id: "1".to_string(),
        status: AgentStatus::Success,
        summary: Some("Task completed".to_string()),
        error: None,
        question: None,
        sdk_session_id: None,
    };

    app.handle_agent_response(resp);

    assert!(!app.session.pending_response);
    assert!(app.session.conversation_id.is_none());
}

#[test]
fn test_agent_response_needs_context_sets_conversation() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.session.pending_response = true;

    let resp = AgentResponse {
        id: "1".to_string(),
        status: AgentStatus::NeedsContext,
        summary: None,
        error: None,
        question: Some("Which button?".to_string()),
        sdk_session_id: Some("conv-123".to_string()),
    };

    app.handle_agent_response(resp);

    assert!(!app.session.pending_response);
    assert_eq!(app.session.conversation_id, Some("conv-123".to_string()));
}

#[test]
fn test_agent_response_error_sets_last_error() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.session.pending_response = true;

    let resp = AgentResponse {
        id: "1".to_string(),
        status: AgentStatus::Error,
        summary: Some("Tool call failed".to_string()),
        error: None,
        question: None,
        sdk_session_id: None,
    };

    app.handle_agent_response(resp);

    assert_eq!(
        app.session.last_agent_error,
        Some("Tool call failed".to_string())
    );
}

// =============================================================================
// Session Switching Tests
// =============================================================================

#[test]
fn test_switch_session_saves_and_restores_state() {
    let mut app = app_with_sessions();

    // Set up sess-1 with some state
    app.switch_to_session("sess-1".to_string());
    app.push_flutter_log(it_tui::flutter_log::FlutterLogEntry::parse("flutter: Hello from sess-1"));

    // Switch to sess-2
    app.switch_to_session("sess-2".to_string());
    assert!(app.session.flutter_logs.is_empty()); // sess-2 has no logs

    // Add logs to sess-2
    app.push_flutter_log(it_tui::flutter_log::FlutterLogEntry::parse("flutter: Hello from sess-2"));

    // Switch back to sess-1
    app.switch_to_session("sess-1".to_string());
    assert_eq!(app.session.flutter_logs.len(), 1);
    assert!(app.session.flutter_logs[0].raw.contains("sess-1"));
}

#[test]
fn test_switch_session_triggers_tree_fetch() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.needs_tree_fetch = false;

    app.switch_to_session("sess-2".to_string());

    assert!(app.needs_tree_fetch);
}

// =============================================================================
// Log/Event Filtering Tests
// =============================================================================

#[test]
fn test_filter_flutter_logs() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());

    app.push_flutter_log(it_tui::flutter_log::FlutterLogEntry::parse("flutter: Error message"));
    app.push_flutter_log(it_tui::flutter_log::FlutterLogEntry::parse("flutter: Info message"));
    app.push_flutter_log(it_tui::flutter_log::FlutterLogEntry::parse("flutter: Another error"));

    app.filter = Some("error".to_string());
    let filtered: Vec<_> = app.filtered_flutter_logs().collect();

    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_clear_events_resets_scroll() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.scroll_offset = 10;

    app.clear_events();

    assert_eq!(app.scroll_offset, 0);
}

// =============================================================================
// Toast Tests
// =============================================================================

#[test]
fn test_push_toast_limits_queue() {
    let mut app = test_app();

    for i in 0..10 {
        app.push_toast(Toast::info(format!("Toast {}", i)));
    }

    // Should only keep max 5
    assert!(app.toasts.len() <= 5);
}

// =============================================================================
// Navigation Tests
// =============================================================================

#[test]
fn test_tab_cycling() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.content_tab = ContentTab::Flutter;

    app.next_tab();
    assert_eq!(app.content_tab, ContentTab::Agent);

    app.next_tab();
    assert_eq!(app.content_tab, ContentTab::Interactions);

    app.next_tab();
    assert_eq!(app.content_tab, ContentTab::Tree);

    app.next_tab();
    assert_eq!(app.content_tab, ContentTab::Flutter);
}

#[test]
fn test_session_picker_navigation() {
    let mut app = app_with_sessions();
    app.mode = Mode::SessionPicker;
    app.session_picker_index = 0;

    app.session_picker_down();
    assert_eq!(app.session_picker_index, 1);

    app.session_picker_down();
    assert_eq!(app.session_picker_index, 2);

    app.session_picker_up();
    assert_eq!(app.session_picker_index, 1);
}

#[test]
fn test_session_picker_select() {
    let mut app = app_with_sessions();
    app.mode = Mode::SessionPicker;
    app.session_picker_index = 1; // sess-2

    app.session_picker_select();

    assert_eq!(app.selected_session, Some("sess-2".to_string()));
}
