use std::collections::{HashMap, VecDeque};

use tui_tree_widget::TreeState;

use crate::flutter_log::FlutterLogEntry;
use crate::project::ProjectInfo;
use crate::tree_format::CompactTree;
use crate::ws::protocol::{AgentResponse, AgentStatus, CommandResponse, MonitoringEvent, Session};

// Re-export types for backward compatibility with existing code
pub use crate::session::SessionState;
pub use crate::types::{
    Action, AppStatus, Capability, ConfirmAction, ContentTab, ContextInfo, InputPromptKind,
    InteractionAction, InteractionTree, LogEntry, LogLevel, LogViewMode, LogViewState, Mode,
    Toast, ToastLevel, TreeNode, WsState,
};

pub struct App {
    pub ws_state: WsState,
    pub server_uri: String,

    pub project: ProjectInfo,
    pub sessions: Vec<Session>,
    pub selected_session: Option<String>,
    pub session_picker_index: usize,

    pub app_status: AppStatus,

    /// Current session state (session-specific data)
    pub session: SessionState,
    /// Session states stored by session ID (preserved across session switches)
    pub session_states: HashMap<String, SessionState>,
    pub max_events: usize,

    pub mode: Mode,
    pub input_buffer: String,
    pub filter: Option<String>,
    pub content_tab: ContentTab,
    pub scroll_offset: usize,
    
    /// Log viewer state (cursor, selection, viewport)
    pub log_view: LogViewState,
    /// Cached viewport height for log navigation
    pub log_viewport_height: usize,

    pub needs_tree_fetch: bool,
    #[allow(dead_code)]
    pub pending_session_connect: Option<String>,

    pub throbber_state: throbber_widgets_tui::ThrobberState,

    // Completions - currently unused, hotkey-driven UI instead
    // pub completions: Vec<&'static str>,
    // pub completion_index: usize,
    // pub completion_start_col: usize,

    /// Action menu items (label, action)
    pub action_menu_items: Vec<(String, InteractionAction)>,
    /// Currently selected index in action menu
    pub action_menu_index: usize,
    /// Currently selected node ID for action menu
    pub action_menu_node_id: Option<String>,

    pub toasts: VecDeque<Toast>,
    pub toast_ttl_secs: u64,

    pub should_quit: bool,
}

impl App {
    pub fn new(uri: String, max_events: usize, project: ProjectInfo) -> Self {
        Self {
            ws_state: WsState::Disconnected,
            server_uri: uri,

            project,
            sessions: Vec::new(),
            selected_session: None,
            session_picker_index: 0,

            app_status: AppStatus::Unknown,

            session: SessionState::new(max_events),
            session_states: HashMap::new(),
            max_events,

            mode: Mode::SessionPicker,
            input_buffer: String::new(),
            filter: None,
            content_tab: ContentTab::default(),
            scroll_offset: 0,
            
            log_view: LogViewState::new(),
            log_viewport_height: 0,

            needs_tree_fetch: false,
            pending_session_connect: None,

            throbber_state: throbber_widgets_tui::ThrobberState::default(),

            action_menu_items: Vec::new(),
            action_menu_index: 0,
            action_menu_node_id: None,

            toasts: VecDeque::new(),
            toast_ttl_secs: 5,

            should_quit: false,
        }
    }

    pub fn push_toast(&mut self, toast: Toast) {
        self.toasts.push_back(toast);
        // Keep max 5 toasts
        while self.toasts.len() > 5 {
            self.toasts.pop_front();
        }
    }

    pub fn expire_toasts(&mut self) {
        self.toasts.retain(|t| !t.is_expired(self.toast_ttl_secs));
    }

    /// Switch to a different session, saving current state and restoring new session's state
    pub fn switch_to_session(&mut self, new_session_id: String) {
        // Save current session's state
        if let Some(old_id) = self.selected_session.take() {
            let old_session = std::mem::replace(&mut self.session, SessionState::new(self.max_events));
            self.session_states.insert(old_id, old_session);
        }
        
        // Restore new session's state (or create new if none)
        self.session = self.session_states
            .remove(&new_session_id)
            .unwrap_or_else(|| SessionState::new(self.max_events));
        
        // Set the new session
        self.selected_session = Some(new_session_id);
        
        // Always fetch tree when switching sessions (it may be stale or missing)
        self.needs_tree_fetch = true;
    }

    // Completion methods - currently unused, hotkey-driven UI instead
    /*
    pub fn update_completions(&mut self) {
        let input = self.input_buffer.to_lowercase();
        if input.is_empty() {
            self.completions.clear();
            self.completion_index = 0;
            return;
        }
        let was_empty = self.completions.is_empty();
        self.completions = COMMANDS
            .iter()
            .copied()
            .filter(|cmd| cmd.starts_with(&input) && *cmd != input)
            .collect();
        self.completion_index = 0;
        if was_empty && !self.completions.is_empty() {
            self.completion_start_col = self.input_buffer.len();
        }
    }

    pub fn cycle_completion_next(&mut self) {
        if self.completions.is_empty() {
            return;
        }
        self.completion_index = (self.completion_index + 1) % self.completions.len();
    }

    pub fn cycle_completion_prev(&mut self) {
        if self.completions.is_empty() {
            return;
        }
        if self.completion_index == 0 {
            self.completion_index = self.completions.len() - 1;
        } else {
            self.completion_index -= 1;
        }
    }

    pub fn accept_completion(&mut self) {
        if let Some(cmd) = self.completions.get(self.completion_index) {
            self.input_buffer = cmd.to_string();
            self.completions.clear();
            self.completion_index = 0;
        }
    }

    pub fn current_completion(&self) -> Option<&'static str> {
        self.completions.get(self.completion_index).copied()
    }
    */

    pub fn set_tree(&mut self, tree: InteractionTree) {
        self.session.tree = Some(tree);
        self.session.tree_state = TreeState::default();
        // Expand all nodes and select first one
        self.tree_expand_all();
        self.select_first_tree_node();
    }
    
    /// Select the first node in the tree
    fn select_first_tree_node(&mut self) {
        if let Some(compact) = self.compact_tree() {
            if let Some(first) = compact.tree.first() {
                let id = match first {
                    crate::tree_format::TreeEntry::Context { .. } => "ctx_0".to_string(),
                    crate::tree_format::TreeEntry::Homogeneous { id, .. } => format!("{}_0", id),
                    crate::tree_format::TreeEntry::Singleton { id, .. } => format!("{}_0", id),
                    crate::tree_format::TreeEntry::Variants { id, .. } => format!("{}_0", id),
                    crate::tree_format::TreeEntry::Heterogeneous { .. } => "het_0".to_string(),
                };
                self.session.tree_state.select(vec![id]);
            }
        }
    }

    pub fn compact_tree(&self) -> Option<CompactTree> {
        self.session.tree.as_ref().map(|t| {
            let (compact, _warnings) = CompactTree::from_tree_nodes(&t.nodes);
            compact
        })
    }

    pub fn push_interaction_log(&mut self, entry: LogEntry) {
        if self.session.interaction_logs.len() >= self.max_events {
            self.session.interaction_logs.pop_front();
        }
        self.session.interaction_logs.push_back(entry);
    }

    pub fn push_flutter_log(&mut self, entry: FlutterLogEntry) {
        if self.session.flutter_logs.len() >= self.max_events {
            self.session.flutter_logs.pop_front();
        }
        self.session.flutter_logs.push_back(entry);
    }

    pub fn push_agent_event(&mut self, event: MonitoringEvent) {
        if self.session.agent_events.len() >= self.max_events {
            self.session.agent_events.pop_front();
        }
        self.session.agent_events.push_back(event);
    }

    pub fn push_event(&mut self, event: MonitoringEvent) {
        let source = event.source.to_lowercase();
        let event_type = event.event_type.to_lowercase();

        // Handle session status changes to update app_status
        if event_type == "session.status_changed" {
            self.handle_session_status_event(&event.payload);
            return;
        }

        // Handle session destroyed - remove from local list
        if event_type == "session.destroyed" {
            if let Some(session_id) = event.payload.get("sessionId").and_then(|s| s.as_str()) {
                self.sessions.retain(|s| s.id != session_id);
                // Adjust picker index if needed
                if self.session_picker_index >= self.sessions.len() && !self.sessions.is_empty() {
                    self.session_picker_index = self.sessions.len() - 1;
                }
                // Clear selected session if it was destroyed
                if self.selected_session.as_deref() == Some(session_id) {
                    self.selected_session = None;
                    self.app_status = AppStatus::Stopped;
                }
            }
            return;
        }

        // Handle session created - add to local list
        if event_type == "session.created" {
            if let Some(session_data) = event.payload.get("session") {
                if let Ok(session) = serde_json::from_value::<Session>(session_data.clone()) {
                    // Only add if not already present
                    if !self.sessions.iter().any(|s| s.id == session.id) {
                        self.sessions.push(session);
                    }
                }
            }
            return;
        }

        // Handle tree updates
        if event_type == "tree.updated" {
            if let Some(tree_data) = event.payload.get("tree").and_then(|t| t.as_array()) {
                let nodes = Self::parse_tree_nodes(tree_data);
                let tree = InteractionTree {
                    nodes,
                    last_updated: Some(chrono::Utc::now().to_rfc3339()),
                };
                self.set_tree(tree);
            }
            return;
        }

        if source.contains("agent") || event_type.starts_with("agent_") || event_type.contains("tool") {
            self.push_agent_event(event);
        } else if source.contains("flutter") || event_type.starts_with("flutter.") {
            // Flutter logs go to Flutter tab
            let line = extract_flutter_log(&event.payload);
            let entry = FlutterLogEntry::parse(&line);
            self.push_flutter_log(entry);
        } else if event_type.contains("interaction") || event_type.contains("tap") || event_type.contains("scroll") {
            // Interaction events (execute_interaction, taps, scrolls)
            let entry = LogEntry {
                ts: event.ts.clone(),
                level: LogLevel::Info,
                message: format!("[{}] {}", event.source, summarize_payload(&event.payload)),
            };
            self.push_interaction_log(entry);
            
            // If interaction came from MCP (Amp), also show in Agent chat
            if let Some(client_type) = event.payload.get("clientType").and_then(|c| c.as_str()) {
                if client_type == "mcp" {
                    let node_id = event.payload.get("nodeId").and_then(|n| n.as_str()).unwrap_or("unknown");
                    let interaction = event.payload.get("interaction").and_then(|i| i.as_str()).unwrap_or("unknown");
                    let action_msg = format!("⚡ {} on {}", interaction, node_id);
                    let mut msg = crate::chat::ChatMessage::user(&action_msg);
                    msg.client_id = Some("mcp".to_string());
                    self.session.chat_messages.push(msg);
                    self.session.chat_scroll = 0;
                }
            }
        }
    }

    fn handle_session_status_event(&mut self, payload: &serde_json::Value) {
        // Check if this event is for our selected session
        if let Some(session_id) = payload.get("sessionId").and_then(|s| s.as_str()) {
            if self.selected_session.as_deref() != Some(session_id) {
                return;
            }
        }

        if let Some(status) = payload.get("status").and_then(|s| s.as_str()) {
            match status {
                "starting" => self.app_status = AppStatus::Starting,
                "running" => {
                    let pid = payload.get("pid").and_then(|p| p.as_u64()).unwrap_or(0) as u32;
                    let uri = payload
                        .get("vmServiceUri")
                        .and_then(|u| u.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    // Check if VM was connected and now disconnected (uri became empty)
                    let was_connected = matches!(&self.app_status, AppStatus::Running { uri, .. } if !uri.is_empty());
                    let now_disconnected = uri.is_empty();
                    
                    self.app_status = AppStatus::Running { pid, uri: uri.clone() };
                    
                    if was_connected && now_disconnected {
                        // VM disconnected (e.g., hot restart) - clear tree
                        self.session.tree = None;
                    } else if !uri.is_empty() && self.session.tree.is_none() {
                        // VM connected and no tree - fetch it
                        self.needs_tree_fetch = true;
                    }
                }
                "stopped" | "not_running" => self.app_status = AppStatus::Stopped,
                "error" => {
                    let error = payload
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("Unknown error")
                        .to_string();
                    self.app_status = AppStatus::Error(error);
                }
                _ => {}
            }
        }
    }

    pub fn filtered_interaction_logs(&self) -> impl Iterator<Item = &LogEntry> {
        let filter = self.filter.clone();
        self.session.interaction_logs.iter().filter(move |e| {
            let Some(ref pattern) = filter else {
                return true;
            };
            let pattern_lower = pattern.to_lowercase();
            e.message.to_lowercase().contains(&pattern_lower)
        })
    }

    pub fn filtered_flutter_logs(&self) -> impl Iterator<Item = &FlutterLogEntry> {
        let filter = self.filter.clone();
        self.session.flutter_logs.iter().filter(move |e| {
            if e.is_noise() {
                return false;
            }
            let Some(ref pattern) = filter else {
                return true;
            };
            let pattern_lower = pattern.to_lowercase();
            e.raw.to_lowercase().contains(&pattern_lower)
        })
    }

    pub fn filtered_agent_events(&self) -> impl Iterator<Item = &MonitoringEvent> {
        let filter = self.filter.clone();
        self.session.agent_events.iter().filter(move |e| {
            let Some(ref pattern) = filter else {
                return true;
            };
            let pattern_lower = pattern.to_lowercase();
            e.source.to_lowercase().contains(&pattern_lower)
                || e.event_type.to_lowercase().contains(&pattern_lower)
        })
    }

    pub fn handle_command_response(&mut self, resp: CommandResponse) {
        if resp.success {
            // Check if this is a tree response (from get_tree)
            if let Some(targets) = resp.data.get("targets").and_then(|t| t.as_array()) {
                let nodes = Self::parse_tree_nodes(targets);
                let tree = InteractionTree {
                    nodes,
                    last_updated: Some(chrono::Utc::now().to_rfc3339()),
                };
                self.set_tree(tree);
                return;
            }

            // Check if response contains updated tree (from execute_interaction, batch)
            if let Some(tree_data) = resp.data.get("tree").and_then(|t| t.as_array()) {
                let nodes = Self::parse_tree_nodes(tree_data);
                let tree = InteractionTree {
                    nodes,
                    last_updated: Some(chrono::Utc::now().to_rfc3339()),
                };
                self.set_tree(tree);
                // Don't return - continue processing other fields
            }

            // Check if this is a sessions list response
            if let Some(sessions) = resp.data.get("sessions").and_then(|s| s.as_array()) {
                if let Ok(parsed) = serde_json::from_value::<Vec<Session>>(serde_json::Value::Array(sessions.clone())) {
                    self.sessions = parsed;
                    // If we have no selected session and there are sessions, select first
                    if self.selected_session.is_none() && !self.sessions.is_empty() {
                        self.session_picker_index = 0;
                    }
                }
                return;
            }

            // Check if this is a session creation/connect response
            if let Some(session) = resp.data.get("session") {
                match serde_json::from_value::<Session>(session.clone()) {
                    Ok(parsed) => {
                        let session_id = parsed.id.clone();
                        // Add to sessions list if not already there
                        if !self.sessions.iter().any(|s| s.id == session_id) {
                            self.sessions.push(parsed);
                        }
                        // Auto-select the new session
                        self.switch_to_session(session_id);
                        self.session_picker_index = self.sessions.len().saturating_sub(1);
                        self.mode = Mode::Normal;
                        
                        // Load chat history if present (from connect_session response)
                        if let Some(chat_history) = resp.data.get("chatHistory") {
                            let messages = crate::chat::parse_chat_history(chat_history);
                            if !messages.is_empty() {
                                tracing::info!(count = messages.len(), "Loaded chat history from daemon");
                                self.session.chat_messages = messages;
                            }
                        } else {
                            self.push_toast(Toast::success("Session created"));
                        }
                    }
                    Err(e) => {
                        tracing::error!(?e, ?session, "Failed to parse session response");
                        self.push_toast(Toast::error(&format!("Parse error: {}", e)));
                    }
                }
                return;
            }

            // Check for app status from response
            if let Some(status) = resp.data.get("status").and_then(|s| s.as_str()) {
                match status {
                    "starting" => self.app_status = AppStatus::Starting,
                    "running" => {
                        let pid = resp.data.get("pid").and_then(|p| p.as_u64()).unwrap_or(0) as u32;
                        let uri = resp
                            .data
                            .get("uri")
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .to_string();
                        self.app_status = AppStatus::Running { pid, uri };
                        // Auto-fetch tree when app starts
                        if self.session.tree.is_none() {
                            self.needs_tree_fetch = true;
                        }
                    }
                    "stopped" | "not_running" => self.app_status = AppStatus::Stopped,
                    _ => {}
                }
            } else if resp.data.get("pid").is_some() {
                // run_app response returns just { pid } - treat as starting
                self.app_status = AppStatus::Starting;
            }
        } else if let Some(err) = resp.error {
            self.push_toast(Toast::error(&err));
        }
    }

    fn parse_tree_nodes(targets: &[serde_json::Value]) -> Vec<TreeNode> {
        targets
            .iter()
            .filter_map(|t| {
                let id = t.get("id")?.as_str()?.to_string();
                let widget_type = t.get("widgetType").and_then(|w| w.as_str()).map(String::from);
                let children = t
                    .get("children")
                    .and_then(|c| c.as_array())
                    .map(|arr| Self::parse_tree_nodes(arr))
                    .unwrap_or_default();
                let contexts = t
                    .get("contexts")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|ctx| {
                                let name = ctx.get("name")?.as_str()?.to_string();
                                let description = ctx.get("description").and_then(|d| d.as_str()).map(String::from);
                                Some(ContextInfo { name, description })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let capabilities = t
                    .get("capabilities")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|cap| {
                                let name = cap.get("name")?.as_str()?.to_string();
                                Some(Capability { name })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let actions = t
                    .get("actions")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|act| {
                                let name = act.get("name")?.as_str()?.to_string();
                                let description = act.get("description").and_then(|d| d.as_str()).map(String::from);
                                Some(Action { name, description })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Some(TreeNode {
                    id,
                    widget_type,
                    children,
                    contexts,
                    capabilities,
                    actions,
                })
            })
            .collect()
    }

    pub fn handle_agent_response(&mut self, resp: AgentResponse) {
        self.session.pending_response = false;

        let event = resp.to_monitoring_event();
        self.push_event(event);

        match resp.status {
            AgentStatus::Success => {
                self.session.conversation_id = None;
                self.session.agent_question = None;
                self.session.last_agent_error = None;
                // Refresh tree after agent completes (it may have changed the UI)
                self.needs_tree_fetch = true;
            }
            AgentStatus::NeedsContext => {
                self.session.conversation_id = resp.sdk_session_id;
                self.session.agent_question = resp.question;
                self.session.last_agent_error = None;
            }
            AgentStatus::Error => {
                self.session.conversation_id = None;
                self.session.agent_question = None;
                if let Some(ref err) = resp.summary {
                    self.push_toast(Toast::error(err));
                }
                self.session.last_agent_error = resp.summary;
            }
        }
    }

    #[allow(dead_code)]
    pub fn in_answer_mode(&self) -> bool {
        self.session.conversation_id.is_some()
    }

    pub fn is_app_running(&self) -> bool {
        matches!(self.app_status, AppStatus::Running { .. } | AppStatus::Starting)
    }

    pub fn has_session(&self) -> bool {
        self.selected_session.is_some()
    }

    /// Get the currently selected session from the sessions list
    pub fn current_session(&self) -> Option<&Session> {
        self.selected_session.as_ref().and_then(|id| {
            self.sessions.iter().find(|s| &s.id == id)
        })
    }

    /// Handle streaming agent events
    pub fn handle_agent_stream_event(&mut self, event: crate::ws::protocol::AgentStreamEvent) {
        use crate::ws::protocol::AgentEventKind;

        match event.event {
            AgentEventKind::TextDelta { text } => {
                // Append text to streaming buffer
                if let Some(streaming) = &mut self.session.chat_streaming {
                    streaming.text_buffer.push_str(&text);
                }
            }
            AgentEventKind::ToolCallStart { tool_name, tool_call_id } => {
                // Add a tool call message
                let tool_call = crate::chat::ToolCall {
                    id: tool_call_id,
                    name: tool_name,
                    args: serde_json::Value::Null,
                    status: crate::chat::ToolStatus::Running,
                    output: None,
                };
                self.session.chat_messages.push(crate::chat::ChatMessage::assistant_tool(tool_call));
            }
            AgentEventKind::ToolCallEnd { tool_name: _, tool_call_id, result } => {
                // Update the tool call message with result
                for msg in self.session.chat_messages.iter_mut().rev() {
                    if let crate::chat::ChatContent::ToolCall(call) = &mut msg.content {
                        if call.id == tool_call_id {
                            call.status = crate::chat::ToolStatus::Success;
                            call.output = result;
                            break;
                        }
                    }
                }
            }
            AgentEventKind::MessageComplete => {
                // Message streaming finished - finalize immediately for responsive UI
                if let Some(streaming) = self.session.chat_streaming.take() {
                    if !streaming.text_buffer.is_empty() {
                        self.session.chat_messages.push(crate::chat::ChatMessage::assistant(&streaming.text_buffer));
                    }
                }
            }
            AgentEventKind::TaskComplete { summary: _ } => {
                // Full task complete (SDK finished) - mark response done
                self.session.pending_response = false;
            }
            AgentEventKind::Error { message } => {
                self.session.chat_streaming = None;
                self.session.pending_response = false;
                self.session.last_agent_error = Some(message.clone());
                self.push_toast(Toast::error(&message));
            }
            AgentEventKind::UserMessage { text, client_id } => {
                // User message from another client (e.g., MCP/Amp)
                // Only add if it's not from us (TUI)
                if client_id != "tui" {
                    let mut msg = crate::chat::ChatMessage::user(&text);
                    msg.client_id = Some(client_id);
                    self.session.chat_messages.push(msg);
                    self.session.chat_scroll = 0; // Auto-scroll to bottom
                    // Start streaming state for the response
                    self.session.pending_response = true;
                    self.session.chat_streaming = Some(crate::chat::StreamingState::default());
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn cancel_answer_mode(&mut self) {
        self.session.conversation_id = None;
        self.session.agent_question = None;
    }

    /// Get the count of log lines for the current tab
    pub fn current_log_count(&self) -> usize {
        match self.content_tab {
            ContentTab::Flutter => self.filtered_flutter_logs().count(),
            ContentTab::Agent => self.filtered_agent_events().count(),
            ContentTab::Interactions => self.filtered_interaction_logs().count(),
            ContentTab::Tree => 0,
        }
    }

    pub fn scroll_up(&mut self) {
        let count = self.current_log_count();
        self.log_view.clamp_cursor(count);
        self.log_view.cursor_up(self.log_viewport_height);
        // Keep scroll_offset in sync for backward compat
        self.scroll_offset = self.log_view.scroll;
    }

    pub fn scroll_down(&mut self) {
        let count = self.current_log_count();
        if count == 0 {
            return;
        }
        self.log_view.clamp_cursor(count);
        self.log_view.cursor_down(count, self.log_viewport_height);
        // Keep scroll_offset in sync for backward compat
        self.scroll_offset = self.log_view.scroll;
    }

    pub fn scroll_to_top(&mut self) {
        self.log_view.cursor_top();
        self.scroll_offset = self.log_view.scroll;
    }

    pub fn scroll_to_bottom(&mut self) {
        let count = self.current_log_count();
        self.log_view.cursor_bottom(count, self.log_viewport_height);
        self.scroll_offset = self.log_view.scroll;
    }

    pub fn chat_scroll_up(&mut self) {
        self.session.chat_scroll = self.session.chat_scroll.saturating_add(1);
    }

    pub fn chat_scroll_down(&mut self) {
        self.session.chat_scroll = self.session.chat_scroll.saturating_sub(1);
    }

    pub fn chat_scroll_to_top(&mut self) {
        self.session.chat_scroll = usize::MAX;
    }

    pub fn chat_scroll_to_bottom(&mut self) {
        self.session.chat_scroll = 0;
    }

    /// Toggle visual line selection mode
    pub fn toggle_visual_mode(&mut self) {
        self.log_view.toggle_visual();
    }

    /// Exit visual mode without yanking
    pub fn exit_visual_mode(&mut self) {
        self.log_view.exit_visual();
    }

    /// Check if in visual mode
    pub fn in_visual_mode(&self) -> bool {
        self.log_view.mode == LogViewMode::Visual
    }

    /// Yank selected lines (or current line if not in visual mode) to clipboard
    /// Returns the yanked text
    pub fn yank_logs(&mut self) -> Option<String> {
        let (start, end) = if self.log_view.mode == LogViewMode::Visual {
            self.log_view.selection_range()?
        } else {
            // Yank current line only
            (self.log_view.cursor, self.log_view.cursor)
        };

        let text = match self.content_tab {
            ContentTab::Flutter => {
                let logs: Vec<_> = self.filtered_flutter_logs().collect();
                logs.iter()
                    .skip(start)
                    .take(end - start + 1)
                    .map(|e| e.to_plain_text())
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            ContentTab::Agent => {
                let logs: Vec<_> = self.filtered_agent_events().collect();
                logs.iter()
                    .skip(start)
                    .take(end - start + 1)
                    .map(|e| format!("[{}] {}: {}", e.ts, e.event_type, e.payload))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            ContentTab::Interactions => {
                let logs: Vec<_> = self.filtered_interaction_logs().collect();
                logs.iter()
                    .skip(start)
                    .take(end - start + 1)
                    .map(|e| format!("[{}] {}", e.ts, e.message))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            ContentTab::Tree => return None,
        };

        // Exit visual mode after yanking
        self.log_view.exit_visual();

        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// Reset log view state (when switching tabs)
    #[allow(dead_code)]
    pub fn reset_log_view(&mut self) {
        self.log_view = LogViewState::new();
        self.scroll_offset = 0;
    }

    pub fn tree_up(&mut self) {
        self.session.tree_state.key_up();
    }

    pub fn tree_down(&mut self) {
        self.session.tree_state.key_down();
    }

    pub fn tree_toggle(&mut self) {
        self.session.tree_state.toggle_selected();
    }

    pub fn tree_left(&mut self) {
        self.session.tree_state.key_left();
    }

    pub fn tree_right(&mut self) {
        self.session.tree_state.key_right();
    }

    #[allow(dead_code)]
    pub fn tree_selected(&self) -> Option<&String> {
        self.session.tree_state.selected().last()
    }

    /// Expand all tree nodes recursively
    pub fn tree_expand_all(&mut self) {
        if let Some(compact) = self.compact_tree() {
            self.collect_and_open_all(&compact.tree, vec![]);
        }
    }

    /// Recursively collect all tree identifiers and open them.
    /// Identifiers match the format used in tree_pane.rs: ctx_N, id_N, het_N, var_N
    fn collect_and_open_all(&mut self, entries: &[crate::tree_format::TreeEntry], path: Vec<String>) {
        use crate::tree_format::TreeEntry;
        for (sibling_index, entry) in entries.iter().enumerate() {
            match entry {
                TreeEntry::Context { children, .. } => {
                    let id = format!("ctx_{}", sibling_index);
                    let mut new_path = path.clone();
                    new_path.push(id);
                    self.session.tree_state.open(new_path.clone());
                    self.collect_and_open_all(children, new_path);
                }
                TreeEntry::Homogeneous { id, children, .. } => {
                    let tree_id = format!("{}_{}", id, sibling_index);
                    let mut new_path = path.clone();
                    new_path.push(tree_id);
                    if !children.is_empty() {
                        self.session.tree_state.open(new_path.clone());
                        self.collect_and_open_all(children, new_path);
                    }
                }
                TreeEntry::Heterogeneous { items } => {
                    let id = format!("het_{}", sibling_index);
                    let mut new_path = path.clone();
                    new_path.push(id);
                    self.session.tree_state.open(new_path.clone());
                    self.collect_and_open_all(items, new_path);
                }
                TreeEntry::Variants { id, variants } => {
                    let tree_id = format!("{}_{}", id, sibling_index);
                    let mut new_path = path.clone();
                    new_path.push(tree_id);
                    self.session.tree_state.open(new_path.clone());
                    for (var_index, variant) in variants.iter().enumerate() {
                        let var_id = format!("var_{}", var_index);
                        let mut var_path = new_path.clone();
                        var_path.push(var_id);
                        if !variant.children.is_empty() {
                            self.session.tree_state.open(var_path.clone());
                            self.collect_and_open_all(&variant.children, var_path);
                        }
                    }
                }
                TreeEntry::Singleton { id, children } => {
                    let tree_id = format!("{}_{}", id, sibling_index);
                    let mut new_path = path.clone();
                    new_path.push(tree_id);
                    if !children.is_empty() {
                        self.session.tree_state.open(new_path.clone());
                        self.collect_and_open_all(children, new_path);
                    }
                }
            }
        }
    }

    /// Collapse all tree nodes
    pub fn tree_collapse_all(&mut self) {
        // Get the top-level ancestor of current selection before collapsing
        let top_ancestor = self.session.tree_state.selected().first().cloned();
        
        self.session.tree_state.close_all();
        
        // Select the top-level ancestor, or first node if none
        if let Some(ancestor) = top_ancestor {
            self.session.tree_state.select(vec![ancestor]);
        } else {
            self.select_first_tree_node();
        }
    }

    pub fn clear_events(&mut self) {
        self.session.clear();
        self.scroll_offset = 0;
    }

    pub fn set_sessions(&mut self, sessions: Vec<Session>) {
        self.sessions = sessions;
        if self.selected_session.is_none() && !self.sessions.is_empty() {
            let first_session = self.sessions[0].clone();
            self.switch_to_session(first_session.id.clone());
            self.update_status_from_session(&first_session);
        }
    }

    pub fn session_picker_up(&mut self) {
        if self.session_picker_index > 0 {
            self.session_picker_index -= 1;
        }
    }

    pub fn session_picker_down(&mut self) {
        if self.session_picker_index < self.sessions.len().saturating_sub(1) {
            self.session_picker_index += 1;
        }
    }

    pub fn session_picker_select(&mut self) {
        if let Some(session) = self.sessions.get(self.session_picker_index).cloned() {
            self.switch_to_session(session.id.clone());
            self.update_status_from_session(&session);
        }
        self.mode = Mode::Normal;
    }

    pub fn update_status_from_session(&mut self, session: &crate::ws::protocol::Session) {
        match session.app_status.as_str() {
            "starting" => self.app_status = AppStatus::Starting,
            "running" => {
                let pid = session.pid.unwrap_or(0);
                let uri = session.vm_service_uri.clone().unwrap_or_default();
                self.app_status = AppStatus::Running { pid, uri };
            }
            "stopped" | "not_running" => self.app_status = AppStatus::Stopped,
            "error" => self.app_status = AppStatus::Error("Unknown error".to_string()),
            _ => self.app_status = AppStatus::Unknown,
        }
    }

    pub fn next_tab(&mut self) {
        self.content_tab = self.content_tab.next();
        self.scroll_offset = 0;
    }

    pub fn prev_tab(&mut self) {
        self.content_tab = self.content_tab.prev();
        self.scroll_offset = 0;
    }

    #[allow(dead_code)]
    pub fn tick(&mut self) {
        self.throbber_state.calc_next();
    }
}

fn summarize_payload(payload: &serde_json::Value) -> String {
    match payload {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => truncate(s, 80),
        serde_json::Value::Object(map) => {
            if let Some(msg) = map.get("message").and_then(|v| v.as_str()) {
                return truncate(msg, 80);
            }
            let keys: Vec<&str> = map.keys().map(|k| k.as_str()).take(3).collect();
            if keys.is_empty() {
                "{}".to_string()
            } else {
                format!("{{{}}}", keys.join(", "))
            }
        }
        serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
        other => truncate(&other.to_string(), 80),
    }
}

fn extract_flutter_log(payload: &serde_json::Value) -> String {
    // Flutter logs come as { "line": "..." }
    if let Some(line) = payload.get("line").and_then(|v| v.as_str()) {
        return line.to_string();
    }
    // Fall back to summarizing the payload
    summarize_payload(payload)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s.chars().take(max - 1).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_project() -> ProjectInfo {
        ProjectInfo {
            path: PathBuf::from("/test/project"),
            name: "test_app".to_string(),
            is_flutter: true,
        }
    }

    fn make_event(source: &str, event_type: &str) -> MonitoringEvent {
        MonitoringEvent {
            ts: "2024-01-01T00:00:00Z".to_string(),
            source: source.to_string(),
            event_type: event_type.to_string(),
            payload: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_new_app() {
        let app = App::new("ws://localhost:9000".to_string(), 100, test_project());
        assert_eq!(app.server_uri, "ws://localhost:9000");
        assert_eq!(app.max_events, 100);
        assert_eq!(app.ws_state, WsState::Disconnected);
        assert_eq!(app.mode, Mode::SessionPicker);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_push_interaction_log_trims_to_max() {
        let mut app = App::new("ws://localhost:9000".to_string(), 3, test_project());
        for i in 0..4 {
            app.push_interaction_log(LogEntry {
                ts: format!("2024-01-01T00:00:0{}Z", i),
                level: LogLevel::Info,
                message: format!("msg{}", i),
            });
        }

        assert_eq!(app.session.interaction_logs.len(), 3);
        assert!(app.session.interaction_logs[0].message.contains("msg1"));
        assert!(app.session.interaction_logs[2].message.contains("msg3"));
    }

    #[test]
    fn test_content_tab_cycling() {
        assert_eq!(ContentTab::Flutter.next(), ContentTab::Agent);
        assert_eq!(ContentTab::Agent.next(), ContentTab::Interactions);
        assert_eq!(ContentTab::Interactions.next(), ContentTab::Tree);
        assert_eq!(ContentTab::Tree.next(), ContentTab::Flutter);

        assert_eq!(ContentTab::Flutter.prev(), ContentTab::Tree);
        assert_eq!(ContentTab::Tree.prev(), ContentTab::Interactions);
        assert_eq!(ContentTab::Agent.prev(), ContentTab::Flutter);
    }

    #[test]
    fn test_clear_events() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        app.push_event(make_event("flutter", "flutter.log"));
        app.push_event(make_event("agent", "tool_call"));
        app.scroll_offset = 5;

        app.clear_events();
        assert!(app.session.flutter_logs.is_empty());
        assert!(app.session.agent_events.is_empty());
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_handle_command_response_running() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        let resp = CommandResponse {
            id: "1".to_string(),
            success: true,
            data: serde_json::json!({"status": "running", "pid": 1234, "uri": "ws://127.0.0.1:5678"}),
            error: None,
        };

        app.handle_command_response(resp);
        match app.app_status {
            AppStatus::Running { pid, uri } => {
                assert_eq!(pid, 1234);
                assert_eq!(uri, "ws://127.0.0.1:5678");
            }
            _ => panic!("Expected Running status"),
        }
    }

    #[test]
    fn test_handle_agent_response_needs_context() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
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
    fn test_handle_agent_response_success_clears_conversation() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        app.session.conversation_id = Some("conv-123".to_string());
        app.session.pending_response = true;

        let resp = AgentResponse {
            id: "1".to_string(),
            status: AgentStatus::Success,
            summary: Some("Done".to_string()),
            error: None,
            question: None,
            sdk_session_id: None,
        };

        app.handle_agent_response(resp);
        assert!(!app.session.pending_response);
        assert!(app.session.conversation_id.is_none());
    }

    #[test]
    fn test_handle_agent_response_creates_agent_event() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
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
        assert_eq!(app.session.agent_events.len(), 1);
        let event = app.session.agent_events.back().unwrap();
        assert_eq!(event.source, "agent");
        assert_eq!(event.event_type, "agent_success");
    }

    #[test]
    fn test_handle_agent_response_error_sets_last_error() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        app.session.pending_response = true;

        let resp = AgentResponse {
            id: "1".to_string(),
            status: AgentStatus::Error,
            summary: Some("Something went wrong".to_string()),
            error: None,
            question: None,
            sdk_session_id: None,
        };

        app.handle_agent_response(resp);
        assert_eq!(app.session.last_agent_error, Some("Something went wrong".to_string()));
        assert!(app.session.conversation_id.is_none());
    }

    #[test]
    fn test_handle_command_response_create_session() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        app.mode = Mode::SessionPicker;
        assert!(app.sessions.is_empty());

        let resp = CommandResponse {
            id: "1".to_string(),
            success: true,
            data: serde_json::json!({
                "session": {
                    "id": "sess-new",
                    "name": "my-new-session",
                    "projectPath": "/path/to/project",
                    "appStatus": "not_running",
                    "createdAt": "2024-01-01T00:00:00Z",
                    "lastActiveAt": "2024-01-01T00:00:00Z"
                }
            }),
            error: None,
        };

        app.handle_command_response(resp);

        // Session should be added
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.sessions[0].id, "sess-new");
        assert_eq!(app.sessions[0].name, "my-new-session");

        // Session should be auto-selected
        assert_eq!(app.selected_session, Some("sess-new".to_string()));

        // Mode should switch to Normal
        assert_eq!(app.mode, Mode::Normal);

        // Toast should be shown (can't easily check content, but toasts queue should have one)
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_handle_command_response_list_sessions() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        assert!(app.sessions.is_empty());

        let resp = CommandResponse {
            id: "1".to_string(),
            success: true,
            data: serde_json::json!({
                "sessions": [
                    {
                        "id": "sess-1",
                        "name": "app1",
                        "projectPath": "/path/1",
                        "appStatus": "running",
                        "vmServiceUri": "ws://127.0.0.1:5678",
                        "pid": 1234,
                        "createdAt": "2024-01-01T00:00:00Z",
                        "lastActiveAt": "2024-01-01T00:00:00Z"
                    },
                    {
                        "id": "sess-2",
                        "name": "app2",
                        "projectPath": "/path/2",
                        "appStatus": "not_running",
                        "createdAt": "2024-01-01T00:00:00Z",
                        "lastActiveAt": "2024-01-01T00:00:00Z"
                    }
                ]
            }),
            error: None,
        };

        app.handle_command_response(resp);

        assert_eq!(app.sessions.len(), 2);
        assert_eq!(app.sessions[0].id, "sess-1");
        assert_eq!(app.sessions[1].id, "sess-2");
        assert_eq!(app.session_picker_index, 0);
    }

    #[test]
    fn test_session_picker() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        app.set_sessions(vec![
            Session {
                id: "sess-1".to_string(),
                name: "app1".to_string(),
                project_path: "/path/1".to_string(),
                app_status: "running".to_string(),
                vm_service_uri: Some("ws://127.0.0.1:5678".to_string()),
                pid: Some(1234),
                connected_clients: Vec::new(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                last_active_at: "2024-01-01T00:00:00Z".to_string(),
            },
            Session {
                id: "sess-2".to_string(),
                name: "app2".to_string(),
                project_path: "/path/2".to_string(),
                app_status: "not_running".to_string(),
                vm_service_uri: None,
                pid: None,
                connected_clients: Vec::new(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                last_active_at: "2024-01-01T00:00:00Z".to_string(),
            },
        ]);

        assert_eq!(app.selected_session, Some("sess-1".to_string()));
        assert_eq!(app.session_picker_index, 0);

        app.session_picker_down();
        assert_eq!(app.session_picker_index, 1);

        app.session_picker_select();
        assert_eq!(app.selected_session, Some("sess-2".to_string()));
    }

    #[test]
    fn test_session_destroyed_event_removes_session() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        app.set_sessions(vec![
            Session {
                id: "sess-1".to_string(),
                name: "app1".to_string(),
                project_path: "/path/1".to_string(),
                app_status: "running".to_string(),
                vm_service_uri: None,
                pid: None,
                connected_clients: Vec::new(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                last_active_at: "2024-01-01T00:00:00Z".to_string(),
            },
            Session {
                id: "sess-2".to_string(),
                name: "app2".to_string(),
                project_path: "/path/2".to_string(),
                app_status: "not_running".to_string(),
                vm_service_uri: None,
                pid: None,
                connected_clients: Vec::new(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                last_active_at: "2024-01-01T00:00:00Z".to_string(),
            },
        ]);

        assert_eq!(app.sessions.len(), 2);
        assert_eq!(app.selected_session, Some("sess-1".to_string()));

        // Simulate session.destroyed event
        let event = MonitoringEvent {
            ts: "2024-01-01T00:00:00Z".to_string(),
            source: "session".to_string(),
            event_type: "session.destroyed".to_string(),
            payload: serde_json::json!({ "sessionId": "sess-1" }),
        };

        app.push_event(event);

        // Session should be removed
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.sessions[0].id, "sess-2");
        // Selected session was destroyed, should be cleared
        assert_eq!(app.selected_session, None);
        // Picker index should be adjusted
        assert_eq!(app.session_picker_index, 0);
    }

    #[test]
    fn test_session_destroyed_event_adjusts_picker_index() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        app.set_sessions(vec![
            Session {
                id: "sess-1".to_string(),
                name: "app1".to_string(),
                project_path: "/path/1".to_string(),
                app_status: "running".to_string(),
                vm_service_uri: None,
                pid: None,
                connected_clients: Vec::new(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                last_active_at: "2024-01-01T00:00:00Z".to_string(),
            },
            Session {
                id: "sess-2".to_string(),
                name: "app2".to_string(),
                project_path: "/path/2".to_string(),
                app_status: "not_running".to_string(),
                vm_service_uri: None,
                pid: None,
                connected_clients: Vec::new(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                last_active_at: "2024-01-01T00:00:00Z".to_string(),
            },
        ]);

        // Move picker to second session
        app.session_picker_index = 1;

        // Destroy the second session
        let event = MonitoringEvent {
            ts: "2024-01-01T00:00:00Z".to_string(),
            source: "session".to_string(),
            event_type: "session.destroyed".to_string(),
            payload: serde_json::json!({ "sessionId": "sess-2" }),
        };

        app.push_event(event);

        // Session should be removed
        assert_eq!(app.sessions.len(), 1);
        // Picker index should be adjusted to valid range
        assert_eq!(app.session_picker_index, 0);
    }

    #[test]
    fn test_session_created_event_adds_session() {
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        assert!(app.sessions.is_empty());

        // Simulate session.created event
        let event = MonitoringEvent {
            ts: "2024-01-01T00:00:00Z".to_string(),
            source: "session".to_string(),
            event_type: "session.created".to_string(),
            payload: serde_json::json!({
                "session": {
                    "id": "sess-new",
                    "name": "new-app",
                    "projectPath": "/path/new",
                    "appStatus": "not_running",
                    "createdAt": "2024-01-01T00:00:00Z",
                    "lastActiveAt": "2024-01-01T00:00:00Z"
                }
            }),
        };

        app.push_event(event);

        // Session should be added
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.sessions[0].id, "sess-new");
        assert_eq!(app.sessions[0].name, "new-app");
    }

    #[test]
    fn test_tree_expand_collapse_all() {
        use crate::types::InteractionTree;
        
        let mut app = App::new("ws://localhost:9000".to_string(), 10, test_project());
        
        // Create a tree with some nested nodes
        let tree = InteractionTree {
            nodes: vec![
                TreeNode {
                    id: "btn-1".to_string(),
                    widget_type: Some("Button".to_string()),
                    children: vec![],
                    contexts: vec![ContextInfo {
                        name: "counter-section".to_string(),
                        description: Some("Counter controls".to_string()),
                    }],
                    capabilities: vec![],
                    actions: vec![],
                },
                TreeNode {
                    id: "btn-2".to_string(),
                    widget_type: Some("Button".to_string()),
                    children: vec![],
                    contexts: vec![ContextInfo {
                        name: "counter-section".to_string(),
                        description: Some("Counter controls".to_string()),
                    }],
                    capabilities: vec![],
                    actions: vec![],
                },
                TreeNode {
                    id: "nav-btn".to_string(),
                    widget_type: Some("Button".to_string()),
                    children: vec![],
                    contexts: vec![],
                    capabilities: vec![],
                    actions: vec![],
                },
            ],
            last_updated: None,
        };
        
        app.set_tree(tree);
        
        // After set_tree, first node (context) should be selected
        assert_eq!(app.session.tree_state.selected(), vec!["ctx_0"]);
        
        // Context should be opened (expanded)
        assert!(app.session.tree_state.opened().contains(&vec!["ctx_0".to_string()]));
        
        // Navigate into nested node (simulate selecting a child)
        app.session.tree_state.select(vec!["ctx_0".to_string(), "btn-1_0".to_string()]);
        
        // Collapse all
        app.tree_collapse_all();
        
        // Nothing should be opened after collapse
        assert!(app.session.tree_state.opened().is_empty());
        
        // Should select the top-level ancestor (ctx_0), not first node
        assert_eq!(app.session.tree_state.selected(), vec!["ctx_0"]);
        
        // Expand all again
        app.tree_expand_all();
        
        // Context should be opened again
        assert!(app.session.tree_state.opened().contains(&vec!["ctx_0".to_string()]));
    }
}
