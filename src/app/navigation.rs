use crate::app::App;
use crate::app::mode::ScrollStep;
use crate::git::LineType;

pub fn next_tree_item(app: &mut App) {
    if app.tree_entries.is_empty() {
        return;
    }
    app.selected_tree_item = (app.selected_tree_item + 1).min(app.tree_entries.len() - 1);
    update_selected_file_from_tree(app);
    app.diff_cursor = 0;
    app.diff_scroll = 0;
}

pub fn prev_tree_item(app: &mut App) {
    app.selected_tree_item = app.selected_tree_item.saturating_sub(1);
    update_selected_file_from_tree(app);
    app.diff_cursor = 0;
    app.diff_scroll = 0;
}

pub fn update_selected_file_from_tree(app: &mut App) {
    if let Some(entry) = app.tree_entries.get(app.selected_tree_item)
        && let Some(idx) = entry.file_index
    {
        app.selected_file = idx;
    }
}

pub fn next_file(app: &mut App) {
    if app.files.is_empty() || app.tree_entries.is_empty() {
        return;
    }
    loop {
        if app.selected_tree_item >= app.tree_entries.len() - 1 {
            break;
        }
        app.selected_tree_item += 1;
        if app.tree_entries[app.selected_tree_item]
            .file_index
            .is_some()
        {
            break;
        }
    }
    update_selected_file_from_tree(app);
    app.diff_cursor = 0;
    app.diff_scroll = 0;
}

pub fn prev_file(app: &mut App) {
    if app.files.is_empty() || app.tree_entries.is_empty() {
        return;
    }
    loop {
        if app.selected_tree_item == 0 {
            break;
        }
        app.selected_tree_item -= 1;
        if app.tree_entries[app.selected_tree_item]
            .file_index
            .is_some()
        {
            break;
        }
    }
    update_selected_file_from_tree(app);
    app.diff_cursor = 0;
    app.diff_scroll = 0;
}

pub fn toggle_dir(app: &mut App) {
    if let Some(entry) = app.tree_entries.get(app.selected_tree_item)
        && entry.is_dir
    {
        let path = entry.full_path.clone();
        if app.expanded_dirs.contains(&path) {
            app.expanded_dirs.remove(&path);
        } else {
            app.expanded_dirs.insert(path);
        }
        app.rebuild_tree();
        app.selected_tree_item = app
            .selected_tree_item
            .min(app.tree_entries.len().saturating_sub(1));
    }
}

pub fn collapse_dir_or_parent(app: &mut App) {
    let Some(entry) = app.tree_entries.get(app.selected_tree_item) else {
        return;
    };
    if entry.is_dir && app.expanded_dirs.contains(&entry.full_path) {
        app.expanded_dirs.remove(&entry.full_path);
        app.rebuild_tree();
        return;
    }
    let target_depth = entry.depth.saturating_sub(1);
    for i in (0..app.selected_tree_item).rev() {
        if app.tree_entries[i].depth == target_depth && app.tree_entries[i].is_dir {
            app.expanded_dirs.remove(&app.tree_entries[i].full_path);
            app.selected_tree_item = i;
            app.rebuild_tree();
            return;
        }
    }
}

pub fn move_diff_cursor(app: &mut App, delta: i32) {
    let max = app.diff_lines().len().saturating_sub(1);
    if delta > 0 {
        app.diff_cursor = (app.diff_cursor + delta as usize).min(max);
    } else {
        app.diff_cursor = app.diff_cursor.saturating_sub((-delta) as usize);
    }
}

pub fn page_diff_down(app: &mut App) {
    let delta = app.diff_visible_height.saturating_sub(2).max(1);
    move_diff_cursor(app, delta as i32);
}

pub fn page_diff_up(app: &mut App) {
    let delta = app.diff_visible_height.saturating_sub(2).max(1);
    move_diff_cursor(app, -(delta as i32));
}

pub fn half_page_diff_down(app: &mut App) {
    let delta = app.diff_visible_height / 2;
    move_diff_cursor(app, delta.max(1) as i32);
}

pub fn half_page_diff_up(app: &mut App) {
    let delta = app.diff_visible_height / 2;
    move_diff_cursor(app, -(delta.max(1) as i32));
}

pub fn page_tree_down(app: &mut App) {
    let delta = app.tree_visible_height.saturating_sub(2).max(1);
    app.selected_tree_item =
        (app.selected_tree_item + delta).min(app.tree_entries.len().saturating_sub(1));
    update_selected_file_from_tree(app);
    app.diff_cursor = 0;
    app.diff_scroll = 0;
}

pub fn page_tree_up(app: &mut App) {
    let delta = app.tree_visible_height.saturating_sub(2).max(1);
    app.selected_tree_item = app.selected_tree_item.saturating_sub(delta);
    update_selected_file_from_tree(app);
    app.diff_cursor = 0;
    app.diff_scroll = 0;
}

pub fn half_page_tree_down(app: &mut App) {
    let delta = app.tree_visible_height / 2;
    app.selected_tree_item =
        (app.selected_tree_item + delta.max(1)).min(app.tree_entries.len().saturating_sub(1));
    update_selected_file_from_tree(app);
    app.diff_cursor = 0;
    app.diff_scroll = 0;
}

pub fn half_page_tree_up(app: &mut App) {
    let delta = app.tree_visible_height / 2;
    app.selected_tree_item = app.selected_tree_item.saturating_sub(delta.max(1));
    update_selected_file_from_tree(app);
    app.diff_cursor = 0;
    app.diff_scroll = 0;
}

pub fn next_hunk(app: &mut App) -> bool {
    let lines = app.diff_lines();
    if lines.is_empty() {
        return false;
    }
    let cursor = app.diff_cursor;
    let current_hunk = lines.get(cursor).map(|l| l.hunk_index).unwrap_or(0);

    let mut header_idx = None;
    for (i, line) in lines.iter().enumerate().skip(cursor + 1) {
        if line.line_type == LineType::Header && line.hunk_index > current_hunk {
            header_idx = Some(i);
            break;
        }
    }

    let Some(header_idx) = header_idx else {
        return false;
    };
    for (i, line) in lines.iter().enumerate().skip(header_idx + 1) {
        if line.line_type != LineType::Header {
            app.diff_cursor = i;
            return true;
        }
    }
    app.diff_cursor = header_idx;
    true
}

pub fn prev_hunk(app: &mut App) -> bool {
    let lines = app.diff_lines();
    if lines.is_empty() {
        return false;
    }
    let cursor = app.diff_cursor;
    let current_hunk = lines.get(cursor).map(|l| l.hunk_index).unwrap_or(0);
    if current_hunk == 0 {
        return false;
    }
    let target_hunk = current_hunk - 1;
    let mut header_idx = None;
    for (i, line) in lines.iter().enumerate().take(cursor) {
        if line.line_type == LineType::Header && line.hunk_index == target_hunk {
            header_idx = Some(i);
        }
    }

    let Some(header_idx) = header_idx else {
        return false;
    };
    for (i, line) in lines.iter().enumerate().skip(header_idx + 1) {
        if line.line_type != LineType::Header {
            app.diff_cursor = i;
            return true;
        }
    }
    app.diff_cursor = header_idx;
    true
}

pub fn prev_file_last_hunk(app: &mut App) {
    prev_file(app);
    let lines = app.diff_lines();
    if lines.is_empty() {
        return;
    }
    let last_hunk = lines.last().map(|l| l.hunk_index).unwrap_or(0);
    for (i, line) in lines.iter().enumerate().rev() {
        if line.hunk_index == last_hunk && line.line_type != LineType::Header {
            app.diff_cursor = i;
            return;
        }
    }
    for (i, line) in lines.iter().enumerate().rev() {
        if line.hunk_index == last_hunk && line.line_type == LineType::Header {
            app.diff_cursor = i;
            return;
        }
    }
}

pub fn adjust_scroll(app: &mut App) {
    if app.diff_visible_height == 0 {
        return;
    }
    let half = app.diff_visible_height / 2;
    let target = app.diff_cursor.saturating_sub(half);
    let max_scroll = app
        .diff_lines()
        .len()
        .saturating_sub(app.diff_visible_height);
    app.diff_scroll = target.min(max_scroll);
}

pub fn scroll_diff_down(app: &mut App) {
    match app.scroll_step {
        ScrollStep::Hunk => {
            if !next_hunk(app) {
                next_file(app);
            }
        }
        ScrollStep::Line => move_diff_cursor(app, 1),
        ScrollStep::Page => page_diff_down(app),
    }
}

pub fn scroll_diff_up(app: &mut App) {
    match app.scroll_step {
        ScrollStep::Hunk => {
            if !prev_hunk(app) {
                prev_file_last_hunk(app);
            }
        }
        ScrollStep::Line => move_diff_cursor(app, -1),
        ScrollStep::Page => page_diff_up(app),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{FileChange, Hunk, HunkLine, LineType};

    fn file_with_path(path: &str) -> FileChange {
        FileChange {
            path: path.to_string(),
            status: 'M',
            staged_status: None,
            unstaged_status: Some('M'),
            hunks: vec![Hunk {
                header: "@@ -1,1 +1,1 @@".to_string(),
                lines: vec![HunkLine {
                    content: " context".to_string(),
                    line_type: LineType::Context,
                }],
            }],
            is_binary: false,
        }
    }

    fn app_with_files(files: Vec<FileChange>) -> App {
        App::test_new(files)
    }

    #[test]
    fn next_file_skips_directories() {
        let files = vec![file_with_path("src/a.rs"), file_with_path("src/b.rs")];
        let mut app = app_with_files(files);
        app.expanded_dirs.insert("src".to_string());
        app.rebuild_tree();
        // Tree is: src (dir), a.rs, b.rs
        app.selected_tree_item = 0; // on directory
        next_file(&mut app);
        assert_eq!(app.tree_entries[app.selected_tree_item].name, "a.rs");
    }

    #[test]
    fn prev_file_stops_at_first() {
        let files = vec![file_with_path("src/a.rs"), file_with_path("src/b.rs")];
        let mut app = app_with_files(files);
        app.expanded_dirs.insert("src".to_string());
        app.rebuild_tree();
        app.selected_tree_item = app.tree_entries.len() - 1;
        prev_file(&mut app);
        assert_eq!(app.tree_entries[app.selected_tree_item].name, "a.rs");
    }

    #[test]
    fn move_diff_cursor_respects_bounds() {
        let files = vec![file_with_path("x.rs")];
        let mut app = app_with_files(files);
        app.diff_visible_height = 10;
        let max = app.diff_lines().len().saturating_sub(1);
        move_diff_cursor(&mut app, 100);
        assert_eq!(app.diff_cursor, max);
    }
}
