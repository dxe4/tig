use crate::git::{FileChange, LineType, get_changed_files, open_repo};
use crate::search::{
    GlobalMatch, SearchResult, global_search, search_content_filtered, search_filename,
};
use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::{BTreeMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    SearchContent,
    SearchFilename,
    GlobalSearch,
    BranchSelect,
    CommitSelect,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Files,
    Diff,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Filter {
    All,
    Staged,
    Unstaged,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub content: String,
    pub line_type: LineType,
    pub hunk_index: usize,
}

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub name: String,
    pub full_path: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
    pub file_index: Option<usize>,
    pub status: Option<char>,
    pub added: usize,
    pub removed: usize,
}

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
    pub commits: Vec<(String, String)>,
    pub selected_commit: usize,
    pub compare_commit: Option<String>,
    pub side_by_side: bool,
    clipboard: Option<Clipboard>,
}

impl App {
    pub fn new() -> Result<Self> {
        open_repo()?;
        let files = get_changed_files()?;
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
            commits: Vec::new(),
            selected_commit: 0,
            compare_commit: None,
            side_by_side: false,
            clipboard: Clipboard::new().ok(),
        };
        app.reset_view_to_files();
        Ok(app)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(true);
        }
        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::SearchContent | Mode::SearchFilename => self.handle_search_key(key),
            Mode::GlobalSearch => self.handle_global_search_key(key),
            Mode::BranchSelect => self.handle_branch_select_key(key),
            Mode::CommitSelect => self.handle_commit_select_key(key),
            Mode::Help => self.handle_help_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<bool> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('d') => {
                    if self.focus == Focus::Files {
                        self.page_tree_down();
                    } else {
                        self.page_diff_down();
                    }
                    self.adjust_scroll();
                    return Ok(false);
                }
                KeyCode::Char('u') => {
                    if self.focus == Focus::Files {
                        self.page_tree_up();
                    } else {
                        self.page_diff_up();
                    }
                    self.adjust_scroll();
                    return Ok(false);
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('j') | KeyCode::Down => {
                if self.focus == Focus::Files {
                    self.next_tree_item();
                } else if !self.next_hunk() {
                    self.next_file();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.focus == Focus::Files {
                    self.prev_tree_item();
                } else if !self.prev_hunk() {
                    self.prev_file_last_hunk();
                }
            }
            KeyCode::Char('J') => {
                if self.focus == Focus::Diff {
                    self.move_diff_cursor(1);
                }
            }
            KeyCode::Char('K') => {
                if self.focus == Focus::Diff {
                    self.move_diff_cursor(-1);
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.focus == Focus::Diff {
                    self.focus = Focus::Files;
                } else {
                    self.collapse_dir_or_parent();
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.focus == Focus::Files
                    && let Some(entry) = self.tree_entries.get(self.selected_tree_item)
                {
                    if entry.is_dir {
                        self.toggle_dir();
                    } else {
                        self.focus = Focus::Diff;
                    }
                }
            }
            KeyCode::Char(']') => self.next_file(),
            KeyCode::Char('[') => self.prev_file(),
            KeyCode::Char('d') => {
                if self.focus == Focus::Files {
                    self.page_tree_down();
                } else {
                    self.page_diff_down();
                }
            }
            KeyCode::Char('u') => {
                if self.focus == Focus::Files {
                    self.page_tree_up();
                } else {
                    self.page_diff_up();
                }
            }
            KeyCode::Char('g') => {
                if self.focus == Focus::Files {
                    self.selected_tree_item = 0;
                    self.update_selected_file_from_tree();
                } else {
                    self.diff_cursor = 0;
                }
            }
            KeyCode::Char('G') => {
                if self.focus == Focus::Files {
                    self.selected_tree_item = self.tree_entries.len().saturating_sub(1);
                    self.update_selected_file_from_tree();
                } else {
                    let max = self.diff_lines().len().saturating_sub(1);
                    self.diff_cursor = max;
                }
            }
            KeyCode::Char('y') => self.copy_hunk()?,
            KeyCode::Char('Y') => self.copy_hunk_clean()?,
            KeyCode::Char(' ') => {
                if self.focus == Focus::Files {
                    self.toggle_dir();
                }
            }
            KeyCode::Enter => {
                if self.focus == Focus::Files
                    && let Some(entry) = self.tree_entries.get(self.selected_tree_item)
                {
                    if entry.is_dir {
                        self.toggle_dir();
                    } else {
                        self.focus = Focus::Diff;
                    }
                }
            }
            KeyCode::Char('/') => {
                self.mode = Mode::SearchContent;
                self.search_query.clear();
                self.search_results.clear();
            }
            KeyCode::Char('f') => {
                self.mode = Mode::SearchFilename;
                self.search_query.clear();
                self.search_results.clear();
            }
            KeyCode::Char('S') => {
                self.mode = Mode::GlobalSearch;
                self.global_search_query.clear();
                self.global_search_results.clear();
            }
            KeyCode::Char('r') => {
                self.reload_files()?;
            }
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
            }
            KeyCode::Char('s') => {
                self.cycle_filter();
            }
            KeyCode::Char('b') => {
                if self.compare_branch.is_some() {
                    self.clear_compare_branch();
                } else {
                    self.enter_branch_select()?;
                }
            }
            KeyCode::Char('c') => {
                if self.compare_commit.is_some() {
                    self.clear_compare_commit();
                } else {
                    self.enter_commit_select()?;
                }
            }
            KeyCode::Char('|') => {
                self.side_by_side = !self.side_by_side;
                let label = if self.side_by_side {
                    "side-by-side"
                } else {
                    "unified"
                };
                self.show_message(format!("Diff view: {}", label));
            }
            KeyCode::Char('n') => self.next_search_result(),
            KeyCode::Char('N') => self.prev_search_result(),
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Esc => {
                self.search_results.clear();
            }
            _ => {}
        }
        self.adjust_scroll();
        Ok(false)
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.search_results.clear();
            }
            KeyCode::Enter => {
                self.execute_search()?;
                self.mode = Mode::Normal;
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.incremental_search()?;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                if self.search_query.is_empty() {
                    self.search_results.clear();
                } else {
                    self.incremental_search()?;
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_global_search_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.global_search_results.clear();
            }
            KeyCode::Enter => {
                self.jump_to_global_result();
                self.mode = Mode::Normal;
                self.global_search_results.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => self.next_global_result(),
            KeyCode::Char('k') | KeyCode::Up => self.prev_global_result(),
            KeyCode::Char(c) => {
                self.global_search_query.push(c);
                self.incremental_global_search();
            }
            KeyCode::Backspace => {
                self.global_search_query.pop();
                if self.global_search_query.is_empty() {
                    self.global_search_results.clear();
                } else {
                    self.incremental_global_search();
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_branch_select_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                if let Some(branch) = self.branches.get(self.selected_branch) {
                    let branch = branch.clone();
                    self.set_compare_branch(&branch)?;
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.branches.is_empty() {
                    self.selected_branch = (self.selected_branch + 1) % self.branches.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.branches.is_empty() {
                    if self.selected_branch == 0 {
                        self.selected_branch = self.branches.len() - 1;
                    } else {
                        self.selected_branch -= 1;
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn enter_branch_select(&mut self) -> Result<()> {
        self.branches = crate::git::get_branches()?;
        self.selected_branch = 0;
        if self.branches.is_empty() {
            self.show_message("No branches found".to_string());
        } else {
            self.mode = Mode::BranchSelect;
        }
        Ok(())
    }

    fn set_compare_branch(&mut self, branch: &str) -> Result<()> {
        self.compare_commit = None;
        self.files = crate::git::get_branch_files(branch)?;
        self.compare_branch = Some(branch.to_string());
        self.reset_view_to_files();
        self.show_message(format!("Comparing against: {}", branch));
        Ok(())
    }

    fn clear_compare_branch(&mut self) {
        self.compare_branch = None;
        if let Ok(files) = get_changed_files() {
            self.files = files;
        }
        self.reset_view_to_files();
        self.show_message("Cleared branch comparison".to_string());
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_commit_select_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                if let Some((hash, _)) = self.commits.get(self.selected_commit) {
                    let hash = hash.clone();
                    self.set_compare_commit(&hash)?;
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.commits.is_empty() {
                    self.selected_commit = (self.selected_commit + 1) % self.commits.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.commits.is_empty() {
                    if self.selected_commit == 0 {
                        self.selected_commit = self.commits.len() - 1;
                    } else {
                        self.selected_commit -= 1;
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn enter_commit_select(&mut self) -> Result<()> {
        self.commits = crate::git::get_commits(50)?;
        self.selected_commit = 0;
        if self.commits.is_empty() {
            self.show_message("No commits found".to_string());
        } else {
            self.mode = Mode::CommitSelect;
        }
        Ok(())
    }

    fn set_compare_commit(&mut self, commit: &str) -> Result<()> {
        self.compare_branch = None;
        self.files = crate::git::get_commit_files(commit)?;
        self.compare_commit = Some(commit.to_string());
        self.reset_view_to_files();
        self.show_message(format!("Showing commit: {}", commit));
        Ok(())
    }

    fn clear_compare_commit(&mut self) {
        self.compare_commit = None;
        if let Ok(files) = get_changed_files() {
            self.files = files;
        }
        self.reset_view_to_files();
        self.show_message("Cleared commit view".to_string());
    }

    pub fn diff_lines(&self) -> Vec<DiffLine> {
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

    fn rebuild_tree(&mut self) {
        self.tree_entries = build_tree_entries(&self.files, &self.expanded_dirs, self.filter);
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
        if self.tree_entries.is_empty() {
            return;
        }
        self.selected_tree_item = (self.selected_tree_item + 1).min(self.tree_entries.len() - 1);
        self.update_selected_file_from_tree();
        self.diff_cursor = 0;
        self.diff_scroll = 0;
    }

    fn prev_tree_item(&mut self) {
        self.selected_tree_item = self.selected_tree_item.saturating_sub(1);
        self.update_selected_file_from_tree();
        self.diff_cursor = 0;
        self.diff_scroll = 0;
    }

    fn update_selected_file_from_tree(&mut self) {
        if let Some(entry) = self.tree_entries.get(self.selected_tree_item)
            && let Some(idx) = entry.file_index
        {
            self.selected_file = idx;
        }
    }

    fn next_file(&mut self) {
        if self.files.is_empty() || self.tree_entries.is_empty() {
            return;
        }
        loop {
            if self.selected_tree_item >= self.tree_entries.len() - 1 {
                break;
            }
            self.selected_tree_item += 1;
            if self.tree_entries[self.selected_tree_item]
                .file_index
                .is_some()
            {
                break;
            }
        }
        self.update_selected_file_from_tree();
        self.diff_cursor = 0;
        self.diff_scroll = 0;
    }

    fn prev_file(&mut self) {
        if self.files.is_empty() || self.tree_entries.is_empty() {
            return;
        }
        loop {
            if self.selected_tree_item == 0 {
                break;
            }
            self.selected_tree_item -= 1;
            if self.tree_entries[self.selected_tree_item]
                .file_index
                .is_some()
            {
                break;
            }
        }
        self.update_selected_file_from_tree();
        self.diff_cursor = 0;
        self.diff_scroll = 0;
    }

    fn toggle_dir(&mut self) {
        if let Some(entry) = self.tree_entries.get(self.selected_tree_item)
            && entry.is_dir
        {
            let path = entry.full_path.clone();
            if self.expanded_dirs.contains(&path) {
                self.expanded_dirs.remove(&path);
            } else {
                self.expanded_dirs.insert(path);
            }
            self.rebuild_tree();
            self.selected_tree_item = self
                .selected_tree_item
                .min(self.tree_entries.len().saturating_sub(1));
        }
    }

    fn collapse_dir_or_parent(&mut self) {
        let Some(entry) = self.tree_entries.get(self.selected_tree_item) else {
            return;
        };
        if entry.is_dir && self.expanded_dirs.contains(&entry.full_path) {
            self.expanded_dirs.remove(&entry.full_path);
            self.rebuild_tree();
            return;
        }
        let target_depth = entry.depth.saturating_sub(1);
        for i in (0..self.selected_tree_item).rev() {
            if self.tree_entries[i].depth == target_depth && self.tree_entries[i].is_dir {
                self.expanded_dirs.remove(&self.tree_entries[i].full_path);
                self.selected_tree_item = i;
                self.rebuild_tree();
                return;
            }
        }
    }

    fn move_diff_cursor(&mut self, delta: i32) {
        let max = self.diff_lines().len().saturating_sub(1);
        if delta > 0 {
            self.diff_cursor = (self.diff_cursor + delta as usize).min(max);
        } else {
            self.diff_cursor = self.diff_cursor.saturating_sub((-delta) as usize);
        }
    }

    fn page_diff_down(&mut self) {
        let delta = self.diff_visible_height.saturating_sub(2).max(1);
        self.move_diff_cursor(delta as i32);
    }

    fn page_diff_up(&mut self) {
        let delta = self.diff_visible_height.saturating_sub(2).max(1);
        self.move_diff_cursor(-(delta as i32));
    }

    fn page_tree_down(&mut self) {
        let delta = self.tree_visible_height.saturating_sub(2).max(1);
        self.selected_tree_item =
            (self.selected_tree_item + delta).min(self.tree_entries.len().saturating_sub(1));
        self.update_selected_file_from_tree();
        self.diff_cursor = 0;
        self.diff_scroll = 0;
    }

    fn page_tree_up(&mut self) {
        let delta = self.tree_visible_height.saturating_sub(2).max(1);
        self.selected_tree_item = self.selected_tree_item.saturating_sub(delta);
        self.update_selected_file_from_tree();
        self.diff_cursor = 0;
        self.diff_scroll = 0;
    }

    fn next_hunk(&mut self) -> bool {
        let lines = self.diff_lines();
        if lines.is_empty() {
            return false;
        }
        let cursor = self.diff_cursor;
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
                self.diff_cursor = i;
                return true;
            }
        }
        self.diff_cursor = header_idx;
        true
    }

    fn prev_hunk(&mut self) -> bool {
        let lines = self.diff_lines();
        if lines.is_empty() {
            return false;
        }
        let cursor = self.diff_cursor;
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
                self.diff_cursor = i;
                return true;
            }
        }
        self.diff_cursor = header_idx;
        true
    }

    fn prev_file_last_hunk(&mut self) {
        self.prev_file();
        let lines = self.diff_lines();
        if lines.is_empty() {
            return;
        }
        let last_hunk = lines.last().map(|l| l.hunk_index).unwrap_or(0);
        for (i, line) in lines.iter().enumerate().rev() {
            if line.hunk_index == last_hunk && line.line_type != LineType::Header {
                self.diff_cursor = i;
                return;
            }
        }
        // Fallback: land on the header of the last hunk
        for (i, line) in lines.iter().enumerate().rev() {
            if line.hunk_index == last_hunk && line.line_type == LineType::Header {
                self.diff_cursor = i;
                return;
            }
        }
    }

    fn adjust_scroll(&mut self) {
        if self.diff_visible_height == 0 {
            return;
        }
        let half = self.diff_visible_height / 2;
        let target = self.diff_cursor.saturating_sub(half);
        let max_scroll = self
            .diff_lines()
            .len()
            .saturating_sub(self.diff_visible_height);
        self.diff_scroll = target.min(max_scroll);
    }

    fn copy_hunk(&mut self) -> Result<()> {
        if self.files.is_empty() {
            return Ok(());
        }
        let lines = self.diff_lines();
        if lines.is_empty() {
            return Ok(());
        }
        let cursor = self.diff_cursor.min(lines.len() - 1);
        let hunk_idx = lines[cursor].hunk_index;

        let file = &self.files[self.selected_file];
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
        if let Some(ref mut cb) = self.clipboard {
            cb.set_text(text)?;
            self.show_message(String::from("Hunk copied to clipboard"));
        } else {
            self.show_message(String::from("Clipboard not available"));
        }
        Ok(())
    }

    fn copy_hunk_clean(&mut self) -> Result<()> {
        if self.files.is_empty() {
            return Ok(());
        }
        let lines = self.diff_lines();
        if lines.is_empty() {
            return Ok(());
        }
        let cursor = self.diff_cursor.min(lines.len() - 1);
        let hunk_idx = lines[cursor].hunk_index;

        let file = &self.files[self.selected_file];
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
        if let Some(ref mut cb) = self.clipboard {
            cb.set_text(text)?;
            self.show_message(String::from("Code copied to clipboard"));
        } else {
            self.show_message(String::from("Clipboard not available"));
        }
        Ok(())
    }

    fn execute_search(&mut self) -> Result<()> {
        self.perform_search(true)
    }

    fn incremental_search(&mut self) -> Result<()> {
        self.perform_search(false)
    }

    fn perform_search(&mut self, show_no_results_msg: bool) -> Result<()> {
        if self.search_query.is_empty() {
            self.search_results.clear();
            return Ok(());
        }
        self.search_results = match self.mode {
            Mode::SearchContent => {
                let (filter, content) = parse_search_query(&self.search_query);
                if content.is_empty() {
                    self.last_search_query.clear();
                    Vec::new()
                } else {
                    self.last_search_query = content.to_string();
                    search_content_filtered(&self.files, content, filter)
                }
            }
            Mode::SearchFilename => {
                self.last_search_query = self.search_query.clone();
                search_filename(&self.files, &self.search_query)
            }
            _ => Vec::new(),
        };
        self.selected_search_result = 0;
        if !self.search_results.is_empty() {
            self.jump_to_search_result(false);
        } else if show_no_results_msg {
            self.show_message(String::from("No results found"));
        }
        Ok(())
    }

    fn next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.selected_search_result = (self.selected_search_result + 1) % self.search_results.len();
        self.jump_to_search_result(true);
    }

    fn prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        if self.selected_search_result == 0 {
            self.selected_search_result = self.search_results.len() - 1;
        } else {
            self.selected_search_result -= 1;
        }
        self.jump_to_search_result(true);
    }

    fn jump_to_search_result(&mut self, change_focus: bool) {
        if let Some(result) = self.search_results.get(self.selected_search_result)
            && result.file_index < self.files.len()
        {
            self.selected_file = result.file_index;
            for (i, entry) in self.tree_entries.iter().enumerate() {
                if entry.file_index == Some(result.file_index) {
                    self.selected_tree_item = i;
                    break;
                }
            }
            if let Some(line_idx) = result.line_number {
                self.diff_cursor = line_idx;
            } else {
                self.diff_cursor = 0;
            }
            self.adjust_scroll();
            if change_focus {
                self.focus = Focus::Diff;
            }
        }
    }

    fn incremental_global_search(&mut self) {
        if self.global_search_query.is_empty() {
            self.global_search_results.clear();
            return;
        }
        match global_search(&self.global_search_query) {
            Ok(results) => {
                self.global_search_results = results;
                self.global_search_selected = 0;
            }
            Err(e) => {
                self.show_message(format!("Regex error: {}", e));
            }
        }
    }

    fn next_global_result(&mut self) {
        if self.global_search_results.is_empty() {
            return;
        }
        self.global_search_selected =
            (self.global_search_selected + 1) % self.global_search_results.len();
    }

    fn prev_global_result(&mut self) {
        if self.global_search_results.is_empty() {
            return;
        }
        if self.global_search_selected == 0 {
            self.global_search_selected = self.global_search_results.len() - 1;
        } else {
            self.global_search_selected -= 1;
        }
    }

    fn jump_to_global_result(&mut self) {
        let (path, _line_number) = {
            let Some(result) = self.global_search_results.get(self.global_search_selected) else {
                return;
            };
            (result.file_path.clone(), result.line_number)
        };

        let file_idx = match self.files.iter().position(|f| f.path == path) {
            Some(idx) => idx,
            None => {
                let Ok((hunks, is_binary)) = crate::git::get_diff(&path) else {
                    self.show_message(format!("Cannot read {}", path));
                    return;
                };
                let status = if std::fs::metadata(&path).is_ok() {
                    ' '
                } else {
                    'D'
                };
                self.files.push(crate::git::FileChange {
                    path: path.clone(),
                    status,
                    staged_status: None,
                    unstaged_status: None,
                    hunks,
                    is_binary,
                });
                self.files.sort_by(|a, b| a.path.cmp(&b.path));
                self.rebuild_tree();
                self.files.iter().position(|f| f.path == path).unwrap_or(0)
            }
        };

        self.selected_file = file_idx;
        for (i, entry) in self.tree_entries.iter().enumerate() {
            if entry.file_index == Some(file_idx) {
                self.selected_tree_item = i;
                break;
            }
        }

        self.diff_cursor = 0;
        self.diff_scroll = 0;
        self.focus = Focus::Diff;
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
        self.files = if let Some(ref branch) = self.compare_branch {
            crate::git::get_branch_files(branch)?
        } else if let Some(ref commit) = self.compare_commit {
            crate::git::get_commit_files(commit)?
        } else {
            get_changed_files()?
        };
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
            let (filter, _) = parse_search_query(&self.search_query);
            filter.map(|s| s.to_string())
        } else {
            None
        }
    }

    pub fn status_text(&self) -> String {
        match self.mode {
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
        if let Some(ref msg) = self.message {
            return msg.clone();
        }
        if !self.search_results.is_empty() {
            return format!(
                "Result {}/{}    n:next  N:prev  Esc:clear",
                self.selected_search_result + 1,
                self.search_results.len()
            );
        }
        let (added, removed) = self.total_stats();
        let filter_label = match self.filter {
            Filter::All => "all",
            Filter::Staged => "staged",
            Filter::Unstaged => "unstaged",
        };
        let stats = format!("+{} -{} [{}]  ", added, removed, filter_label);
        match self.focus {
            Focus::Files => {
                format!(
                    "{}q:quit  h/l:focus  j/k:nav  ]/[:file  d/u:page  enter:open  space:toggle  s:filter  b:branch  c:commit  |:split  S:global  r:refresh",
                    stats
                )
            }
            Focus::Diff => {
                format!(
                    "{}q:quit  h:focus  j/k:hunk  J/K:line  d/u:page  g/G:top/bot  y:copy  Y:clean  /:search  s:filter  b:branch  c:commit  |:split  S:global  r:refresh",
                    stats
                )
            }
        }
    }
}

fn parse_search_query(query: &str) -> (Option<&str>, &str) {
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

fn build_tree_entries(
    files: &[FileChange],
    expanded_dirs: &HashSet<String>,
    filter: Filter,
) -> Vec<TreeEntry> {
    #[derive(Default)]
    struct Node {
        name: String,
        children: BTreeMap<String, Node>,
        file_index: Option<usize>,
        status: Option<char>,
        added: usize,
        removed: usize,
    }

    let mut root = Node::default();
    for (i, file) in files.iter().enumerate() {
        let visible = match filter {
            Filter::All => true,
            Filter::Staged => file.staged_status.is_some(),
            Filter::Unstaged => file.unstaged_status.is_some(),
        };
        if !visible {
            continue;
        }
        let parts: Vec<&str> = file.path.split('/').collect();
        let mut current = &mut root;
        for (j, part) in parts.iter().enumerate() {
            if j == parts.len() - 1 {
                let node = current.children.entry(part.to_string()).or_default();
                node.name = part.to_string();
                node.file_index = Some(i);
                node.status = Some(file.status);
            } else {
                current = current.children.entry(part.to_string()).or_default();
                current.name = part.to_string();
            }
        }
    }

    fn compute_stats(node: &mut Node, files: &[FileChange]) {
        if let Some(idx) = node.file_index {
            let file = &files[idx];
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    match line.line_type {
                        LineType::Added => node.added += 1,
                        LineType::Removed => node.removed += 1,
                        _ => {}
                    }
                }
            }
        }
        for child in node.children.values_mut() {
            compute_stats(child, files);
            node.added += child.added;
            node.removed += child.removed;
        }
    }

    for child in root.children.values_mut() {
        compute_stats(child, files);
    }

    fn walk(
        node: &Node,
        path_prefix: &str,
        depth: usize,
        expanded: &HashSet<String>,
        out: &mut Vec<TreeEntry>,
    ) {
        let full_path = if path_prefix.is_empty() {
            node.name.clone()
        } else {
            format!("{}/{}", path_prefix, node.name)
        };

        let is_dir = !node.children.is_empty() && node.file_index.is_none();
        let has_file = node.file_index.is_some();

        if !node.name.is_empty() {
            if has_file {
                out.push(TreeEntry {
                    name: node.name.clone(),
                    full_path: full_path.clone(),
                    depth,
                    is_dir: false,
                    expanded: false,
                    file_index: node.file_index,
                    status: node.status,
                    added: node.added,
                    removed: node.removed,
                });
            }
            if is_dir {
                out.push(TreeEntry {
                    name: node.name.clone(),
                    full_path: full_path.clone(),
                    depth,
                    is_dir: true,
                    expanded: expanded.contains(&full_path),
                    file_index: None,
                    status: None,
                    added: node.added,
                    removed: node.removed,
                });
            }
        }

        if (is_dir || has_file) && expanded.contains(&full_path) {
            for child in node.children.values() {
                walk(child, &full_path, depth + 1, expanded, out);
            }
        }
    }

    let mut entries = Vec::new();
    for child in root.children.values() {
        walk(child, "", 0, expanded_dirs, &mut entries);
    }
    entries
}
