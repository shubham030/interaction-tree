use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use serde::Deserialize;

use crate::theme::theme;

#[derive(Debug, Clone)]
pub struct FlutterLogEntry {
    pub kind: FlutterLogKind,
    pub raw: String,
}

#[derive(Debug, Clone)]
pub enum FlutterLogKind {
    AppProgress { #[allow(dead_code)] id: String, message: String, finished: bool },
    AppLog { message: String, error: bool },
    AppStarted,
    AppDebugPort { ws_uri: String },
    DaemonConnected,
    DeviceAdded { name: String, platform: String },
    AppWebLaunchUrl { url: String },
    Plain(String),
    MachineJson(serde_json::Value),
}

#[derive(Debug, Deserialize)]
struct MachineEvent {
    event: Option<String>,
    params: Option<serde_json::Value>,
}

impl FlutterLogEntry {
    pub fn parse(line: &str) -> Self {
        let raw = line.to_string();
        let trimmed = line.trim();

        if trimmed.starts_with('[') || trimmed.starts_with('{') {
            if let Ok(events) = serde_json::from_str::<Vec<MachineEvent>>(trimmed) {
                if let Some(event) = events.into_iter().next() {
                    return Self {
                        kind: Self::parse_machine_event(event),
                        raw,
                    };
                }
            }
            if let Ok(event) = serde_json::from_str::<MachineEvent>(trimmed) {
                return Self {
                    kind: Self::parse_machine_event(event),
                    raw,
                };
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                return Self {
                    kind: FlutterLogKind::MachineJson(json),
                    raw,
                };
            }
        }

        Self {
            kind: FlutterLogKind::Plain(line.to_string()),
            raw,
        }
    }

    fn parse_machine_event(event: MachineEvent) -> FlutterLogKind {
        let params = event.params.unwrap_or(serde_json::Value::Null);

        match event.event.as_deref() {
            Some("app.progress") => {
                let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let message = params.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let finished = params.get("finished").and_then(|v| v.as_bool()).unwrap_or(false);
                FlutterLogKind::AppProgress { id, message, finished }
            }
            Some("app.log") => {
                let log = params.get("log").and_then(|v| v.as_str()).unwrap_or("");
                let error = params.get("error").and_then(|v| v.as_bool()).unwrap_or(false);
                FlutterLogKind::AppLog { message: log.to_string(), error }
            }
            Some("app.started") => FlutterLogKind::AppStarted,
            Some("app.debugPort") => {
                let ws_uri = params.get("wsUri").and_then(|v| v.as_str()).unwrap_or("").to_string();
                FlutterLogKind::AppDebugPort { ws_uri }
            }
            Some("daemon.connected") => FlutterLogKind::DaemonConnected,
            Some("device.added") => {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let platform = params.get("platform").and_then(|v| v.as_str()).unwrap_or("").to_string();
                FlutterLogKind::DeviceAdded { name, platform }
            }
            Some("app.webLaunchUrl") => {
                let url = params.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                FlutterLogKind::AppWebLaunchUrl { url }
            }
            _ => FlutterLogKind::MachineJson(serde_json::json!({
                "event": event.event,
                "params": params
            })),
        }
    }

    pub fn to_line(&self) -> Line<'static> {
        let t = theme();

        match &self.kind {
            FlutterLogKind::AppProgress { id: _, message, finished } => {
                let icon = if *finished { "✓" } else { "⏳" };
                let color = if *finished { t.success } else { t.text_dim };
                Line::from(vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(color)),
                    Span::styled(message.clone(), Style::default().fg(t.text)),
                ])
            }
            FlutterLogKind::AppLog { message, error } => {
                let (icon, color) = if *error {
                    ("✗", t.error)
                } else {
                    ("│", t.text_dim)
                };
                let msg_color = if *error { t.error } else { t.text };

                let processed = process_ansi_escapes(message, msg_color);
                let mut spans = vec![Span::styled(format!("{} ", icon), Style::default().fg(color))];
                spans.extend(processed);
                Line::from(spans)
            }
            FlutterLogKind::AppStarted => Line::from(vec![
                Span::styled("✓ ", Style::default().fg(t.success)),
                Span::styled("App started", Style::default().fg(t.success)),
            ]),
            FlutterLogKind::AppDebugPort { ws_uri } => Line::from(vec![
                Span::styled("🔗 ", Style::default().fg(t.info)),
                Span::styled("Debug: ", Style::default().fg(t.text_dim)),
                Span::styled(ws_uri.clone(), Style::default().fg(t.info)),
            ]),
            FlutterLogKind::DaemonConnected => Line::from(vec![
                Span::styled("✓ ", Style::default().fg(t.success)),
                Span::styled("Flutter daemon connected", Style::default().fg(t.text_dim)),
            ]),
            FlutterLogKind::DeviceAdded { name, platform } => Line::from(vec![
                Span::styled("📱 ", Style::default().fg(t.info)),
                Span::styled(format!("{} ", name), Style::default().fg(t.text)),
                Span::styled(format!("({})", platform), Style::default().fg(t.text_dim)),
            ]),
            FlutterLogKind::AppWebLaunchUrl { url } => Line::from(vec![
                Span::styled("🌐 ", Style::default().fg(t.info)),
                Span::styled(url.clone(), Style::default().fg(t.info)),
            ]),
            FlutterLogKind::Plain(text) => {
                let processed = process_ansi_escapes(text, t.text);
                Line::from(processed)
            }
            FlutterLogKind::MachineJson(json) => {
                let event = json.get("event").and_then(|v| v.as_str()).unwrap_or("unknown");
                Line::from(vec![
                    Span::styled("⚙ ", Style::default().fg(t.text_dim)),
                    Span::styled(event.to_string(), Style::default().fg(t.text_dim)),
                ])
            }
        }
    }

    pub fn is_noise(&self) -> bool {
        match &self.kind {
            FlutterLogKind::MachineJson(_) => true,
            FlutterLogKind::DaemonConnected => true,
            FlutterLogKind::DeviceAdded { .. } => true,
            // Empty progress messages (just a checkmark with no text)
            FlutterLogKind::AppProgress { message, .. } if message.trim().is_empty() => true,
            _ => false,
        }
    }

    /// Convert to plain text for yanking/copying
    pub fn to_plain_text(&self) -> String {
        match &self.kind {
            FlutterLogKind::AppProgress { message, finished, .. } => {
                let icon = if *finished { "✓" } else { "⏳" };
                format!("{} {}", icon, message)
            }
            FlutterLogKind::AppLog { message, error } => {
                let icon = if *error { "✗" } else { "│" };
                format!("{} {}", icon, strip_ansi(message))
            }
            FlutterLogKind::AppStarted => "✓ App started".to_string(),
            FlutterLogKind::AppDebugPort { ws_uri } => format!("🔗 Debug: {}", ws_uri),
            FlutterLogKind::DaemonConnected => "✓ Flutter daemon connected".to_string(),
            FlutterLogKind::DeviceAdded { name, platform } => format!("📱 {} ({})", name, platform),
            FlutterLogKind::AppWebLaunchUrl { url } => format!("🌐 {}", url),
            FlutterLogKind::Plain(text) => strip_ansi(text),
            FlutterLogKind::MachineJson(json) => {
                let event = json.get("event").and_then(|v| v.as_str()).unwrap_or("unknown");
                format!("⚙ {}", event)
            }
        }
    }
}

/// Strip ANSI escape codes from text
fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&ch) = chars.peek() {
                    chars.next();
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn process_ansi_escapes(text: &str, default_color: Color) -> Vec<Span<'static>> {
    use ansi_to_tui::IntoText;

    if let Ok(parsed) = text.as_bytes().into_text() {
        let mut spans = Vec::new();
        for line in parsed.lines {
            spans.extend(line.spans.into_iter().map(|s| Span::from(s)));
        }
        if spans.is_empty() {
            vec![Span::styled(text.to_string(), Style::default().fg(default_color))]
        } else {
            spans
        }
    } else {
        vec![Span::styled(text.to_string(), Style::default().fg(default_color))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_app_progress() {
        let line = r#"[{"event":"app.progress","params":{"id":"1","message":"Compiling...","finished":false}}]"#;
        let entry = FlutterLogEntry::parse(line);
        assert!(matches!(entry.kind, FlutterLogKind::AppProgress { .. }));
    }

    #[test]
    fn test_parse_app_log() {
        let line = r#"[{"event":"app.log","params":{"log":"Hello world","error":false}}]"#;
        let entry = FlutterLogEntry::parse(line);
        assert!(matches!(entry.kind, FlutterLogKind::AppLog { error: false, .. }));
    }

    #[test]
    fn test_parse_plain() {
        let line = "Launching lib/main.dart";
        let entry = FlutterLogEntry::parse(line);
        assert!(matches!(entry.kind, FlutterLogKind::Plain(_)));
    }

    #[test]
    fn test_parse_app_started() {
        let line = r#"{"event":"app.started","params":{}}"#;
        let entry = FlutterLogEntry::parse(line);
        assert!(matches!(entry.kind, FlutterLogKind::AppStarted));
    }
}
