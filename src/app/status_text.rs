use crate::app::App;
use crate::app::mode::{Filter, Focus, Mode};

pub fn status_text(app: &App) -> String {
    match app.mode {
        Mode::BranchSelect => {
            return String::from("j/k:nav  Enter:select  Esc:cancel");
        }
        Mode::CommitSelect => {
            return String::from("j/k:nav  Enter:select  Esc:cancel");
        }
        Mode::SearchContent | Mode::SearchFilename | Mode::GlobalSearch => {
            return String::from("Enter:confirm  Esc:cancel");
        }
        Mode::Help => {
            return String::from("q/Esc:close");
        }
        _ => {}
    }
    if let Some(ref msg) = app.message {
        return msg.clone();
    }
    if !app.search_results.is_empty() {
        return format!(
            "Result {}/{}    n:next  N:prev  Esc:clear",
            app.selected_search_result + 1,
            app.search_results.len()
        );
    }
    let (added, removed) = app.total_stats();
    let filter_label = match app.filter {
        Filter::All => "all",
        Filter::Staged => "staged",
        Filter::Unstaged => "unstaged",
    };
    let stats = format!("+{} -{} [{}]  ", added, removed, filter_label);
    match app.focus {
        Focus::Files => {
            format!(
                "{}q:quit  h/l:focus  j/k:nav  ]/[:file  d/u:page  enter:open  space:toggle  s:filter  U:untracked  b:branch  c:commit  |:split  S:global  r:refresh",
                stats
            )
        }
        Focus::Diff => {
            format!(
                "{}q:quit  h:focus  j/k:{}  J/K:line  d/u:page  g/G:top/bot  y:copy  Y:clean  /:search  s:filter  v:scroll  U:untracked  b:branch  c:commit  |:split  S:global  r:refresh",
                stats,
                app.scroll_step.label()
            )
        }
    }
}
