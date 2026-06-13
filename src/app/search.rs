use crate::app::App;
use crate::app::mode::{Focus, Mode};
use crate::git::{FileChange, get_diff};
use crate::search::{global_search, search_content_filtered, search_filename};
use anyhow::Result;

pub fn parse_search_query(query: &str) -> (Option<&str>, &str) {
    if let Some(pos) = query.find('>') {
        let filter = query[..pos].trim();
        let content = query[pos + 1..].trim();
        let filter_opt = if filter.is_empty() {
            None
        } else {
            Some(filter)
        };
        (filter_opt, content)
    } else {
        (None, query)
    }
}

pub fn execute_search(app: &mut App) -> Result<()> {
    app.perform_search(true)
}

pub fn incremental_search(app: &mut App) -> Result<()> {
    app.perform_search(false)
}

pub fn perform_search(app: &mut App, show_no_results_msg: bool) -> Result<()> {
    if app.search_query.is_empty() {
        app.search_results.clear();
        return Ok(());
    }
    app.search_results = match app.mode {
        Mode::SearchContent => {
            let (filter, content) = parse_search_query(&app.search_query);
            if content.is_empty() {
                app.last_search_query.clear();
                Vec::new()
            } else {
                app.last_search_query = content.to_string();
                search_content_filtered(&app.files, content, filter)
            }
        }
        Mode::SearchFilename => {
            app.last_search_query = app.search_query.clone();
            search_filename(&app.files, &app.search_query)
        }
        _ => Vec::new(),
    };
    app.selected_search_result = 0;
    if !app.search_results.is_empty() {
        jump_to_search_result(app, false);
    } else if show_no_results_msg {
        app.show_message(String::from("No results found"));
    }
    Ok(())
}

pub fn next_search_result(app: &mut App) {
    if app.search_results.is_empty() {
        return;
    }
    app.selected_search_result = (app.selected_search_result + 1) % app.search_results.len();
    jump_to_search_result(app, true);
}

pub fn prev_search_result(app: &mut App) {
    if app.search_results.is_empty() {
        return;
    }
    if app.selected_search_result == 0 {
        app.selected_search_result = app.search_results.len() - 1;
    } else {
        app.selected_search_result -= 1;
    }
    jump_to_search_result(app, true);
}

pub fn jump_to_search_result(app: &mut App, change_focus: bool) {
    if let Some(result) = app.search_results.get(app.selected_search_result)
        && result.file_index < app.files.len()
    {
        app.selected_file = result.file_index;
        for (i, entry) in app.tree_entries.iter().enumerate() {
            if entry.file_index == Some(result.file_index) {
                app.selected_tree_item = i;
                break;
            }
        }
        if let Some(line_idx) = result.line_number {
            app.diff_cursor = line_idx;
        } else {
            app.diff_cursor = 0;
        }
        app.adjust_scroll();
        if change_focus {
            app.focus = Focus::Diff;
        }
    }
}

pub fn incremental_global_search(app: &mut App) {
    if app.global_search_query.is_empty() {
        app.global_search_results.clear();
        return;
    }
    match global_search(&app.global_search_query) {
        Ok(results) => {
            app.global_search_results = results;
            app.global_search_selected = 0;
        }
        Err(e) => {
            app.show_message(format!("Regex error: {}", e));
        }
    }
}

pub fn next_global_result(app: &mut App) {
    if app.global_search_results.is_empty() {
        return;
    }
    app.global_search_selected = (app.global_search_selected + 1) % app.global_search_results.len();
}

pub fn prev_global_result(app: &mut App) {
    if app.global_search_results.is_empty() {
        return;
    }
    if app.global_search_selected == 0 {
        app.global_search_selected = app.global_search_results.len() - 1;
    } else {
        app.global_search_selected -= 1;
    }
}

pub fn jump_to_global_result(app: &mut App) {
    let (path, _line_number) = {
        let Some(result) = app.global_search_results.get(app.global_search_selected) else {
            return;
        };
        (result.file_path.clone(), result.line_number)
    };

    let file_idx = match app.files.iter().position(|f| f.path == path) {
        Some(idx) => idx,
        None => {
            let Ok((hunks, is_binary)) = get_diff(&path) else {
                app.show_message(format!("Cannot read {}", path));
                return;
            };
            let status = if std::fs::metadata(&path).is_ok() {
                ' '
            } else {
                'D'
            };
            app.files.push(FileChange {
                path: path.clone(),
                status,
                staged_status: None,
                unstaged_status: None,
                hunks,
                is_binary,
            });
            app.files.sort_by(|a, b| a.path.cmp(&b.path));
            app.invalidate_diff_cache();
            app.rebuild_tree();
            app.files.iter().position(|f| f.path == path).unwrap_or(0)
        }
    };

    app.selected_file = file_idx;
    for (i, entry) in app.tree_entries.iter().enumerate() {
        if entry.file_index == Some(file_idx) {
            app.selected_tree_item = i;
            break;
        }
    }

    app.diff_cursor = 0;
    app.diff_scroll = 0;
    app.focus = Focus::Diff;
}
