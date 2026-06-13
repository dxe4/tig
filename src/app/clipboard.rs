use crate::app::App;
use crate::git::LineType;
use anyhow::Result;

pub fn copy_hunk(app: &mut App) -> Result<()> {
    if app.files.is_empty() {
        return Ok(());
    }
    let lines = app.diff_lines();
    if lines.is_empty() {
        return Ok(());
    }
    let cursor = app.diff_cursor.min(lines.len() - 1);
    let hunk_idx = lines[cursor].hunk_index;

    let file = &app.files[app.selected_file];
    let Some(hunk) = file.hunks.get(hunk_idx) else {
        return Ok(());
    };
    let mut text = String::new();
    text.push_str(&hunk.header);
    text.push('\n');
    for line in &hunk.lines {
        text.push_str(&line.content);
        text.push('\n');
    }
    if let Some(ref mut cb) = app.clipboard {
        cb.set_text(text)?;
        app.show_message(String::from("Hunk copied to clipboard"));
    } else {
        app.show_message(String::from("Clipboard not available"));
    }
    Ok(())
}

pub fn copy_hunk_clean(app: &mut App) -> Result<()> {
    if app.files.is_empty() {
        return Ok(());
    }
    let lines = app.diff_lines();
    if lines.is_empty() {
        return Ok(());
    }
    let cursor = app.diff_cursor.min(lines.len() - 1);
    let hunk_idx = lines[cursor].hunk_index;

    let file = &app.files[app.selected_file];
    let Some(hunk) = file.hunks.get(hunk_idx) else {
        return Ok(());
    };
    let mut text = String::new();
    for line in &hunk.lines {
        let stripped = match line.line_type {
            LineType::Added | LineType::Removed | LineType::Context => {
                line.content.get(1..).unwrap_or(&line.content)
            }
            LineType::Header | LineType::NoNewline => continue,
        };
        text.push_str(stripped);
        text.push('\n');
    }
    if let Some(ref mut cb) = app.clipboard {
        cb.set_text(text)?;
        app.show_message(String::from("Code copied to clipboard"));
    } else {
        app.show_message(String::from("Clipboard not available"));
    }
    Ok(())
}
