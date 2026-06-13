use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::app::{App, Mode};

pub mod diff;
pub mod files;
pub mod popups;
pub mod status_bar;
pub mod theme;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .split(chunks[0]);

    files::draw_files(f, app, main_chunks[0]);
    diff::draw_diff(f, app, main_chunks[1]);

    status_bar::draw_status_bar(f, app, chunks[1]);
    match app.mode {
        Mode::SearchContent | Mode::SearchFilename => popups::draw_search_popup(f, app),
        Mode::GlobalSearch => popups::draw_global_search_popup(f, app),
        Mode::BranchSelect => popups::draw_branch_select_popup(f, app),
        Mode::CommitSelect => popups::draw_commit_select_popup(f, app),
        Mode::Help => popups::draw_help_popup(f, app),
        _ => {}
    }
}
