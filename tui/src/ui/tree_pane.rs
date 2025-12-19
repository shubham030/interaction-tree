use std::collections::BTreeMap;

use crate::app::App;
use crate::theme::theme;
use crate::tree_format::{Schema, TreeEntry, Variant};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_tree_widget::{Tree, TreeItem};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let t = theme();

    let block = Block::default()
        .title(" Interaction Tree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border));

    if let Some(compact) = app.compact_tree() {
        let items: Vec<TreeItem<'_, String>> = compact
            .tree
            .iter()
            .enumerate()
            .map(|(i, entry)| build_entry_item(entry, &compact.schemas, i))
            .collect();

        let tree_widget = Tree::new(&items)
            .expect("tree items have unique identifiers")
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(t.text_highlight)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_stateful_widget(tree_widget, area, &mut app.session.tree_state);
    } else {
        let paragraph = Paragraph::new("No tree data (app not running?)")
            .style(Style::default().fg(t.text_dim))
            .block(block);
        frame.render_widget(paragraph, area);
    }
}

fn build_entry_item(
    entry: &TreeEntry,
    schemas: &BTreeMap<String, Schema>,
    sibling_index: usize,
) -> TreeItem<'static, String> {
    match entry {
        TreeEntry::Context { context, children, .. } => {
            let label = Line::from(Span::styled(
                context.clone(),
                Style::new().add_modifier(Modifier::BOLD),
            ));
            let child_items: Vec<_> = children
                .iter()
                .enumerate()
                .map(|(i, c)| build_entry_item(c, schemas, i))
                .collect();
            TreeItem::new(format!("ctx_{}", sibling_index), label, child_items)
                .expect("unique ids")
        }
        TreeEntry::Homogeneous {
            id,
            count,
            children,
        } => {
            let label = format_homogeneous_label(id, *count, schemas);
            let child_items = build_children(children, schemas);
            if child_items.is_empty() {
                TreeItem::new_leaf(format!("{}_{}", id, sibling_index), label)
            } else {
                TreeItem::new(format!("{}_{}", id, sibling_index), label, child_items)
                    .expect("unique ids")
            }
        }
        TreeEntry::Singleton { id, children } => {
            let label = format_singleton_label(id, schemas);
            let child_items = build_children(children, schemas);
            if child_items.is_empty() {
                TreeItem::new_leaf(format!("{}_{}", id, sibling_index), label)
            } else {
                TreeItem::new(format!("{}_{}", id, sibling_index), label, child_items)
                    .expect("unique ids")
            }
        }
        TreeEntry::Variants { id, variants } => {
            let label = format_id_label(id, schemas);
            let child_items: Vec<_> = variants
                .iter()
                .enumerate()
                .map(|(i, v)| build_variant_item(v, schemas, i))
                .collect();
            TreeItem::new(format!("{}_{}", id, sibling_index), label, child_items)
                .expect("unique ids")
        }
        TreeEntry::Heterogeneous { items } => {
            let child_items: Vec<_> = items
                .iter()
                .enumerate()
                .map(|(i, item)| build_entry_item(item, schemas, i))
                .collect();
            TreeItem::new(
                format!("het_{}", sibling_index),
                Line::from("[items]"),
                child_items,
            )
            .expect("unique ids")
        }
    }
}

fn build_variant_item(
    variant: &Variant,
    schemas: &BTreeMap<String, Schema>,
    index: usize,
) -> TreeItem<'static, String> {
    let mut spans = vec![Span::styled(
        format!("(×{})", variant.count),
        Style::new().fg(Color::Cyan),
    )];

    // If single child, add arrow inline
    if variant.children.len() == 1 {
        if let Some(child_id) = variant.children[0].id() {
            spans.push(Span::raw(" → "));
            spans.push(Span::raw(child_id.to_string()));
            // Add capabilities for the child
            if let Some(schema) = schemas.get(child_id) {
                if !schema.capabilities.is_empty() {
                    spans.push(Span::styled(
                        format!(" [{}]", schema.capabilities.join(", ")),
                        Style::new().fg(Color::Green),
                    ));
                }
            }
        }
    }

    let label = Line::from(spans);

    if variant.children.len() <= 1 {
        TreeItem::new_leaf(format!("var_{}", index), label)
    } else {
        let child_items = build_children(&variant.children, schemas);
        TreeItem::new(format!("var_{}", index), label, child_items).expect("unique ids")
    }
}

fn build_children(
    children: &[TreeEntry],
    schemas: &BTreeMap<String, Schema>,
) -> Vec<TreeItem<'static, String>> {
    children
        .iter()
        .enumerate()
        .map(|(i, c)| build_entry_item(c, schemas, i))
        .collect()
}

fn format_singleton_label(id: &str, schemas: &BTreeMap<String, Schema>) -> Line<'static> {
    let mut spans = vec![Span::raw(id.to_string())];
    add_schema_info(&mut spans, id, schemas);
    Line::from(spans)
}

fn format_homogeneous_label(
    id: &str,
    count: usize,
    schemas: &BTreeMap<String, Schema>,
) -> Line<'static> {
    let mut spans = vec![
        Span::raw(id.to_string()),
        Span::styled(
            format!(" ×{}", count),
            Style::new()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    add_schema_info(&mut spans, id, schemas);
    Line::from(spans)
}

fn format_id_label(id: &str, schemas: &BTreeMap<String, Schema>) -> Line<'static> {
    let mut spans = vec![Span::raw(id.to_string())];
    add_schema_info(&mut spans, id, schemas);
    Line::from(spans)
}

fn add_schema_info(spans: &mut Vec<Span<'static>>, id: &str, schemas: &BTreeMap<String, Schema>) {
    if let Some(schema) = schemas.get(id) {
        if !schema.capabilities.is_empty() {
            spans.push(Span::styled(
                format!(" [{}]", schema.capabilities.join(", ")),
                Style::new().fg(Color::Green),
            ));
        }
        if !schema.actions.is_empty() {
            spans.push(Span::styled(
                format!(" {{{}}}", schema.actions.join(", ")),
                Style::new().fg(Color::Magenta),
            ));
        }
    }
}
