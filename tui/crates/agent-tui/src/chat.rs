//! Chat widget for displaying conversation messages.

use agent_protocol::{Message, MessageContent, Role, ToolCall, ToolStatus};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use crate::markdown::render_markdown;

/// Renders a single message.
pub fn render_message(msg: &Message) -> Text<'static> {
    let mut lines = Vec::new();
    
    // Role header
    let (role_label, role_color) = match msg.role {
        Role::User => ("You", Color::Blue),
        Role::Assistant => ("Claude", Color::Green),
    };
    
    lines.push(Line::from(vec![
        Span::styled(
            role_label,
            Style::default().fg(role_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            msg.timestamp.format("%H:%M:%S").to_string(),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    
    // Content
    match &msg.content {
        MessageContent::Text(text) => {
            let rendered = render_markdown(text);
            lines.extend(rendered.lines);
        }
        MessageContent::ToolUse { calls } => {
            for call in calls {
                lines.extend(render_tool_call(call).lines);
            }
        }
    }
    
    // Add blank line after message
    lines.push(Line::default());
    
    Text::from(lines)
}

/// Format tool arguments in a human-readable way.
fn format_tool_args(name: &str, args: &serde_json::Value) -> String {
    match name {
        // Interaction tree tools
        "getTree" => "Fetching widget tree...".to_string(),
        "getStatus" => "Checking app status...".to_string(),
        "getLogs" => {
            let max_lines = args.get("maxLines").and_then(|v| v.as_u64()).unwrap_or(100);
            format!("Getting last {} log lines", max_lines)
        }
        "getErrors" => "Checking for runtime errors...".to_string(),
        "getState" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            format!("Getting state of '{}'", id)
        }
        "execute" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let interaction = args.get("interaction").and_then(|v| v.as_str()).unwrap_or("?");
            let extra_args = args.get("args");
            match (interaction, extra_args) {
                ("enterText", Some(a)) => {
                    let text = a.get("text").and_then(|v| v.as_str()).unwrap_or("...");
                    format!("Typing '{}' into '{}'", text, id)
                }
                ("tap", _) => format!("Tapping '{}'", id),
                ("doubleTap", _) => format!("Double-tapping '{}'", id),
                ("longPress", _) => format!("Long-pressing '{}'", id),
                ("scroll", Some(a)) => {
                    let dx = a.get("dx").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let dy = a.get("dy").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    format!("Scrolling '{}' by ({:.0}, {:.0})", id, dx, dy)
                }
                _ => format!("{} on '{}'", interaction, id),
            }
        }
        "batch" => {
            let steps = args.get("steps").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
            format!("Executing {} steps in batch", steps)
        }
        // App lifecycle
        "hotReload" => "Hot reloading app...".to_string(),
        "hotRestart" => "Hot restarting app...".to_string(),
        "runApp" => {
            let device = args.get("deviceId").and_then(|v| v.as_str()).unwrap_or("default");
            format!("Starting app on '{}'", device)
        }
        "stopApp" => "Stopping app...".to_string(),
        // Session management
        "createSession" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            format!("Creating session '{}'", name)
        }
        "listSessions" => "Listing available sessions...".to_string(),
        "connectSession" => {
            let id = args.get("sessionId").and_then(|v| v.as_str()).unwrap_or("?");
            format!("Connecting to session '{}'", &id[..8.min(id.len())])
        }
        "destroySession" => "Destroying session...".to_string(),
        // Default: show truncated JSON
        _ => {
            let json = args.to_string();
            if json.len() > 60 {
                format!("{:.60}...", json)
            } else {
                json
            }
        }
    }
}

/// Format tool output in a human-readable way.
fn format_tool_output(name: &str, output: &str) -> Vec<String> {
    // Try to parse as JSON for smart formatting
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        match name {
            "getStatus" => {
                let vm_connected = json.get("vmConnected").and_then(|v| v.as_bool()).unwrap_or(false);
                let session = json.get("currentSessionId").and_then(|v| v.as_str());
                let mut result = Vec::new();
                result.push(format!("  {} App: {}", 
                    if vm_connected { "✓" } else { "✗" },
                    if vm_connected { "Running" } else { "Not running" }
                ));
                if let Some(sid) = session {
                    result.push(format!("  Session: {}", &sid[..8.min(sid.len())]));
                }
                return result;
            }
            "getTree" => {
                // Count widgets in tree
                fn count_nodes(node: &serde_json::Value) -> usize {
                    let child_count: usize = node.get("children")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().map(count_nodes).sum())
                        .unwrap_or(0);
                    1 + child_count
                }
                if let Some(tree) = json.get("tree") {
                    let count = count_nodes(tree);
                    return vec![format!("  Found {} interactable widgets", count)];
                }
            }
            "execute" => {
                let success = json.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                if success {
                    return vec!["  ✓ Action completed".to_string()];
                } else {
                    let error = json.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                    return vec![format!("  ✗ Failed: {}", error)];
                }
            }
            "getLogs" => {
                if let Some(logs) = json.get("logs").and_then(|v| v.as_array()) {
                    let error_count = logs.iter().filter(|l| {
                        l.as_str().map(|s| s.contains("ERROR") || s.contains("Exception")).unwrap_or(false)
                    }).count();
                    let mut result = vec![format!("  {} log lines", logs.len())];
                    if error_count > 0 {
                        result.push(format!("  ⚠ {} errors found", error_count));
                    }
                    return result;
                }
            }
            "getErrors" => {
                if let Some(errors) = json.get("errors").and_then(|v| v.as_array()) {
                    if errors.is_empty() {
                        return vec!["  ✓ No runtime errors".to_string()];
                    } else {
                        return vec![format!("  ⚠ {} runtime errors", errors.len())];
                    }
                }
            }
            "createSession" | "connectSession" => {
                let created = json.get("created").and_then(|v| v.as_bool()).unwrap_or(false);
                let connected = json.get("connected").and_then(|v| v.as_bool()).unwrap_or(false);
                if created || connected {
                    if let Some(session) = json.get("session") {
                        let id = session.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                        return vec![format!("  ✓ Session: {}", &id[..8.min(id.len())])];
                    }
                }
            }
            "hotReload" | "hotRestart" => {
                let success = json.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                if success {
                    return vec!["  ✓ Reload complete".to_string()];
                }
            }
            "listSessions" => {
                if let Some(sessions) = json.get("sessions").and_then(|v| v.as_array()) {
                    if sessions.is_empty() {
                        return vec!["  No sessions found".to_string()];
                    } else {
                        return vec![format!("  Found {} sessions", sessions.len())];
                    }
                }
            }
            _ => {}
        }
    }
    
    // Default: show first few lines, truncated
    output.lines()
        .take(3)
        .map(|line| {
            let truncated = if line.len() > 80 {
                format!("  {}...", &line[..77])
            } else {
                format!("  {}", line)
            };
            truncated
        })
        .collect()
}

/// Renders a tool call.
fn render_tool_call(call: &ToolCall) -> Text<'static> {
    let mut lines = Vec::new();
    
    // Status indicator and tool name
    let (status_icon, status_color) = match call.status {
        ToolStatus::Running => ("⟳", Color::Yellow),
        ToolStatus::Success => ("✓", Color::Green),
        ToolStatus::Failed => ("✗", Color::Red),
    };
    
    lines.push(Line::from(vec![
        Span::styled(status_icon, Style::default().fg(status_color)),
        Span::raw(" "),
        Span::styled(
            call.name.clone(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
    ]));
    
    // Human-readable description of what the tool is doing
    let description = format_tool_args(&call.name, &call.args);
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(description, Style::default().fg(Color::White)),
    ]));
    
    // Output if available - formatted nicely
    if let Some(output) = &call.output {
        let formatted_lines = format_tool_output(&call.name, output);
        for line in formatted_lines {
            lines.push(Line::from(vec![
                Span::styled(line, Style::default().fg(Color::Gray)),
            ]));
        }
        
        // Show expand hint if output is large
        let total_lines = output.lines().count();
        if total_lines > 3 {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  [+{} more lines - press 'e' to expand]", total_lines - 3),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }
    
    Text::from(lines)
}

/// Chat display widget showing all messages.
pub struct ChatWidget<'a> {
    messages: &'a [Message],
    streaming_text: Option<&'a str>,
    scroll: u16,
}

impl<'a> ChatWidget<'a> {
    pub fn new(messages: &'a [Message]) -> Self {
        Self {
            messages,
            streaming_text: None,
            scroll: 0,
        }
    }
    
    pub fn streaming(mut self, text: Option<&'a str>) -> Self {
        self.streaming_text = text;
        self
    }
    
    pub fn scroll(mut self, scroll: u16) -> Self {
        self.scroll = scroll;
        self
    }
}

impl<'a> Widget for ChatWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Collect all rendered text
        let mut all_lines: Vec<Line<'static>> = Vec::new();
        
        for msg in self.messages {
            all_lines.extend(render_message(msg).lines);
        }
        
        // Add streaming text if present
        if let Some(streaming) = self.streaming_text {
            all_lines.push(Line::from(vec![
                Span::styled("Claude", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled("...", Style::default().fg(Color::Yellow)),
            ]));
            all_lines.extend(render_markdown(streaming).lines);
        }
        
        let text = Text::from(all_lines);
        
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0))
            .render(area, buf);
    }
}
