use crate::app::{App, ContentTab, InputPromptKind, Mode};
use crate::theme::theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let t = theme();

    let bindings = match &app.mode {
        Mode::Normal => {
            let mut bindings = vec![
                ("hl", "tabs"),
                ("↑↓/jk", "scroll"),
                ("/", "filter"),
                ("s", "sessions"),
            ];

            match app.content_tab {
                ContentTab::Tree => {
                    bindings[1] = ("↑↓/jk", "navigate");
                    bindings.push(("←→", "expand/collapse"));
                    bindings.push(("Enter", "toggle"));
                    bindings.push(("e/E", "expand/collapse all"));
                    if app.is_app_running() {
                        bindings.push(("r", "refresh"));
                    }
                }
                ContentTab::Flutter => {
                    if app.has_session() {
                        if app.is_app_running() {
                            bindings.push(("r", "reload"));
                            bindings.push(("R", "restart"));
                            bindings.push(("x", "stop"));
                        } else {
                            bindings.push(("p", "run"));
                        }
                    }
                }
                ContentTab::Agent => {
                    bindings.push(("Enter", "chat"));
                }
                _ => {}
            }

            bindings.push(("c", "clear"));
            bindings.push(("?", "help"));
            bindings
        }
        Mode::Filter => vec![("Enter", "apply"), ("Esc", "cancel")],
        Mode::Help => vec![("Esc", "close"), ("q", "quit")],
        Mode::Confirm(_) => vec![("y", "confirm"), ("n/Esc", "cancel")],
        Mode::SessionPicker => vec![
            ("↑↓/jk", "select"),
            ("Enter", "connect"),
            ("c", "create"),
            ("d", "delete"),
            ("Esc", "close"),
        ],
        Mode::InputPrompt(kind) => match kind {
            InputPromptKind::CreateSession => vec![("Enter", "create"), ("Esc", "cancel")],
            InputPromptKind::RunApp => vec![("Enter", "run (empty=default)"), ("Esc", "cancel")],
        },
        Mode::ActionMenu => vec![
            ("↑↓/jk", "select"),
            ("←→/hl", "submenu"),
            ("Enter", "execute"),
            ("Esc", "cancel"),
        ],
        Mode::AgentChat => vec![
            ("Enter", "send"),
            ("Esc", "cancel"),
        ],
    };

    let mut spans = Vec::new();
    for (i, (key, desc)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ", Style::default()));
        }
        spans.push(Span::styled(
            *key,
            Style::default()
                .fg(t.text_highlight)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(":{}", desc),
            Style::default().fg(t.text_dim),
        ));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
