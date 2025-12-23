use std::collections::VecDeque;

use tui_tree_widget::TreeState;

use crate::chat::{ChatMessage, StreamingState};
use crate::flutter_log::FlutterLogEntry;
use crate::types::{InteractionTree, LogEntry};
use crate::ws::protocol::MonitoringEvent;

/// Session-specific state that is preserved when switching between sessions
#[derive(Debug)]
pub struct SessionState {
    /// Flutter logs (from flutter run process)
    pub flutter_logs: VecDeque<FlutterLogEntry>,
    /// Agent events (tool calls, responses)
    pub agent_events: VecDeque<MonitoringEvent>,
    /// Interaction execution history (taps, scrolls, etc.)
    pub interaction_logs: VecDeque<LogEntry>,

    /// Chat messages for the Agent pane
    pub chat_messages: Vec<ChatMessage>,
    /// Current streaming response from agent
    pub chat_streaming: Option<StreamingState>,
    /// Composer text input
    pub chat_input: String,
    /// Cursor position in chat input
    pub chat_cursor: usize,
    /// Scroll offset for chat view (0 = auto-scroll to bottom)
    pub chat_scroll: usize,

    pub tree: Option<InteractionTree>,
    pub tree_state: TreeState<String>,

    pub conversation_id: Option<String>,
    pub agent_question: Option<String>,
    pub pending_response: bool,
    pub pending_intent: Option<String>,
    pub last_agent_error: Option<String>,

    /// Maximum events to keep per log type
    #[allow(dead_code)]
    max_events: usize,
}

impl SessionState {
    pub fn new(max_events: usize) -> Self {
        Self {
            flutter_logs: VecDeque::with_capacity(max_events),
            agent_events: VecDeque::with_capacity(max_events),
            interaction_logs: VecDeque::with_capacity(max_events),
            chat_messages: Vec::new(),
            chat_streaming: None,
            chat_input: String::new(),
            chat_cursor: 0,
            chat_scroll: 0,
            tree: None,
            tree_state: TreeState::default(),
            conversation_id: None,
            agent_question: None,
            pending_response: false,
            pending_intent: None,
            last_agent_error: None,
            max_events,
        }
    }

    /// Clear all session-specific data
    pub fn clear(&mut self) {
        self.flutter_logs.clear();
        self.agent_events.clear();
        self.interaction_logs.clear();
        self.chat_messages.clear();
        self.chat_streaming = None;
        self.chat_input.clear();
        self.chat_cursor = 0;
        self.chat_scroll = 0;
        self.tree = None;
        self.tree_state = TreeState::default();
        self.conversation_id = None;
        self.agent_question = None;
        self.pending_response = false;
        self.pending_intent = None;
        self.last_agent_error = None;
    }
}
