use crate::git::LineType;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollStep {
    Hunk,
    Line,
    Page,
}

impl ScrollStep {
    pub fn next(self) -> Self {
        match self {
            ScrollStep::Hunk => ScrollStep::Line,
            ScrollStep::Line => ScrollStep::Page,
            ScrollStep::Page => ScrollStep::Hunk,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ScrollStep::Hunk => "hunk",
            ScrollStep::Line => "line",
            ScrollStep::Page => "page",
        }
    }
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
