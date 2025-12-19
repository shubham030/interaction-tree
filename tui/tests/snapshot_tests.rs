//! Snapshot tests for TUI rendering.
//!
//! These tests use ratatui's TestBackend and insta for snapshot testing.
//! They verify that UI components render correctly without needing a real terminal.
//!
//! Run with: cargo test --test snapshot_tests
//! Update snapshots: cargo insta review

mod common;

use insta::assert_snapshot;
use ratatui::{backend::TestBackend, Terminal};

use it_tui::app::{App, AppStatus, ContentTab, Mode, WsState};
use it_tui::project::ProjectInfo;
use it_tui::ws::protocol::Session;
use std::path::PathBuf;

use common::fixtures::{connected_app, app_with_sessions, mock_session, test_app};

/// Helper to create a terminal with TestBackend
fn test_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    Terminal::new(backend).unwrap()
}

/// Render the app and return the buffer as a string for snapshot testing
fn render_to_string(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = test_terminal(width, height);
    terminal.draw(|f| it_tui::ui::render(f, app)).unwrap();
    format!("{:?}", terminal.backend())
}

// =============================================================================
// Status Bar Tests
// =============================================================================

#[test]
fn test_status_bar_disconnected() {
    let mut app = test_app();
    app.ws_state = WsState::Disconnected;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_status_bar_connecting() {
    let mut app = test_app();
    app.ws_state = WsState::Connecting;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_status_bar_connected_no_session() {
    let mut app = connected_app();
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_status_bar_with_running_session() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.app_status = AppStatus::Running {
        pid: 12345,
        uri: "ws://127.0.0.1:5678/ws".to_string(),
    };
    app.mode = Mode::Normal;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_status_bar_starting() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-3".to_string());
    app.app_status = AppStatus::Starting;
    app.mode = Mode::Normal;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

// =============================================================================
// Session Picker Tests
// =============================================================================

#[test]
fn test_session_picker_empty() {
    let mut app = connected_app();
    app.mode = Mode::SessionPicker;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_session_picker_with_sessions() {
    let mut app = app_with_sessions();
    app.mode = Mode::SessionPicker;
    app.session_picker_index = 0;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_session_picker_second_selected() {
    let mut app = app_with_sessions();
    app.mode = Mode::SessionPicker;
    app.session_picker_index = 1;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

// =============================================================================
// Content Tab Tests
// =============================================================================

#[test]
fn test_flutter_tab_empty() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::Normal;
    app.content_tab = ContentTab::Flutter;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_agent_tab_empty() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::Normal;
    app.content_tab = ContentTab::Agent;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_interactions_tab_empty() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::Normal;
    app.content_tab = ContentTab::Interactions;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_tree_tab_no_tree() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::Normal;
    app.content_tab = ContentTab::Tree;
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

// =============================================================================
// Help Overlay Tests
// =============================================================================

#[test]
fn test_help_overlay() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::Help;
    
    assert_snapshot!(render_to_string(&mut app, 80, 30));
}

// =============================================================================
// Different Terminal Sizes
// =============================================================================

#[test]
fn test_small_terminal() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::Normal;
    
    assert_snapshot!(render_to_string(&mut app, 40, 12));
}

#[test]
fn test_wide_terminal() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::Normal;
    
    assert_snapshot!(render_to_string(&mut app, 120, 24));
}

// =============================================================================
// Toast Tests
// =============================================================================

#[test]
fn test_with_success_toast() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::Normal;
    app.push_toast(it_tui::app::Toast::success("Operation completed"));
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_with_error_toast() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::Normal;
    app.push_toast(it_tui::app::Toast::error("Something went wrong"));
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

// =============================================================================
// Filter Mode Tests  
// =============================================================================

#[test]
fn test_filter_mode() {
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::Filter;
    app.input_buffer = "flutter".to_string();
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

// =============================================================================
// Input Prompt Tests
// =============================================================================

#[test]
fn test_run_app_prompt() {
    use it_tui::app::InputPromptKind;
    
    let mut app = app_with_sessions();
    app.switch_to_session("sess-1".to_string());
    app.mode = Mode::InputPrompt(InputPromptKind::RunApp);
    app.input_buffer = "macos".to_string();
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}

#[test]
fn test_create_session_prompt() {
    use it_tui::app::InputPromptKind;
    
    let mut app = app_with_sessions();
    app.mode = Mode::InputPrompt(InputPromptKind::CreateSession);
    app.input_buffer = "my-new-session".to_string();
    
    assert_snapshot!(render_to_string(&mut app, 80, 24));
}
