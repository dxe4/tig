use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};

use crate::app::{App, Focus};
use crate::ui::theme::Theme;

pub fn draw_files(f: &mut Frame, app: &mut App, area: Rect) {
    app.tree_visible_height = area.height.saturating_sub(2) as usize;

    let items: Vec<ListItem> = app
        .tree_entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = i == app.selected_tree_item;
            let prefix = if entry.is_dir {
                if entry.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };
            let indent = "  ".repeat(entry.depth);

            let mut spans = vec![Span::styled(
                format!("{}{}", indent, prefix),
                Style::default().fg(Theme::SUBTEXT),
            )];

            if !entry.is_dir {
                let status_color = match entry.status {
                    Some('M') => Theme::YELLOW,
                    Some('A') => Theme::GREEN,
                    Some('D') => Theme::RED,
                    Some('?') => Theme::BLUE,
                    Some('R') => Theme::MAUVE,
                    _ => Theme::SUBTEXT,
                };
                spans.push(Span::styled(
                    format!("{} ", entry.status.unwrap_or(' ')),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            let name_color = Theme::TEXT;
            spans.push(Span::styled(&entry.name, Style::default().fg(name_color)));

            if entry.added > 0 {
                spans.push(Span::styled(
                    format!(" +{}", entry.added),
                    Style::default().fg(Theme::GREEN),
                ));
            }
            if entry.removed > 0 {
                spans.push(Span::styled(
                    format!(" -{}", entry.removed),
                    Style::default().fg(Theme::RED),
                ));
            }

            let line = Line::from(spans);
            let style = if is_selected && app.focus == Focus::Files {
                Style::default()
                    .bg(Theme::CURSOR)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().bg(Theme::OVERLAY)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let current_file_pos = app
        .tree_entries
        .iter()
        .take(app.selected_tree_item + 1)
        .filter(|e| e.file_index.is_some())
        .count();
    let title = format!("Files {}/{}", current_file_pos, app.files.len());

    let border_style = if app.focus == Focus::Files {
        Style::default().fg(Theme::BLUE)
    } else {
        Style::default().fg(Theme::OVERLAY)
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Span::styled(title, Style::default().fg(Theme::TEXT))),
    );

    let mut state = ListState::default();
    state.select(Some(app.selected_tree_item));
    if app.tree_visible_height > 0 {
        let offset = state.offset();
        if app.selected_tree_item >= offset + app.tree_visible_height {
            *state.offset_mut() = app
                .selected_tree_item
                .saturating_sub(app.tree_visible_height - 1);
        } else if app.selected_tree_item < offset {
            *state.offset_mut() = app.selected_tree_item;
        }
    }
    f.render_stateful_widget(list, area, &mut state);
}
