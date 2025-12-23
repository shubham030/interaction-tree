use std::io;
use std::time::Duration;

use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{
    self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::app::{App, ConfirmAction, ContentTab, InputPromptKind, InteractionAction, Mode, WsState};
use crate::ws::protocol::Session;
use crate::commands::{parse_command, TuiCommand};
use crate::ui;
use crate::ws::client::{WsClient, WsEvent};
use crate::ws::protocol::{IncomingMessage, OutgoingMessage, SessionSummary};

/// Copy text to system clipboard
fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

pub async fn run(app: &mut App) -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_event_loop(&mut terminal, app).await;

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;

    result
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let (ws_tx, mut ws_rx) = mpsc::channel::<WsEvent>(100);

    app.ws_state = WsState::Connecting;
    let ws_client = match WsClient::connect(&app.server_uri, ws_tx).await {
        Ok(client) => {
            app.ws_state = WsState::Connected;
            // Sessions are received from ServerHello, no need to request them
            Some(client)
        }
        Err(e) => {
            app.ws_state = WsState::Disconnected;
            app.push_toast(crate::app::Toast::error(format!("Connection failed: {}", e)));
            tracing::error!("Failed to connect to WebSocket: {}", e);
            None
        }
    };

    let mut last_throbber_tick = std::time::Instant::now();
    
    loop {
        // Expire old toasts
        app.expire_toasts();
        
        // Tick throbber at ~10 FPS (not every frame)
        if last_throbber_tick.elapsed() >= Duration::from_millis(100) {
            app.throbber_state.calc_next();
            last_throbber_tick = std::time::Instant::now();
        }

        // Auto-fetch tree if needed
        if app.needs_tree_fetch {
            app.needs_tree_fetch = false;
            if let Some(ref client) = ws_client {
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "get_tree".to_string(),
                    key: None,
                    data: None,
                };
                let _ = client.send(msg).await;
            }
        }

        terminal.draw(|f| ui::render(f, app))?;

        if app.should_quit {
            break;
        }

        // Use tokio::select! to handle both terminal and WebSocket events
        // Poll terminal with short timeout for responsive typing
        tokio::select! {
            biased;  // Prefer terminal events for responsiveness

            // Check for terminal events (non-blocking with short poll)
            result = tokio::task::spawn_blocking(|| {
                if event::poll(Duration::from_millis(10)).unwrap_or(false) {
                    Some(event::read())
                } else {
                    None
                }
            }) => {
                if let Ok(Some(Ok(evt))) = result {
                    match evt {
                        CrosstermEvent::Key(key) => {
                            handle_key_event(app, key, &ws_client).await?;
                        }
                        CrosstermEvent::Mouse(mouse) => {
                            handle_mouse_event(app, mouse);
                        }
                        _ => {}
                    }
                }
            }

            // Check for WebSocket events
            Some(ws_event) = ws_rx.recv() => {
                handle_ws_event(app, ws_event);
            }
        }
    }

    Ok(())
}

async fn handle_key_event(
    app: &mut App,
    key: KeyEvent,
    ws: &Option<WsClient>,
) -> Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Ok(());
    }

    match &app.mode {
        Mode::Normal => handle_normal_mode(app, key, ws).await?,
        Mode::Filter => handle_filter_mode(app, key),
        Mode::Help => handle_help_mode(app, key),
        Mode::Confirm(action) => handle_confirm_mode(app, key, action.clone()),
        Mode::SessionPicker => handle_session_picker_mode(app, key, ws).await?,
        Mode::InputPrompt(kind) => handle_input_prompt_mode(app, key, ws, kind.clone()).await?,
        Mode::AgentChat => handle_agent_chat_mode(app, key, ws).await?,
        Mode::ActionMenu => handle_action_menu_mode(app, key, ws).await?,
    }

    Ok(())
}

fn handle_mouse_event(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if app.content_tab == ContentTab::Tree {
                app.tree_up();
            } else {
                app.scroll_up();
            }
        }
        MouseEventKind::ScrollDown => {
            if app.content_tab == ContentTab::Tree {
                app.tree_down();
            } else {
                app.scroll_down();
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            // Future: could detect click position to switch tabs
        }
        _ => {}
    }
}

async fn handle_normal_mode(app: &mut App, key: KeyEvent, ws: &Option<WsClient>) -> Result<()> {
    // Tree tab has special navigation (tree_up/down instead of scroll)
    let on_tree_tab = app.content_tab == ContentTab::Tree;

    match key.code {
        KeyCode::Char('q') => {
            app.mode = Mode::Confirm(ConfirmAction::Quit);
        }
        KeyCode::Char('Q') => {
            app.should_quit = true;
        }
        KeyCode::Char('r') => {
            if on_tree_tab {
                // Refresh tree on Tree tab
                if app.is_app_running() {
                    if let Some(client) = ws {
                        let msg = OutgoingMessage::Command {
                            id: Uuid::new_v4().to_string(),
                            client_id: client.client_id().to_string(),
                            action: "get_tree".to_string(),
                            key: None,
                            data: None,
                        };
                        let _ = client.send(msg).await;
                        app.push_toast(crate::app::Toast::info("Tree refresh triggered"));
                    }
                }
            } else if app.content_tab == ContentTab::Flutter {
                // Hot reload on Flutter tab
                if app.is_app_running() {
                    if let Some(client) = ws {
                        let msg = OutgoingMessage::Command {
                            id: Uuid::new_v4().to_string(),
                            client_id: client.client_id().to_string(),
                            action: "hot_reload".to_string(),
                            key: None,
                            data: None,
                        };
                        let _ = client.send(msg).await;
                        app.push_toast(crate::app::Toast::info("Hot reload triggered"));
                    }
                }
            }
        }
        KeyCode::Char('R') => {
            // Hot restart only on Flutter tab
            if app.content_tab == ContentTab::Flutter && app.is_app_running() {
                if let Some(client) = ws {
                    let msg = OutgoingMessage::Command {
                        id: Uuid::new_v4().to_string(),
                        client_id: client.client_id().to_string(),
                        action: "hot_restart".to_string(),
                        key: None,
                        data: None,
                    };
                    let _ = client.send(msg).await;
                    app.push_toast(crate::app::Toast::info("Hot restart triggered"));
                }
            }
        }
        // Tree-specific: expand/collapse all
        KeyCode::Char('e') if on_tree_tab => {
            app.tree_expand_all();
        }
        KeyCode::Char('E') if on_tree_tab => {
            app.tree_collapse_all();
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Filter;
            app.input_buffer.clear();
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if on_tree_tab {
                app.tree_down();
            } else if app.content_tab == ContentTab::Agent {
                app.chat_scroll_down();
            } else {
                app.scroll_down();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if on_tree_tab {
                app.tree_up();
            } else if app.content_tab == ContentTab::Agent {
                app.chat_scroll_up();
            } else {
                app.scroll_up();
            }
        }
        KeyCode::Char('g') => {
            if on_tree_tab {
                // no-op for tree
            } else if app.content_tab == ContentTab::Agent {
                app.chat_scroll_to_top();
            } else {
                app.scroll_to_top();
            }
        }
        KeyCode::Char('G') => {
            if on_tree_tab {
                // no-op for tree
            } else if app.content_tab == ContentTab::Agent {
                app.chat_scroll_to_bottom();
            } else {
                app.scroll_to_bottom();
            }
        }
        KeyCode::Char('V') => {
            if !on_tree_tab {
                app.toggle_visual_mode();
            }
        }
        KeyCode::Char('y') => {
            if !on_tree_tab {
                if let Some(text) = app.yank_logs() {
                    // Copy to clipboard
                    if let Err(e) = copy_to_clipboard(&text) {
                        app.push_toast(crate::app::Toast::error(format!("Failed to copy: {}", e)));
                    } else {
                        let line_count = text.lines().count();
                        app.push_toast(crate::app::Toast::success(format!("{} line(s) yanked", line_count)));
                    }
                }
            }
        }
        KeyCode::Char('c') => {
            app.clear_events();
        }
        KeyCode::Char('f') => {
            app.filter = None;
        }
        // Tab navigation: h/l always switch tabs, arrow keys do tree collapse/expand on Tree tab
        KeyCode::Char('l') => {
            app.next_tab();
            maybe_fetch_tree(app, ws).await;
        }
        KeyCode::Char('h') => {
            app.prev_tab();
            maybe_fetch_tree(app, ws).await;
        }
        KeyCode::Right => {
            if on_tree_tab {
                app.tree_right(); // Expand node
            } else {
                app.next_tab();
                maybe_fetch_tree(app, ws).await;
            }
        }
        KeyCode::Left => {
            if on_tree_tab {
                app.tree_left(); // Collapse node
            } else {
                app.prev_tab();
                maybe_fetch_tree(app, ws).await;
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') if on_tree_tab => {
            app.tree_toggle();
        }
        // Agent-tab specific: Enter chat mode
        KeyCode::Enter if app.content_tab == ContentTab::Agent => {
            app.mode = Mode::AgentChat;
        }
        KeyCode::Char('s') => {
            app.mode = Mode::SessionPicker;
        }
        // Run app (play) - show device prompt (Flutter tab only)
        KeyCode::Char('p') => {
            if app.content_tab == ContentTab::Flutter && !app.is_app_running() && app.has_session() {
                app.mode = Mode::InputPrompt(InputPromptKind::RunApp);
                app.input_buffer.clear();
            }
        }
        // Stop app (Flutter tab only)
        KeyCode::Char('x') => {
            if app.content_tab == ContentTab::Flutter && app.is_app_running() {
                if let Some(client) = ws {
                    let msg = OutgoingMessage::Command {
                        id: Uuid::new_v4().to_string(),
                        client_id: client.client_id().to_string(),
                        action: "stop_app".to_string(),
                        key: None,
                        data: None,
                    };
                    let _ = client.send(msg).await;
                }
            }
        }
        KeyCode::Esc => {
            if app.in_visual_mode() {
                app.exit_visual_mode();
            } else {
                app.filter = None;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn maybe_fetch_tree(app: &mut App, ws: &Option<WsClient>) {
    if app.content_tab == ContentTab::Tree && app.session.tree.is_none() && app.is_app_running() {
        if let Some(client) = ws {
            let msg = OutgoingMessage::Command {
                id: Uuid::new_v4().to_string(),
                client_id: client.client_id().to_string(),
                action: "get_tree".to_string(),
                key: None,
                data: None,
            };
            let _ = client.send(msg).await;
        }
    }
}

// Command mode - currently unused, hotkey-driven UI instead
/*
async fn handle_command_mode(app: &mut App, key: KeyEvent, ws: &Option<WsClient>) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input_buffer.clear();
            app.completions.clear();
            app.completion_index = 0;
        }
        KeyCode::Tab => {
            if app.completions.is_empty() {
                app.update_completions();
            } else {
                app.cycle_completion_next();
            }
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if !app.completions.is_empty() {
                app.cycle_completion_next();
            }
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if !app.completions.is_empty() {
                app.cycle_completion_prev();
            }
        }
        KeyCode::Enter => {
            if !app.completions.is_empty() {
                app.accept_completion();
            } else {
                let input = app.input_buffer.clone();
                app.input_buffer.clear();
                app.completions.clear();
                app.completion_index = 0;
                app.mode = Mode::Normal;

                if !input.is_empty() {
                    execute_command(app, &input, ws).await?;
                }
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
            app.update_completions();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
            app.update_completions();
        }
        _ => {}
    }
    Ok(())
}
*/

fn handle_filter_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            if app.input_buffer.is_empty() {
                app.filter = None;
            } else {
                app.filter = Some(app.input_buffer.clone());
            }
            app.input_buffer.clear();
            app.mode = Mode::Normal;
            app.scroll_offset = 0;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}

fn handle_help_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

fn handle_confirm_mode(app: &mut App, key: KeyEvent, action: ConfirmAction) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => match action {
            ConfirmAction::Quit => {
                app.should_quit = true;
            }
        },
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

async fn handle_session_picker_mode(
    app: &mut App,
    key: KeyEvent,
    ws: &Option<WsClient>,
) -> Result<()> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.session_picker_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.session_picker_up();
        }
        KeyCode::Enter => {
            if let Some(session) = app.sessions.get(app.session_picker_index) {
                if let Some(client) = ws {
                    let msg = OutgoingMessage::Command {
                        id: Uuid::new_v4().to_string(),
                        client_id: client.client_id().to_string(),
                        action: "connect_session".to_string(),
                        key: None,
                        data: Some(serde_json::json!({ "sessionId": session.id })),
                    };
                    let _ = client.send(msg).await;
                }
            }
            app.session_picker_select();
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Normal;
        }
        // Create new session
        KeyCode::Char('c') => {
            app.input_buffer.clear();
            app.mode = Mode::InputPrompt(InputPromptKind::CreateSession);
        }
        // Delete selected session
        KeyCode::Char('d') | KeyCode::Char('x') => {
            if let Some(session) = app.sessions.get(app.session_picker_index) {
                if let Some(client) = ws {
                    let msg = OutgoingMessage::Command {
                        id: Uuid::new_v4().to_string(),
                        client_id: client.client_id().to_string(),
                        action: "destroy_session".to_string(),
                        key: None,
                        data: Some(serde_json::json!({ "sessionId": session.id })),
                    };
                    let _ = client.send(msg).await;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_input_prompt_mode(
    app: &mut App,
    key: KeyEvent,
    ws: &Option<WsClient>,
    kind: InputPromptKind,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.input_buffer.clear();
            // Return to appropriate mode
            match kind {
                InputPromptKind::CreateSession => app.mode = Mode::SessionPicker,
                InputPromptKind::RunApp => app.mode = Mode::Normal,
            }
        }
        KeyCode::Enter => {
            let input = app.input_buffer.clone();
            app.input_buffer.clear();

            match kind {
                InputPromptKind::CreateSession => {
                    if !input.is_empty() {
                        if let Some(client) = ws {
                            // Use project path from app.project
                            let project_path = app.project.path.to_string_lossy().to_string();
                            let msg = OutgoingMessage::Command {
                                id: Uuid::new_v4().to_string(),
                                client_id: client.client_id().to_string(),
                                action: "create_session".to_string(),
                                key: None,
                                data: Some(serde_json::json!({
                                    "name": input,
                                    "projectPath": project_path,
                                })),
                            };
                            let _ = client.send(msg).await;
                        }
                    }
                    app.mode = Mode::SessionPicker;
                }
                InputPromptKind::RunApp => {
                    if let Some(client) = ws {
                        if let Some(ref session_id) = app.selected_session {
                            // Device is optional - empty string means default device
                            let mut data = serde_json::json!({ "sessionId": session_id });
                            if !input.is_empty() {
                                data["device"] = serde_json::json!(input);
                            }
                            let msg = OutgoingMessage::Command {
                                id: Uuid::new_v4().to_string(),
                                client_id: client.client_id().to_string(),
                                action: "run_app".to_string(),
                                key: None,
                                data: Some(data),
                            };
                            let _ = client.send(msg).await;
                        } else {
                            app.push_toast(crate::app::Toast::error("No session selected"));
                        }
                    }
                    app.mode = Mode::Normal;
                }
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

async fn handle_action_menu_mode(
    app: &mut App,
    key: KeyEvent,
    ws: &Option<WsClient>,
) -> Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.action_menu_items.clear();
            app.action_menu_index = 0;
            app.action_menu_node_id = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.action_menu_items.is_empty() {
                app.action_menu_index = (app.action_menu_index + 1) % app.action_menu_items.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.action_menu_items.is_empty() {
                app.action_menu_index = app.action_menu_index.checked_sub(1)
                    .unwrap_or(app.action_menu_items.len() - 1);
            }
        }
        KeyCode::Enter => {
            if let Some((_, action)) = app.action_menu_items.get(app.action_menu_index).cloned() {
                if let Some(node_id) = app.action_menu_node_id.clone() {
                    match action {
                        InteractionAction::Tap => {
                            execute_interaction(app, ws, &node_id, "tap", None).await?;
                        }
                        InteractionAction::LongPress => {
                            execute_interaction(app, ws, &node_id, "longPress", None).await?;
                        }
                        InteractionAction::DoubleTap => {
                            execute_interaction(app, ws, &node_id, "doubleTap", None).await?;
                        }
                        InteractionAction::Scroll { dx, dy } => {
                            execute_interaction(
                                app,
                                ws,
                                &node_id,
                                "scroll",
                                Some(serde_json::json!({ "dx": dx, "dy": dy })),
                            )
                            .await?;
                        }
                        InteractionAction::EnterText(_) => {
                            app.push_toast(crate::app::Toast::info("Enter text not implemented yet"));
                        }
                        InteractionAction::Custom(action_name) => {
                            execute_interaction(app, ws, &action_name, &action_name, None).await?;
                        }
                    }
                }
                app.action_menu_items.clear();
                app.action_menu_index = 0;
                app.action_menu_node_id = None;
                app.mode = Mode::Normal;
            }
        }
        _ => {}
    }

    Ok(())
}

async fn handle_agent_chat_mode(
    app: &mut App,
    key: KeyEvent,
    ws: &Option<WsClient>,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if !app.session.chat_input.is_empty() {
                let text = std::mem::take(&mut app.session.chat_input);
                app.session.chat_cursor = 0;
                
                // Add user message
                app.session.chat_messages.push(crate::chat::ChatMessage::user(&text));
                
                // Send to daemon (daemon manages session/conversation internally)
                if let Some(client) = ws {
                    let msg = OutgoingMessage::Command {
                        id: Uuid::new_v4().to_string(),
                        client_id: client.client_id().to_string(),
                        action: "agent_message".to_string(),
                        key: None,
                        data: Some(serde_json::json!({ "intent": text })),
                    };
                    let _ = client.send(msg).await;
                    app.session.pending_response = true;
                    app.session.chat_streaming = Some(crate::chat::StreamingState::default());
                }
                // Stay in AgentChat mode for follow-up messages
            }
        }
        KeyCode::Char(c) => {
            app.session.chat_input.insert(app.session.chat_cursor, c);
            app.session.chat_cursor += 1;
        }
        KeyCode::Backspace => {
            if app.session.chat_cursor > 0 {
                app.session.chat_cursor -= 1;
                app.session.chat_input.remove(app.session.chat_cursor);
            }
        }
        KeyCode::Delete => {
            if app.session.chat_cursor < app.session.chat_input.len() {
                app.session.chat_input.remove(app.session.chat_cursor);
            }
        }
        KeyCode::Left => {
            app.session.chat_cursor = app.session.chat_cursor.saturating_sub(1);
        }
        KeyCode::Right => {
            app.session.chat_cursor = (app.session.chat_cursor + 1).min(app.session.chat_input.len());
        }
        KeyCode::Home => {
            app.session.chat_cursor = 0;
        }
        KeyCode::End => {
            app.session.chat_cursor = app.session.chat_input.len();
        }
        _ => {}
    }
    Ok(())
}

async fn execute_interaction(
    app: &mut App,
    ws: &Option<WsClient>,
    node_id: &str,
    interaction: &str,
    args: Option<serde_json::Value>,
) -> Result<()> {
    if let Some(client) = ws {
        let mut data = serde_json::json!({
            "nodeId": node_id,
            "interaction": interaction,
        });
        if let Some(args) = args {
            data["args"] = args;
        }
        let msg = OutgoingMessage::Command {
            id: Uuid::new_v4().to_string(),
            client_id: client.client_id().to_string(),
            action: "execute_interaction".to_string(),
            key: None,
            data: Some(data),
        };
        let _ = client.send(msg).await;
        app.push_toast(crate::app::Toast::info(format!("{} on {}", interaction, node_id)));
    }
    Ok(())
}

#[allow(dead_code)]
fn current_event_count(app: &App) -> usize {
    use crate::app::ContentTab;
    match app.content_tab {
        ContentTab::Flutter => app.filtered_flutter_logs().count(),
        ContentTab::Agent => app.filtered_agent_events().count(),
        ContentTab::Interactions => app.filtered_interaction_logs().count(),
        ContentTab::Tree => 0, // Tree uses tree_state, not scroll_offset
    }
}

// Keep execute_command for potential future use, but it's not used in hotkey-driven UI
#[allow(dead_code)]
async fn execute_command(app: &mut App, input: &str, ws: &Option<WsClient>) -> Result<()> {
    let command = parse_command(input);

    match command {
        TuiCommand::Quit => {
            app.should_quit = true;
        }
        TuiCommand::Reload => {
            if let Some(client) = ws {
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "hot_reload".to_string(),
                    key: None,
                    data: None,
                };
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::Restart => {
            if let Some(client) = ws {
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "hot_restart".to_string(),
                    key: None,
                    data: None,
                };
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::Run { device } => {
            if let Some(client) = ws {
                let data = device.map(|d| serde_json::json!({ "device": d }));
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "run_app".to_string(),
                    key: None,
                    data,
                };
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::Stop => {
            if let Some(client) = ws {
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "stop_app".to_string(),
                    key: None,
                    data: None,
                };
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::Status => {
            if let Some(client) = ws {
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "get_status".to_string(),
                    key: None,
                    data: None,
                };
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::Tree => {
            if let Some(client) = ws {
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "get_tree".to_string(),
                    key: None,
                    data: None,
                };
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::CreateSession { name, project_path } => {
            if let Some(client) = ws {
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "create_session".to_string(),
                    key: None,
                    data: Some(serde_json::json!({
                        "name": name,
                        "projectPath": project_path,
                    })),
                };
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::ListSessions => {
            if let Some(client) = ws {
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "list_sessions".to_string(),
                    key: None,
                    data: None,
                };
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::Connect { session } => {
            if let Some(client) = ws {
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "connect_session".to_string(),
                    key: None,
                    data: Some(serde_json::json!({ "sessionId": session })),
                };
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::DestroySession { session } => {
            if let Some(client) = ws {
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "destroy_session".to_string(),
                    key: None,
                    data: Some(serde_json::json!({ "sessionId": session })),
                };
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::Filter(pattern) => {
            app.filter = pattern;
            app.scroll_offset = 0;
        }
        TuiCommand::Clear => {
            app.clear_events();
        }
        TuiCommand::Agent { intent, answer } => {
            if let Some(client) = ws {
                let mut data = serde_json::json!({ "intent": intent });
                if let Some(ans) = answer {
                    data["answer"] = serde_json::Value::String(ans);
                }
                let msg = OutgoingMessage::Command {
                    id: Uuid::new_v4().to_string(),
                    client_id: client.client_id().to_string(),
                    action: "agent_message".to_string(),
                    key: None,
                    data: Some(data),
                };
                app.session.pending_response = true;
                let _ = client.send(msg).await;
            }
        }
        TuiCommand::Help => {
            app.mode = Mode::Help;
        }
        TuiCommand::Unknown(cmd) => {
            tracing::warn!("Unknown command: {}", cmd);
        }
    }

    Ok(())
}

fn handle_ws_event(app: &mut App, event: WsEvent) {
    match event {
        WsEvent::Connected {
            client_id: _,
            daemon_version,
            sessions,
        } => {
            app.ws_state = WsState::Connected;
            tracing::info!("Connected to daemon v{}", daemon_version);
            // Convert SessionSummary to Session for the app
            let sessions: Vec<Session> = sessions
                .into_iter()
                .map(|s| session_from_summary(s))
                .collect();
            app.set_sessions(sessions);
        }
        WsEvent::Disconnected => {
            app.ws_state = WsState::Disconnected;
            app.push_toast(crate::app::Toast::error("Disconnected from server"));
        }
        WsEvent::Error(e) => {
            tracing::error!("WebSocket error: {}", e);
            app.push_toast(crate::app::Toast::error(format!("WebSocket: {}", e)));
        }
        WsEvent::Message(msg) => match msg {
            IncomingMessage::CommandResponse(resp) => {
                app.handle_command_response(resp);
            }
            IncomingMessage::AgentResponse(resp) => {
                app.handle_agent_response(resp);
            }
            IncomingMessage::Event(event) => {
                app.push_event(event);
            }
            IncomingMessage::AgentStream(stream_event) => {
                app.handle_agent_stream_event(stream_event);
            }
        },
    }
}

fn session_from_summary(s: SessionSummary) -> Session {
    Session {
        id: s.id,
        name: s.name,
        project_path: s.project_path,
        app_status: s.app_status,
        vm_service_uri: None,
        pid: None,
        connected_clients: Vec::new(),
        created_at: String::new(),
        last_active_at: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::project::ProjectInfo;

    fn test_project() -> ProjectInfo {
        ProjectInfo {
            path: PathBuf::from("/test/project"),
            name: "test_app".to_string(),
            is_flutter: true,
        }
    }

    #[test]
    fn test_handle_filter_mode_enter_applies_filter() {
        let mut app = App::new("ws://localhost:9000".to_string(), 100, test_project());
        app.mode = Mode::Filter;
        app.input_buffer = "flutter".to_string();

        handle_filter_mode(&mut app, KeyEvent::from(KeyCode::Enter));

        assert_eq!(app.filter, Some("flutter".to_string()));
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.input_buffer.is_empty());
    }

    #[test]
    fn test_handle_filter_mode_empty_clears_filter() {
        let mut app = App::new("ws://localhost:9000".to_string(), 100, test_project());
        app.mode = Mode::Filter;
        app.filter = Some("old".to_string());
        app.input_buffer.clear();

        handle_filter_mode(&mut app, KeyEvent::from(KeyCode::Enter));

        assert!(app.filter.is_none());
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_handle_confirm_mode_yes_quits() {
        let mut app = App::new("ws://localhost:9000".to_string(), 100, test_project());
        app.mode = Mode::Confirm(ConfirmAction::Quit);

        handle_confirm_mode(&mut app, KeyEvent::from(KeyCode::Char('y')), ConfirmAction::Quit);

        assert!(app.should_quit);
    }

    #[test]
    fn test_handle_confirm_mode_no_cancels() {
        let mut app = App::new("ws://localhost:9000".to_string(), 100, test_project());
        app.mode = Mode::Confirm(ConfirmAction::Quit);

        handle_confirm_mode(&mut app, KeyEvent::from(KeyCode::Char('n')), ConfirmAction::Quit);

        assert!(!app.should_quit);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_handle_help_mode_escape_returns_to_normal() {
        let mut app = App::new("ws://localhost:9000".to_string(), 100, test_project());
        app.mode = Mode::Help;

        handle_help_mode(&mut app, KeyEvent::from(KeyCode::Esc));

        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_handle_ws_event_connected() {
        let mut app = App::new("ws://localhost:9000".to_string(), 100, test_project());
        app.ws_state = WsState::Connecting;

        handle_ws_event(
            &mut app,
            WsEvent::Connected {
                client_id: "test-client-123".to_string(),
                daemon_version: "0.1.0".to_string(),
                sessions: vec![],
            },
        );

        assert_eq!(app.ws_state, WsState::Connected);
    }

    #[test]
    fn test_handle_ws_event_disconnected() {
        let mut app = App::new("ws://localhost:9000".to_string(), 100, test_project());
        app.ws_state = WsState::Connected;

        handle_ws_event(&mut app, WsEvent::Disconnected);

        assert_eq!(app.ws_state, WsState::Disconnected);
    }
}
