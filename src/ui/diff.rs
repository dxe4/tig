use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::{App, Focus};
use crate::git::LineType;
use crate::ui::theme::Theme;
use regex::Regex;

pub fn draw_diff(f: &mut Frame, app: &mut App, area: Rect) {
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

pub fn draw_side_by_side(f: &mut Frame, app: &mut App, area: Rect, title: &str) {
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

    // Determine which visible row corresponds to the current diff cursor.
    let cursor_row = scroll + app.diff_cursor;

    let mut left_lines = Vec::new();
    let mut right_lines = Vec::new();

    for (row_idx, row) in rows.iter().skip(scroll).take(visible_height).enumerate() {
        let global_idx = scroll + row_idx;
        let is_cursor = global_idx == cursor_row && app.focus == Focus::Diff;

        let left_fg = row.left_type.map_or(Theme::SUBTEXT, line_color);
        let right_fg = row.right_type.map_or(Theme::SUBTEXT, line_color);
        let left_style = if is_cursor {
            Style::default().fg(left_fg).bg(Theme::CURSOR)
        } else {
            Style::default().fg(left_fg)
        };
        let right_style = if is_cursor {
            Style::default().fg(right_fg).bg(Theme::CURSOR)
        } else {
            Style::default().fg(right_fg)
        };
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

    let prefix_bytes: usize = a
        .chars()
        .zip(b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .map(|(c, _)| c.len_utf8())
        .sum();

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
