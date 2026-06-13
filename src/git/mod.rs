use anyhow::Result;

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub status: char,
    pub staged_status: Option<char>,
    pub unstaged_status: Option<char>,
    pub hunks: Vec<Hunk>,
    pub is_binary: bool,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<HunkLine>,
}

#[derive(Debug, Clone)]
pub struct HunkLine {
    pub content: String,
    pub line_type: LineType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineType {
    Added,
    Removed,
    Context,
    Header,
    NoNewline,
}

pub(crate) fn check_git_output(output: &std::process::Output) -> Result<()> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git command failed: {}", stderr.trim());
    }
    Ok(())
}

pub mod parse;
pub mod status;

pub use parse::parse_diff;
pub use status::*;
