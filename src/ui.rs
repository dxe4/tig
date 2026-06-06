use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::app::{App, Focus, Mode};
use crate::git::LineType;
use regex::Regex;

// Catppuccin Mocha-inspired palette
struct Theme;
impl Theme {
    const TEXT: Color = Color::Rgb(205, 214, 244);
    const SUBTEXT: Color = Color::Rgb(166, 173, 200);
    const GREEN: Color = Color::Rgb(166, 227, 161);
    const RED: Color = Color::Rgb(243, 139, 168);
    const BLUE: Color = Color::Rgb(137, 180, 250);
    const YELLOW: Color = Color::Rgb(249, 226, 175);
    const MAUVE: Color = Color::Rgb(203, 166, 247);
    const TEAL: Color = Color::Rgb(148, 226, 213);
    const SURFACE: Color = Color::Rgb(49, 50, 68);
    const OVERLAY: Color = Color::Rgb(69, 71, 90);
    const BASE: Color = Color::Rgb(30, 30, 46);
    const CURSOR: Color = Color::Rgb(88, 91, 112);
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .split(chunks[0]);

    draw_files(f, app, main_chunks[0]);
    draw_diff(f, app, main_chunks[1]);

    draw_status_bar(f, app, chunks[1]);
    match app.mode {
        Mode::SearchContent | Mode::SearchFilename => draw_search_popup(f, app),
        Mode::GlobalSearch => draw_global_search_popup(f, app),
        Mode::BranchSelect => draw_branch_select_popup(f, app),
        Mode::CommitSelect => draw_commit_select_popup(f, app),
        Mode::Help => draw_help_popup(f, app),
        _ => {}
    }
}

fn draw_files(f: &mut Frame, app: &mut App, area: Rect) {
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

fn draw_diff(f: &mut Frame, app: &mut App, area: Rect) {
    let diff_lines = app.diff_lines();
    app.diff_visible_height = area.height.saturating_sub(2) as usize;

    let hunk_count = if app.files.is_empty() {
        0
    } else {
        app.files[app.selected_file].hunks.len()
    };

    let cursor = app.diff_cursor;
    let current_hunk_num = if diff_lines.is_empty() || cursor >= diff_lines.len() {
        0
    } else {
        diff_lines[cursor].hunk_index + 1
    };

    let file_name = if app.files.is_empty() {
        String::new()
    } else {
        app.files[app.selected_file].path.clone()
    };

    let mut title = if let Some(ref commit) = app.compare_commit {
        format!(
            "{} @ {}  hunk {}/{}",
            file_name,
            &commit[..7.min(commit.len())],
            current_hunk_num,
            hunk_count
        )
    } else if let Some(ref branch) = app.compare_branch {
        format!(
            "{} vs {}  hunk {}/{}",
            file_name, branch, current_hunk_num, hunk_count
        )
    } else {
        format!("{}  hunk {}/{}", file_name, current_hunk_num, hunk_count)
    };

    let has_more_above = app.diff_scroll > 0;
    let has_more_below = app.diff_scroll + app.diff_visible_height < diff_lines.len();
    if has_more_above {
        title = format!("↑ {title}");
    }
    if has_more_below {
        title = format!("{title} ↓");
    }

    if diff_lines.is_empty() {
        let paragraph = Paragraph::new("No changes to display")
            .style(Style::default().fg(Theme::SUBTEXT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(if app.focus == Focus::Diff {
                        Style::default().fg(Theme::BLUE)
                    } else {
                        Style::default().fg(Theme::OVERLAY)
                    })
                    .title(Span::styled(title, Style::default().fg(Theme::TEXT))),
            );
        f.render_widget(paragraph, area);
        return;
    }

    if app.side_by_side {
        draw_side_by_side(f, app, area, &title);
        return;
    }

    let scroll = app.diff_scroll;
    let visible_height = app.diff_visible_height;

    let cursor_hunk = if cursor < diff_lines.len() {
        diff_lines[cursor].hunk_index
    } else {
        0
    };

    let highlight_regex = if app.last_search_query.is_empty() {
        None
    } else {
        Regex::new(&format!("(?i){}", regex::escape(&app.last_search_query))).ok()
    };

    let mut text_lines = Vec::new();
    for (i, line) in diff_lines
        .iter()
        .skip(scroll)
        .take(visible_height)
        .enumerate()
    {
        let global_idx = scroll + i;
        let fg = line_color(line.line_type);

        let is_cursor = global_idx == cursor && app.focus == Focus::Diff;
        let in_hunk = line.hunk_index == cursor_hunk && app.focus == Focus::Diff;

        let bg = if is_cursor {
            Theme::CURSOR
        } else if in_hunk {
            Theme::SURFACE
        } else {
            Color::Reset
        };

        let mut base_style = Style::default().fg(fg).bg(bg);
        if line.line_type == LineType::Header || is_cursor {
            base_style = base_style.add_modifier(Modifier::BOLD);
        }

        if let Some(ref re) = highlight_regex
            && re.is_match(&line.content)
        {
            let match_style = Style::default().fg(Theme::BASE).bg(Theme::YELLOW);
            text_lines.push(highlight_line(&line.content, re, base_style, match_style));
        } else if (line.line_type == LineType::Added || line.line_type == LineType::Removed)
            && global_idx > 0
            && global_idx < diff_lines.len()
        {
            let other = if line.line_type == LineType::Added {
                diff_lines.get(global_idx.saturating_sub(1))
            } else {
                diff_lines.get(global_idx + 1)
            };
            if let Some(other_line) = other
                && ((line.line_type == LineType::Added
                    && other_line.line_type == LineType::Removed)
                    || (line.line_type == LineType::Removed
                        && other_line.line_type == LineType::Added))
            {
                text_lines.push(word_diff_line(
                    &line.content,
                    &other_line.content,
                    base_style,
                    line.line_type,
                ));
            } else {
                text_lines.push(Line::styled(line.content.clone(), base_style));
            }
        } else {
            text_lines.push(Line::styled(line.content.clone(), base_style));
        }
    }

    let text = Text::from(text_lines);
    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if app.focus == Focus::Diff {
                Style::default().fg(Theme::BLUE)
            } else {
                Style::default().fg(Theme::OVERLAY)
            })
            .title(Span::styled(title, Style::default().fg(Theme::TEXT))),
    );

    f.render_widget(paragraph, area);
}

fn draw_side_by_side(f: &mut Frame, app: &mut App, area: Rect, title: &str) {
    let hunks: &[crate::git::Hunk] = if app.files.is_empty() {
        &[]
    } else {
        &app.files[app.selected_file].hunks
    };

    #[derive(Clone)]
    struct Row {
        left: String,
        right: String,
        left_type: Option<LineType>,
        right_type: Option<LineType>,
    }

    let mut rows = Vec::new();
    for hunk in hunks {
        let mut pending_removed: Vec<String> = Vec::new();
        for line in &hunk.lines {
            match line.line_type {
                LineType::Context => {
                    for rem in pending_removed.drain(..) {
                        rows.push(Row {
                            left: rem,
                            right: String::new(),
                            left_type: Some(LineType::Removed),
                            right_type: None,
                        });
                    }
                    rows.push(Row {
                        left: line.content.clone(),
                        right: line.content.clone(),
                        left_type: Some(LineType::Context),
                        right_type: Some(LineType::Context),
                    });
                }
                LineType::Removed => {
                    pending_removed.push(line.content.clone());
                }
                LineType::Added => {
                    if let Some(rem) = pending_removed.pop() {
                        rows.push(Row {
                            left: rem,
                            right: line.content.clone(),
                            left_type: Some(LineType::Removed),
                            right_type: Some(LineType::Added),
                        });
                    } else {
                        rows.push(Row {
                            left: String::new(),
                            right: line.content.clone(),
                            left_type: None,
                            right_type: Some(LineType::Added),
                        });
                    }
                }
                _ => {}
            }
        }
        for rem in pending_removed.drain(..) {
            rows.push(Row {
                left: rem,
                right: String::new(),
                left_type: Some(LineType::Removed),
                right_type: None,
            });
        }
    }

    app.diff_visible_height = area.height.saturating_sub(2) as usize;
    let scroll = app.diff_scroll;
    let visible_height = app.diff_visible_height;

    let mut left_lines = Vec::new();
    let mut right_lines = Vec::new();

    for row in rows.iter().skip(scroll).take(visible_height) {
        let left_fg = row.left_type.map_or(Theme::SUBTEXT, line_color);
        let right_fg = row.right_type.map_or(Theme::SUBTEXT, line_color);
        let left_style = Style::default().fg(left_fg);
        let right_style = Style::default().fg(right_fg);
        left_lines.push(Line::styled(row.left.clone(), left_style));
        right_lines.push(Line::styled(row.right.clone(), right_style));
    }

    let wrapper = Block::default()
        .title(Span::styled(title, Style::default().fg(Theme::TEXT)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if app.focus == Focus::Diff {
            Style::default().fg(Theme::BLUE)
        } else {
            Style::default().fg(Theme::OVERLAY)
        });
    let inner = wrapper.inner(area);
    f.render_widget(wrapper, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let left_block = Block::default()
        .title("Original")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::OVERLAY));
    let right_block = Block::default()
        .title("Modified")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::OVERLAY));

    let left_para = Paragraph::new(Text::from(left_lines)).block(left_block);
    let right_para = Paragraph::new(Text::from(right_lines)).block(right_block);
    f.render_widget(left_para, cols[0]);
    f.render_widget(right_para, cols[1]);
}

fn highlight_line(
    content: &str,
    regex: &Regex,
    base_style: Style,
    match_style: Style,
) -> Line<'static> {
    let mut spans = Vec::new();
    let mut last_end = 0;

    for m in regex.find_iter(content) {
        if m.start() > last_end {
            spans.push(Span::styled(
                content[last_end..m.start()].to_string(),
                base_style,
            ));
        }
        spans.push(Span::styled(
            content[m.start()..m.end()].to_string(),
            match_style,
        ));
        last_end = m.end();
    }

    if last_end < content.len() {
        spans.push(Span::styled(content[last_end..].to_string(), base_style));
    }

    Line::from(spans)
}

fn word_diff_line(
    content: &str,
    other: &str,
    base_style: Style,
    line_type: LineType,
) -> Line<'static> {
    let marker_len = if content.starts_with('+') || content.starts_with('-') {
        1
    } else {
        0
    };
    let other_marker_len = if other.starts_with('+') || other.starts_with('-') {
        1
    } else {
        0
    };

    let a = &content[marker_len..];
    let b = &other[other_marker_len..];

    // Common prefix in bytes
    let prefix_bytes: usize = a
        .chars()
        .zip(b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .map(|(c, _)| c.len_utf8())
        .sum();

    // Common suffix in bytes (avoid overlap with prefix)
    let a_rev: Vec<char> = a.chars().rev().collect();
    let b_rev: Vec<char> = b.chars().rev().collect();
    let suffix_bytes: usize = a_rev
        .iter()
        .zip(b_rev.iter())
        .take_while(|(ca, cb)| ca == cb)
        .take(a_rev.len().saturating_sub(prefix_bytes))
        .map(|(c, _)| c.len_utf8())
        .sum();

    if prefix_bytes == 0 && suffix_bytes == 0 {
        return Line::styled(content.to_string(), base_style);
    }

    let unchanged_mid_start = marker_len + prefix_bytes;
    let unchanged_mid_end = marker_len + a.len() - suffix_bytes;

    let mut spans = Vec::new();
    spans.push(Span::styled(
        content[..unchanged_mid_start].to_string(),
        base_style,
    ));

    if unchanged_mid_start < unchanged_mid_end {
        let highlight_fg = if line_type == LineType::Added {
            Color::Rgb(20, 60, 20)
        } else {
            Color::Rgb(60, 20, 20)
        };
        let highlight_bg = if line_type == LineType::Added {
            Color::Rgb(100, 220, 100)
        } else {
            Color::Rgb(220, 100, 100)
        };
        let highlight_style = Style::default()
            .fg(highlight_fg)
            .bg(highlight_bg)
            .add_modifier(Modifier::BOLD);
        spans.push(Span::styled(
            content[unchanged_mid_start..unchanged_mid_end].to_string(),
            highlight_style,
        ));
    }

    if suffix_bytes > 0 && unchanged_mid_end <= content.len() {
        spans.push(Span::styled(
            content[unchanged_mid_end..].to_string(),
            base_style,
        ));
    }

    Line::from(spans)
}

fn line_color(line_type: LineType) -> Color {
    match line_type {
        LineType::Added => Theme::GREEN,
        LineType::Removed => Theme::RED,
        LineType::Header => Theme::TEAL,
        LineType::Context => Theme::SUBTEXT,
        LineType::NoNewline => Theme::OVERLAY,
    }
}

fn draw_search_popup(f: &mut Frame, app: &App) {
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

fn draw_global_search_popup(f: &mut Frame, app: &App) {
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

    // Search input
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

    // Results list
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

    // Status line
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

    // Cursor
    let cursor_x = inner[0].x
        + 1
        + app
            .global_search_query
            .len()
            .min((inner[0].width.saturating_sub(2)) as usize) as u16;
    let cursor_y = inner[0].y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn draw_branch_select_popup(f: &mut Frame, app: &App) {
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

fn draw_commit_select_popup(f: &mut Frame, app: &App) {
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

fn draw_help_popup(f: &mut Frame, _app: &App) {
    let area = f.area();
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 28.min(area.height.saturating_sub(4));
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
            "  d/u           page down/up",
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
            "  j/k           next/previous hunk",
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
            "  ?             show this help",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled("  q             quit", Style::default().fg(Theme::TEXT)),
    ];

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let text = app.status_text();
    let paragraph =
        Paragraph::new(text).style(Style::default().bg(Theme::SURFACE).fg(Theme::SUBTEXT));
    f.render_widget(paragraph, area);
}
