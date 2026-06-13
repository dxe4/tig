use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::app::{App, Mode};
use crate::ui::theme::Theme;

pub fn draw_search_popup(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 5;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let title = match app.mode {
        Mode::SearchContent => {
            if let Some(ref filter) = app.search_filter_text() {
                format!("Search Content [{}]", filter)
            } else {
                String::from("Search Content")
            }
        }
        Mode::SearchFilename => String::from("Search Filename"),
        _ => String::from(""),
    };

    let match_text = if app.search_results.is_empty() && !app.search_query.is_empty() {
        "No matches".to_string()
    } else if !app.search_results.is_empty() {
        format!("{} matches", app.search_results.len())
    } else {
        String::new()
    };

    let mut lines = vec![Line::styled(
        app.search_query.as_str(),
        Style::default().fg(Theme::TEXT),
    )];
    if !match_text.is_empty() {
        lines.push(Line::styled(
            match_text,
            Style::default().fg(Theme::SUBTEXT),
        ));
    }
    let text = Text::from(lines);

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::BLUE))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Theme::SURFACE)),
    );
    f.render_widget(paragraph, popup_area);

    let cursor_x = popup_area.x
        + 1
        + app
            .search_query
            .len()
            .min(popup_width.saturating_sub(2) as usize) as u16;
    let cursor_y = popup_area.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

pub fn draw_global_search_popup(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup_width = (area.width as f32 * 0.9) as u16;
    let popup_height = (area.height as f32 * 0.8) as u16;
    let x = area.x + (area.width - popup_width) / 2;
    let y = area.y + (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .margin(1)
        .split(popup_area);

    let title = Span::styled(
        "Global Search",
        Style::default()
            .fg(Theme::TEXT)
            .add_modifier(Modifier::BOLD),
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BLUE))
        .title(title)
        .style(Style::default().bg(Theme::SURFACE));
    f.render_widget(block, popup_area);

    let query_text = if app.global_search_query.is_empty() {
        Text::from(Line::styled(
            "Type regex...",
            Style::default().fg(Theme::SUBTEXT),
        ))
    } else {
        Text::from(Line::styled(
            &app.global_search_query,
            Style::default().fg(Theme::TEXT),
        ))
    };
    let query_para = Paragraph::new(query_text).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Theme::OVERLAY)),
    );
    f.render_widget(query_para, inner[0]);

    let visible_height = inner[1].height as usize;
    let result_count = app.global_search_results.len();
    let selected = app.global_search_selected;

    let scroll_offset = if selected >= visible_height {
        selected - visible_height + 1
    } else {
        0
    };

    let mut list_lines: Vec<Line> = Vec::new();
    for (idx, m) in app
        .global_search_results
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
    {
        let is_selected = idx == selected;
        let path_style = Style::default().fg(Theme::TEAL);
        let content_style = Style::default().fg(Theme::TEXT);

        let path_span = Span::styled(format!("{}:{:<4} ", m.file_path, m.line_number), path_style);

        let before = &m.content[..m.match_start];
        let matched = &m.content[m.match_start..m.match_end];
        let after = &m.content[m.match_end..];

        let mut spans = vec![path_span];
        spans.push(Span::styled(before, content_style));
        spans.push(Span::styled(
            matched,
            Style::default().fg(Theme::BASE).bg(Theme::YELLOW),
        ));
        spans.push(Span::styled(after, content_style));

        let line = Line::from(spans);
        let style = if is_selected {
            Style::default().bg(Theme::CURSOR)
        } else {
            Style::default()
        };
        list_lines.push(line.style(style));
    }

    if list_lines.is_empty() {
        list_lines.push(Line::styled(
            if app.global_search_query.is_empty() {
                ""
            } else {
                "No matches"
            },
            Style::default().fg(Theme::SUBTEXT),
        ));
    }

    let results_text = Text::from(list_lines);
    let results_para = Paragraph::new(results_text);
    f.render_widget(results_para, inner[1]);

    let status = if result_count == 0 {
        String::new()
    } else {
        format!(
            "{} / {}    j/k:nav  Enter:jump  Esc:close",
            selected + 1,
            result_count
        )
    };
    let status_para = Paragraph::new(status).style(Style::default().fg(Theme::SUBTEXT));
    f.render_widget(status_para, inner[2]);

    let cursor_x = inner[0].x
        + 1
        + app
            .global_search_query
            .len()
            .min((inner[0].width.saturating_sub(2)) as usize) as u16;
    let cursor_y = inner[0].y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

pub fn draw_branch_select_popup(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 20.min(area.height.saturating_sub(4));
    let popup = Rect::new(
        (area.width - popup_width) / 2,
        (area.height - popup_height) / 2,
        popup_width,
        popup_height,
    );
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title("Select Branch")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BLUE));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut list_lines = Vec::new();
    for (i, branch) in app.branches.iter().enumerate() {
        let prefix = if i == app.selected_branch { "> " } else { "  " };
        let style = if i == app.selected_branch {
            Style::default()
                .fg(Theme::BLUE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT)
        };
        list_lines.push(Line::styled(format!("{}{}", prefix, branch), style));
    }

    if list_lines.is_empty() {
        list_lines.push(Line::styled(
            "No branches",
            Style::default().fg(Theme::SUBTEXT),
        ));
    }

    let text = Text::from(list_lines);
    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}

pub fn draw_commit_select_popup(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup_width = 80.min(area.width.saturating_sub(4));
    let popup_height = 25.min(area.height.saturating_sub(4));
    let popup = Rect::new(
        (area.width - popup_width) / 2,
        (area.height - popup_height) / 2,
        popup_width,
        popup_height,
    );
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title("Select Commit")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BLUE));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut list_lines = Vec::new();
    for (i, (hash, message)) in app.commits.iter().enumerate() {
        let prefix = if i == app.selected_commit { "> " } else { "  " };
        let style = if i == app.selected_commit {
            Style::default()
                .fg(Theme::BLUE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT)
        };
        let short_hash = &hash[..7.min(hash.len())];
        list_lines.push(Line::styled(
            format!("{}{}  {}", prefix, short_hash, message),
            style,
        ));
    }

    if list_lines.is_empty() {
        list_lines.push(Line::styled(
            "No commits",
            Style::default().fg(Theme::SUBTEXT),
        ));
    }

    let text = Text::from(list_lines);
    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}

pub fn draw_help_popup(f: &mut Frame, _app: &App) {
    let area = f.area();
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 35.min(area.height.saturating_sub(4));
    let popup = Rect::new(
        (area.width - popup_width) / 2,
        (area.height - popup_height) / 2,
        popup_width,
        popup_height,
    );
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title("Keybindings")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BLUE))
        .style(Style::default().bg(Theme::SURFACE));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines = vec![
        Line::styled(
            "Navigation",
            Style::default()
                .fg(Theme::BLUE)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            "  j/k, ↓/↑    move down/up",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  h/l, ←/→    switch focus / collapse-expand",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  Tab           toggle focus",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  ]/[           next/previous file",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  g/G           top/bottom",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  d/u           full page down/up",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  Ctrl-d/u      half page down/up",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  space/Enter   toggle directory / open file",
            Style::default().fg(Theme::TEXT),
        ),
        Line::from(""),
        Line::styled(
            "Diff",
            Style::default()
                .fg(Theme::BLUE)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            "  j/k           scroll down/up (hunk/line/page set by v)",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  J/K           line down/up",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  y             copy hunk to clipboard",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  Y             copy clean code (no +/- markers)",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  |             toggle side-by-side view",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  v             cycle scroll step (hunk/line/page)",
            Style::default().fg(Theme::TEXT),
        ),
        Line::from(""),
        Line::styled(
            "Search & Compare",
            Style::default()
                .fg(Theme::BLUE)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            "  /             search in diff content",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "                  *.py>pattern to filter by path",
            Style::default().fg(Theme::SUBTEXT),
        ),
        Line::styled(
            "  f             search file by name",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  S             global regex search",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  n/N           next/previous search result",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  b             compare against branch",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  c             compare against commit",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  s             cycle staged/unstaged filter",
            Style::default().fg(Theme::TEXT),
        ),
        Line::from(""),
        Line::styled(
            "General",
            Style::default()
                .fg(Theme::BLUE)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            "  r             refresh / reload files",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  U             toggle untracked files",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "  ?             show this help",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled("  q             quit", Style::default().fg(Theme::TEXT)),
    ];

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}
