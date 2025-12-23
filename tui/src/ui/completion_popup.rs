use crate::app::App;
use ratatui::{layout::Rect, Frame};

// Completion popup - currently unused, hotkey-driven UI instead
// Keeping the module for potential future use with command mode
#[allow(dead_code)]
pub fn render(_frame: &mut Frame, _app: &App, _input_area: Rect) {
    // Disabled - no command mode currently
}

/*
// Original completion popup implementation - preserved for future use
use crate::theme::theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
};

const MAX_VISIBLE_ITEMS: usize = 8;

pub fn render_completions(frame: &mut Frame, app: &App, input_area: Rect) {
    if app.completions.is_empty() {
        return;
    }

    let t = theme();

    let visible_count = app.completions.len().min(MAX_VISIBLE_ITEMS);
    let popup_height = visible_count as u16 + 2;

    let prefix_len = 2; // ": " prefix in command mode
    let anchor_col = app.completion_start_col.saturating_sub(1);
    let cursor_x = input_area.x + prefix_len + anchor_col as u16;

    let max_width = app
        .completions
        .iter()
        .map(|c| c.len())
        .max()
        .unwrap_or(10) as u16
        + 4;
    let popup_width = max_width.min(30).max(14);

    let popup_x = cursor_x.saturating_sub(1);
    let popup_x = popup_x.min(frame.area().width.saturating_sub(popup_width));

    let popup_y = input_area.y.saturating_sub(popup_height);

    let area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    frame.render_widget(Clear, area);

    let start_idx = if app.completion_index >= MAX_VISIBLE_ITEMS {
        app.completion_index - MAX_VISIBLE_ITEMS + 1
    } else {
        0
    };

    let items: Vec<ListItem> = app
        .completions
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(MAX_VISIBLE_ITEMS)
        .map(|(idx, cmd)| {
            let is_selected = idx == app.completion_index;
            let style = if is_selected {
                Style::default()
                    .fg(t.text)
                    .bg(t.info)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.text)
            };
            ListItem::new(Line::from(Span::styled(*cmd, style)))
        })
        .collect();

    let block = Block::default()
        .title(" Completions ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border));

    let list = List::new(items).block(block);

    frame.render_widget(list, area);
}
*/
