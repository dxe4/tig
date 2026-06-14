use crate::app::App;
use crate::app::mode::{Focus, Mode};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_normal_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('d') => {
                if app.focus == Focus::Files {
                    app.half_page_tree_down();
                } else {
                    app.half_page_diff_down();
                }
                app.adjust_scroll();
                return Ok(false);
            }
            KeyCode::Char('u') => {
                if app.focus == Focus::Files {
                    app.half_page_tree_up();
                } else {
                    app.half_page_diff_up();
                }
                app.adjust_scroll();
                return Ok(false);
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('j') | KeyCode::Down => {
            if app.focus == Focus::Files {
                app.next_tree_item();
            } else {
                app.scroll_diff_down();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.focus == Focus::Files {
                app.prev_tree_item();
            } else {
                app.scroll_diff_up();
            }
        }
        KeyCode::Char('J') => {
            if app.focus == Focus::Diff {
                app.move_diff_cursor(1);
            }
        }
        KeyCode::Char('K') => {
            if app.focus == Focus::Diff {
                app.move_diff_cursor(-1);
            }
        }
        KeyCode::Char('h') | KeyCode::Left => {
            if app.focus == Focus::Diff {
                app.focus = Focus::Files;
            } else {
                app.collapse_dir_or_parent();
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if app.focus == Focus::Files
                && let Some(entry) = app.tree_entries.get(app.selected_tree_item)
            {
                if entry.is_dir {
                    app.toggle_dir();
                } else {
                    app.focus = Focus::Diff;
                }
            }
        }
        KeyCode::Char(']') => app.next_file(),
        KeyCode::Char('[') => app.prev_file(),
        KeyCode::Char('d') => {
            if app.focus == Focus::Files {
                app.page_tree_down();
            } else {
                app.page_diff_down();
            }
        }
        KeyCode::Char('u') => {
            if app.focus == Focus::Files {
                app.page_tree_up();
            } else {
                app.page_diff_up();
            }
        }
        KeyCode::Char('g') => {
            if app.focus == Focus::Files {
                app.selected_tree_item = 0;
                app.update_selected_file_from_tree();
            } else {
                app.diff_cursor = 0;
            }
        }
        KeyCode::Char('G') => {
            if app.focus == Focus::Files {
                app.selected_tree_item = app.tree_entries.len().saturating_sub(1);
                app.update_selected_file_from_tree();
            } else {
                let max = app.diff_lines().len().saturating_sub(1);
                app.diff_cursor = max;
            }
        }
        KeyCode::Char('y') => app.copy_hunk()?,
        KeyCode::Char('Y') => app.copy_hunk_clean()?,
        KeyCode::Char(' ') => {
            if app.focus == Focus::Files {
                app.toggle_dir();
            }
        }
        KeyCode::Enter => {
            if app.focus == Focus::Files
                && let Some(entry) = app.tree_entries.get(app.selected_tree_item)
            {
                if entry.is_dir {
                    app.toggle_dir();
                } else {
                    app.focus = Focus::Diff;
                }
            }
        }
        KeyCode::Char('/') => {
            app.mode = Mode::SearchContent;
            app.search_query.clear();
            app.search_results.clear();
        }
        KeyCode::Char('f') => {
            app.mode = Mode::SearchFilename;
            app.search_query.clear();
            app.search_results.clear();
        }
        KeyCode::Char('S') => {
            app.mode = Mode::GlobalSearch;
            app.global_search_query.clear();
            app.global_search_results.clear();
        }
        KeyCode::Char('r') => {
            app.reload_files()?;
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }
        KeyCode::Char('s') => {
            app.cycle_filter();
        }
        KeyCode::Char('b') => {
            if app.compare_branch.is_some() {
                app.clear_compare_branch()?;
            } else if app.compare_range.is_some() {
                app.clear_compare_range()?;
            } else {
                app.enter_branch_select()?;
            }
        }
        KeyCode::Char('c') => {
            if app.compare_commit.is_some() {
                app.clear_compare_commit()?;
            } else if app.compare_range.is_some() {
                app.clear_compare_range()?;
            } else {
                app.enter_commit_select()?;
            }
        }
        KeyCode::Char('|') => {
            app.side_by_side = !app.side_by_side;
            let label = if app.side_by_side {
                "side-by-side"
            } else {
                "unified"
            };
            app.show_message(format!("Diff view: {}", label));
        }
        KeyCode::Char('v') => {
            if app.focus == Focus::Diff {
                app.scroll_step = app.scroll_step.next();
                app.show_message(format!("Scroll step: {}", app.scroll_step.label()));
            }
        }
        KeyCode::Char('U') => {
            app.show_untracked = !app.show_untracked;
            app.reload_files()?;
            let label = if app.show_untracked {
                "shown"
            } else {
                "hidden"
            };
            app.show_message(format!("Untracked files: {}", label));
        }
        KeyCode::Char('n') => app.next_search_result(),
        KeyCode::Char('N') => app.prev_search_result(),
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::Esc => {
            app.search_results.clear();
        }
        _ => {}
    }
    app.adjust_scroll();
    Ok(false)
}

pub fn handle_search_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.search_results.clear();
        }
        KeyCode::Enter => {
            app.execute_search()?;
            app.mode = Mode::Normal;
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.incremental_search()?;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            if app.search_query.is_empty() {
                app.search_results.clear();
            } else {
                app.incremental_search()?;
            }
        }
        _ => {}
    }
    Ok(false)
}

pub fn handle_global_search_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.global_search_results.clear();
        }
        KeyCode::Enter => {
            app.jump_to_global_result();
            app.mode = Mode::Normal;
            app.global_search_results.clear();
        }
        KeyCode::Char('j') | KeyCode::Down => app.next_global_result(),
        KeyCode::Char('k') | KeyCode::Up => app.prev_global_result(),
        KeyCode::Char(c) => {
            app.global_search_query.push(c);
            app.incremental_global_search();
        }
        KeyCode::Backspace => {
            app.global_search_query.pop();
            if app.global_search_query.is_empty() {
                app.global_search_results.clear();
            } else {
                app.incremental_global_search();
            }
        }
        _ => {}
    }
    Ok(false)
}

pub fn handle_branch_select_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if let Some(branch) = app.branches.get(app.selected_branch) {
                let branch = branch.clone();
                app.set_compare_branch(&branch)?;
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.branches.is_empty() {
                app.selected_branch = (app.selected_branch + 1) % app.branches.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.branches.is_empty() {
                if app.selected_branch == 0 {
                    app.selected_branch = app.branches.len() - 1;
                } else {
                    app.selected_branch -= 1;
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

pub fn handle_commit_select_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if let Some((hash, _)) = app.commits.get(app.selected_commit) {
                let hash = hash.clone();
                app.set_compare_commit(&hash)?;
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.commits.is_empty() {
                app.selected_commit = (app.selected_commit + 1) % app.commits.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.commits.is_empty() {
                if app.selected_commit == 0 {
                    app.selected_commit = app.commits.len() - 1;
                } else {
                    app.selected_commit -= 1;
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

pub fn handle_help_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Normal;
        }
        _ => {}
    }
    Ok(false)
}
