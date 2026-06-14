use crate::app::App;
use crate::app::mode::Mode;
use crate::git::{
    get_branch_files, get_branches, get_changed_files, get_commit_files, get_commits,
    get_range_files,
};
use anyhow::Result;

pub fn enter_branch_select(app: &mut App) -> Result<()> {
    app.branches = get_branches()?;
    app.selected_branch = 0;
    if app.branches.is_empty() {
        app.show_message("No branches found".to_string());
    } else {
        app.mode = Mode::BranchSelect;
    }
    Ok(())
}

pub fn set_compare_branch(app: &mut App, branch: &str) -> Result<()> {
    app.compare_commit = None;
    app.compare_range = None;
    app.files = get_branch_files(branch)?;
    app.compare_branch = Some(branch.to_string());
    app.invalidate_diff_cache();
    app.reset_view_to_files();
    app.show_message(format!("Comparing against: {}", branch));
    Ok(())
}

pub fn clear_compare_branch(app: &mut App) -> Result<()> {
    app.compare_branch = None;
    app.compare_range = None;
    app.files = get_changed_files(app.show_untracked)?;
    app.invalidate_diff_cache();
    app.reset_view_to_files();
    app.show_message("Cleared branch comparison".to_string());
    Ok(())
}

pub fn enter_commit_select(app: &mut App) -> Result<()> {
    app.commits = get_commits(50)?;
    app.selected_commit = 0;
    if app.commits.is_empty() {
        app.show_message("No commits found".to_string());
    } else {
        app.mode = Mode::CommitSelect;
    }
    Ok(())
}

pub fn set_compare_commit(app: &mut App, commit: &str) -> Result<()> {
    app.compare_branch = None;
    app.compare_range = None;
    app.files = get_commit_files(commit)?;
    app.compare_commit = Some(commit.to_string());
    app.invalidate_diff_cache();
    app.reset_view_to_files();
    app.show_message(format!("Showing commit: {}", commit));
    Ok(())
}

pub fn clear_compare_commit(app: &mut App) -> Result<()> {
    app.compare_commit = None;
    app.compare_range = None;
    app.files = get_changed_files(app.show_untracked)?;
    app.invalidate_diff_cache();
    app.reset_view_to_files();
    app.show_message("Cleared commit view".to_string());
    Ok(())
}

pub fn set_compare_range(app: &mut App, left: &str, right: &str) -> Result<()> {
    app.compare_branch = None;
    app.compare_commit = None;
    app.files = get_range_files(left, right)?;
    app.compare_range = Some((left.to_string(), right.to_string()));
    app.invalidate_diff_cache();
    app.reset_view_to_files();
    app.show_message(format!("Comparing range: {}..{}", left, right));
    Ok(())
}

pub fn clear_compare_range(app: &mut App) -> Result<()> {
    app.compare_range = None;
    app.files = get_changed_files(app.show_untracked)?;
    app.invalidate_diff_cache();
    app.reset_view_to_files();
    app.show_message("Cleared range comparison".to_string());
    Ok(())
}
