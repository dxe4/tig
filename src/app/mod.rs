use crate::cli::CompareTarget;
use crate::git::{FileChange, LineType, get_changed_files, open_repo};
use crate::search::{GlobalMatch, SearchResult};
use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub mod clipboard;
pub mod compare;
pub mod key_handlers;
pub mod mode;
pub mod navigation;
pub mod search;
pub mod status_text;
pub mod tree;
pub use mode::*;

pub struct App {
    pub files: Vec<FileChange>,
    pub selected_file: usize,
    pub tree_entries: Vec<TreeEntry>,
    pub selected_tree_item: usize,
    pub expanded_dirs: HashSet<String>,
    pub mode: Mode,
    pub focus: Focus,
    pub search_query: String,
    pub last_search_query: String,
    pub search_results: Vec<SearchResult>,
    pub selected_search_result: usize,
    pub global_search_query: String,
    pub global_search_results: Vec<GlobalMatch>,
    pub global_search_selected: usize,
    pub message: Option<String>,
    pub message_time: Option<Instant>,
    pub diff_cursor: usize,
    pub diff_scroll: usize,
    pub diff_visible_height: usize,
    pub tree_visible_height: usize,
    pub filter: Filter,
    pub branches: Vec<String>,
    pub selected_branch: usize,
    pub compare_branch: Option<String>,
    pub compare_range: Option<(String, String)>,
    pub commits: Vec<(String, String)>,
    pub selected_commit: usize,
    pub compare_commit: Option<String>,
    pub side_by_side: bool,
    pub show_untracked: bool,
    pub scroll_step: ScrollStep,
    clipboard: Option<Clipboard>,
    diff_cache: Option<(usize, Rc<Vec<DiffLine>>)>,
}

impl App {
    pub fn new(initial: Option<CompareTarget>) -> Result<Self> {
        open_repo()?;
        let files = get_changed_files(false)?;
        let mut app = Self {
            files,
            selected_file: 0,
            tree_entries: Vec::new(),
            selected_tree_item: 0,
            expanded_dirs: HashSet::new(),
            mode: Mode::Normal,
            focus: Focus::Files,
            search_query: String::new(),
            last_search_query: String::new(),
            search_results: Vec::new(),
            selected_search_result: 0,
            global_search_query: String::new(),
            global_search_results: Vec::new(),
            global_search_selected: 0,
            message: None,
            message_time: None,
            diff_cursor: 0,
            diff_scroll: 0,
            diff_visible_height: 0,
            tree_visible_height: 0,
            filter: Filter::All,
            branches: Vec::new(),
            selected_branch: 0,
            compare_branch: None,
            compare_range: None,
            commits: Vec::new(),
            selected_commit: 0,
            compare_commit: None,
            side_by_side: false,
            show_untracked: false,
            scroll_step: ScrollStep::Hunk,
            clipboard: Clipboard::new().ok(),
            diff_cache: None,
        };
        app.reset_view_to_files();

        if let Some(target) = initial {
            if let Err(e) = app.apply_target(target) {
                app.show_message(format!("Failed to load target: {}", e));
            }
        }

        Ok(app)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(true);
        }
        match self.mode {
            Mode::Normal => key_handlers::handle_normal_key(self, key),
            Mode::SearchContent | Mode::SearchFilename => {
                key_handlers::handle_search_key(self, key)
            }
            Mode::GlobalSearch => key_handlers::handle_global_search_key(self, key),
            Mode::BranchSelect => key_handlers::handle_branch_select_key(self, key),
            Mode::CommitSelect => key_handlers::handle_commit_select_key(self, key),
            Mode::Help => key_handlers::handle_help_key(self, key),
        }
    }

    fn enter_branch_select(&mut self) -> Result<()> {
        compare::enter_branch_select(self)
    }

    fn set_compare_branch(&mut self, branch: &str) -> Result<()> {
        compare::set_compare_branch(self, branch)
    }

    fn clear_compare_branch(&mut self) -> Result<()> {
        compare::clear_compare_branch(self)
    }

    fn enter_commit_select(&mut self) -> Result<()> {
        compare::enter_commit_select(self)
    }

    fn set_compare_commit(&mut self, commit: &str) -> Result<()> {
        compare::set_compare_commit(self, commit)
    }

    fn clear_compare_commit(&mut self) -> Result<()> {
        compare::clear_compare_commit(self)
    }

    fn set_compare_range(&mut self, left: &str, right: &str) -> Result<()> {
        compare::set_compare_range(self, left, right)
    }

    fn clear_compare_range(&mut self) -> Result<()> {
        compare::clear_compare_range(self)
    }

    fn apply_target(&mut self, target: CompareTarget) -> Result<()> {
        match target {
            CompareTarget::Commit(ref commit) => self.set_compare_commit(commit),
            CompareTarget::Range { ref left, ref right } => self.set_compare_range(left, right),
        }
    }

    pub fn diff_lines(&mut self) -> Rc<Vec<DiffLine>> {
        if self.files.is_empty() {
            return Rc::new(Vec::new());
        }
        if let Some((idx, ref lines)) = self.diff_cache
            && idx == self.selected_file
        {
            return Rc::clone(lines);
        }
        let lines = Rc::new(self.compute_diff_lines());
        self.diff_cache = Some((self.selected_file, Rc::clone(&lines)));
        lines
    }

    fn compute_diff_lines(&self) -> Vec<DiffLine> {
        if self.files.is_empty() {
            return Vec::new();
        }
        let file = &self.files[self.selected_file];
        if file.is_binary {
            return vec![DiffLine {
                content: String::from("Binary file - diff not available"),
                line_type: LineType::Context,
                hunk_index: 0,
            }];
        }
        let mut lines = Vec::new();
        for (hunk_idx, hunk) in file.hunks.iter().enumerate() {
            lines.push(DiffLine {
                content: hunk.header.clone(),
                line_type: LineType::Header,
                hunk_index: hunk_idx,
            });
            for line in &hunk.lines {
                lines.push(DiffLine {
                    content: line.content.clone(),
                    line_type: line.line_type,
                    hunk_index: hunk_idx,
                });
            }
        }
        if lines.is_empty() {
            lines.push(DiffLine {
                content: String::from("No diff available"),
                line_type: LineType::Context,
                hunk_index: 0,
            });
        }
        lines
    }

    fn invalidate_diff_cache(&mut self) {
        self.diff_cache = None;
    }

    fn rebuild_tree(&mut self) {
        self.tree_entries = tree::build_tree_entries(&self.files, &self.expanded_dirs, self.filter);
        // Ensure selected_file points to a visible file
        if !self
            .tree_entries
            .iter()
            .any(|e| e.file_index == Some(self.selected_file))
        {
            for (i, entry) in self.tree_entries.iter().enumerate() {
                if entry.file_index.is_some() {
                    self.selected_tree_item = i;
                    self.selected_file = entry.file_index.unwrap();
                    break;
                }
            }
        }
    }

    fn cycle_filter(&mut self) {
        self.search_results.clear();
        self.filter = match self.filter {
            Filter::All => Filter::Staged,
            Filter::Staged => Filter::Unstaged,
            Filter::Unstaged => Filter::All,
        };
        self.rebuild_tree();
        self.selected_tree_item = self
            .selected_tree_item
            .min(self.tree_entries.len().saturating_sub(1));
        self.update_selected_file_from_tree();
        self.diff_cursor = 0;
        self.diff_scroll = 0;
        let label = match self.filter {
            Filter::All => "all",
            Filter::Staged => "staged",
            Filter::Unstaged => "unstaged",
        };
        self.show_message(format!("Filter: {}", label));
    }

    fn next_tree_item(&mut self) {
        navigation::next_tree_item(self);
    }

    fn prev_tree_item(&mut self) {
        navigation::prev_tree_item(self);
    }

    fn update_selected_file_from_tree(&mut self) {
        navigation::update_selected_file_from_tree(self);
    }

    fn next_file(&mut self) {
        navigation::next_file(self);
    }

    fn prev_file(&mut self) {
        navigation::prev_file(self);
    }

    fn toggle_dir(&mut self) {
        navigation::toggle_dir(self);
    }

    fn collapse_dir_or_parent(&mut self) {
        navigation::collapse_dir_or_parent(self);
    }

    fn move_diff_cursor(&mut self, delta: i32) {
        navigation::move_diff_cursor(self, delta);
    }

    fn page_diff_down(&mut self) {
        navigation::page_diff_down(self);
    }

    fn page_diff_up(&mut self) {
        navigation::page_diff_up(self);
    }

    fn half_page_diff_down(&mut self) {
        navigation::half_page_diff_down(self);
    }

    fn half_page_diff_up(&mut self) {
        navigation::half_page_diff_up(self);
    }

    fn page_tree_down(&mut self) {
        navigation::page_tree_down(self);
    }

    fn page_tree_up(&mut self) {
        navigation::page_tree_up(self);
    }

    fn half_page_tree_down(&mut self) {
        navigation::half_page_tree_down(self);
    }

    fn half_page_tree_up(&mut self) {
        navigation::half_page_tree_up(self);
    }

    fn scroll_diff_down(&mut self) {
        navigation::scroll_diff_down(self);
    }

    fn scroll_diff_up(&mut self) {
        navigation::scroll_diff_up(self);
    }

    fn adjust_scroll(&mut self) {
        navigation::adjust_scroll(self);
    }

    fn copy_hunk(&mut self) -> Result<()> {
        clipboard::copy_hunk(self)
    }

    fn copy_hunk_clean(&mut self) -> Result<()> {
        clipboard::copy_hunk_clean(self)
    }

    fn execute_search(&mut self) -> Result<()> {
        search::execute_search(self)
    }

    fn incremental_search(&mut self) -> Result<()> {
        search::incremental_search(self)
    }

    fn perform_search(&mut self, show_no_results_msg: bool) -> Result<()> {
        search::perform_search(self, show_no_results_msg)
    }

    fn next_search_result(&mut self) {
        search::next_search_result(self);
    }

    fn prev_search_result(&mut self) {
        search::prev_search_result(self);
    }

    fn incremental_global_search(&mut self) {
        search::incremental_global_search(self);
    }

    fn next_global_result(&mut self) {
        search::next_global_result(self);
    }

    fn prev_global_result(&mut self) {
        search::prev_global_result(self);
    }

    fn jump_to_global_result(&mut self) {
        search::jump_to_global_result(self);
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Files => Focus::Diff,
            Focus::Diff => Focus::Files,
        };
    }

    pub fn show_message(&mut self, msg: String) {
        self.message = Some(msg);
        self.message_time = Some(Instant::now());
    }

    pub fn check_message_timeout(&mut self) {
        if let Some(time) = self.message_time
            && time.elapsed() > Duration::from_secs(3)
        {
            self.message = None;
            self.message_time = None;
        }
    }

    fn total_stats(&self) -> (usize, usize) {
        let mut added = 0;
        let mut removed = 0;
        for file in &self.files {
            let visible = match self.filter {
                Filter::All => true,
                Filter::Staged => file.staged_status.is_some(),
                Filter::Unstaged => file.unstaged_status.is_some(),
            };
            if !visible {
                continue;
            }
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    match line.line_type {
                        LineType::Added => added += 1,
                        LineType::Removed => removed += 1,
                        _ => {}
                    }
                }
            }
        }
        (added, removed)
    }

    fn reload_files(&mut self) -> Result<()> {
        self.search_results.clear();
        let old_path = self.files.get(self.selected_file).map(|f| f.path.clone());
        self.files = if let Some((ref left, ref right)) = self.compare_range {
            crate::git::get_range_files(left, right)?
        } else if let Some(ref commit) = self.compare_commit {
            crate::git::get_commit_files(commit)?
        } else if let Some(ref branch) = self.compare_branch {
            crate::git::get_branch_files(branch)?
        } else {
            get_changed_files(self.show_untracked)?
        };
        self.invalidate_diff_cache();
        self.rebuild_tree();
        if let Some(ref path) = old_path {
            if let Some(idx) = self.files.iter().position(|f| f.path == *path) {
                self.selected_file = idx;
                for (i, entry) in self.tree_entries.iter().enumerate() {
                    if entry.file_index == Some(idx) {
                        self.selected_tree_item = i;
                        break;
                    }
                }
            } else if !self.tree_entries.is_empty() {
                for (i, entry) in self.tree_entries.iter().enumerate() {
                    if entry.file_index.is_some() {
                        self.selected_tree_item = i;
                        self.selected_file = entry.file_index.unwrap();
                        break;
                    }
                }
            }
        }
        self.diff_cursor = 0;
        self.diff_scroll = 0;
        self.show_message("Files reloaded".to_string());
        Ok(())
    }

    fn reset_view_to_files(&mut self) {
        self.search_results.clear();
        self.filter = Filter::All;
        self.rebuild_tree();
        let all_dirs: Vec<String> = self
            .tree_entries
            .iter()
            .filter(|e| e.is_dir)
            .map(|e| e.full_path.clone())
            .collect();
        self.expanded_dirs.extend(all_dirs);
        self.rebuild_tree();
        self.selected_file = 0;
        self.selected_tree_item = 0;
        for (i, entry) in self.tree_entries.iter().enumerate() {
            if entry.file_index.is_some() {
                self.selected_tree_item = i;
                self.selected_file = entry.file_index.unwrap();
                break;
            }
        }
        self.diff_cursor = 0;
        self.diff_scroll = 0;
    }

    pub fn search_filter_text(&self) -> Option<String> {
        if self.mode == Mode::SearchContent {
            let (filter, _) = search::parse_search_query(&self.search_query);
            filter.map(|s| s.to_string())
        } else {
            None
        }
    }

    pub fn status_text(&self) -> String {
        status_text::status_text(self)
    }
}

#[cfg(test)]
impl App {
    pub fn test_new(files: Vec<FileChange>) -> Self {
        let mut app = Self {
            files,
            selected_file: 0,
            tree_entries: Vec::new(),
            selected_tree_item: 0,
            expanded_dirs: std::collections::HashSet::new(),
            mode: Mode::Normal,
            focus: Focus::Files,
            search_query: String::new(),
            last_search_query: String::new(),
            search_results: Vec::new(),
            selected_search_result: 0,
            global_search_query: String::new(),
            global_search_results: Vec::new(),
            global_search_selected: 0,
            message: None,
            message_time: None,
            diff_cursor: 0,
            diff_scroll: 0,
            diff_visible_height: 0,
            tree_visible_height: 0,
            filter: Filter::All,
            branches: Vec::new(),
            selected_branch: 0,
            compare_branch: None,
            compare_range: None,
            commits: Vec::new(),
            selected_commit: 0,
            compare_commit: None,
            side_by_side: false,
            show_untracked: false,
            scroll_step: ScrollStep::Hunk,
            clipboard: None,
            diff_cache: None,
        };
        app.reset_view_to_files();
        app
    }
}


#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn app_new_with_commit_target() {
        let app = App::new(Some(CompareTarget::Commit("HEAD".to_string()))).unwrap();
        assert!(app.compare_commit.is_some());
        assert!(app.compare_branch.is_none());
        assert!(app.compare_range.is_none());
        assert!(!app.files.is_empty());
    }

    #[test]
    fn app_new_with_range_target() {
        let app = App::new(Some(CompareTarget::Range {
            left: "main".to_string(),
            right: "test-diff-branch".to_string(),
        }))
        .unwrap();
        assert!(app.compare_range.is_some());
        assert!(app.compare_branch.is_none());
        assert!(app.compare_commit.is_none());
    }

    #[test]
    fn app_new_with_invalid_target_shows_message() {
        let app = App::new(Some(CompareTarget::Commit("does-not-exist".to_string()))).unwrap();
        assert!(app.compare_commit.is_none());
        assert!(app.compare_branch.is_none());
        assert!(app.compare_range.is_none());
        assert!(app.message.is_some());
    }
}
