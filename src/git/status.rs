use crate::git::{FileChange, Hunk, HunkLine, LineType, check_git_output};
use anyhow::Result;
use std::process::Command;

pub fn open_repo() -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()?;
    check_git_output(&output)?;
    Ok(())
}

pub fn get_changed_files(include_untracked: bool) -> Result<Vec<FileChange>> {
    let mut args = vec!["status", "--porcelain"];
    if include_untracked {
        args.push("-uall");
    }
    let output = Command::new("git").args(&args).output()?;
    check_git_output(&output)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut files = Vec::new();
    for line in stdout.lines() {
        if line.len() < 3 {
            continue;
        }
        let staged = line.chars().next().unwrap_or(' ');
        let unstaged = line.chars().nth(1).unwrap_or(' ');
        let path_part = &line[3..];

        let status = if staged != ' ' && staged != '?' {
            staged
        } else if unstaged != ' ' {
            unstaged
        } else {
            continue;
        };

        let path = if status == 'R' || status == 'C' {
            path_part
                .split(" -> ")
                .nth(1)
                .unwrap_or(path_part)
                .to_string()
        } else {
            path_part.to_string()
        };

        let staged_status = if staged != ' ' && staged != '?' {
            Some(staged)
        } else {
            None
        };
        let unstaged_status = if unstaged != ' ' {
            Some(unstaged)
        } else {
            None
        };

        let (hunks, is_binary) = get_diff(&path)?;
        files.push(FileChange {
            path,
            status,
            staged_status,
            unstaged_status,
            hunks,
            is_binary,
        });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

pub fn get_diff(path: &str) -> Result<(Vec<Hunk>, bool)> {
    get_diff_against_branch(path, None)
}

pub fn get_diff_against_branch(path: &str, branch: Option<&str>) -> Result<(Vec<Hunk>, bool)> {
    let output = if let Some(b) = branch {
        Command::new("git")
            .args(["diff", "--no-ext-diff", "-U3", "HEAD", b, "--", path])
            .output()?
    } else {
        Command::new("git")
            .args(["diff", "--no-ext-diff", "-U3", "--", path])
            .output()?
    };
    check_git_output(&output)?;

    let diff_text = String::from_utf8_lossy(&output.stdout);

    if diff_text.contains("Binary files") {
        return Ok((Vec::new(), true));
    }

    if diff_text.trim().is_empty() {
        if std::fs::metadata(path).is_ok() {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            let lines: Vec<HunkLine> = content
                .lines()
                .map(|l| HunkLine {
                    content: format!("+{}", l),
                    line_type: LineType::Added,
                })
                .collect();
            if lines.is_empty() {
                return Ok((Vec::new(), false));
            }
            let hunk = Hunk {
                header: format!("@@ -0,0 +1,{} @@", lines.len()),
                lines,
            };
            return Ok((vec![hunk], false));
        }
        return Ok((Vec::new(), false));
    }

    let hunks = crate::git::parse_diff(&diff_text)?;
    Ok((hunks, false))
}

pub fn get_branches() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .output()?;
    check_git_output(&output)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut branches: Vec<String> = stdout
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if let Ok(current) = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
    {
        let cur = String::from_utf8_lossy(&current.stdout).trim().to_string();
        if let Some(pos) = branches.iter().position(|b| b == &cur) {
            let item = branches.remove(pos);
            branches.insert(0, item);
        }
    }
    Ok(branches)
}

pub fn get_commits(limit: usize) -> Result<Vec<(String, String)>> {
    let output = Command::new("git")
        .args(["log", "--oneline", "-n", &limit.to_string()])
        .output()?;
    check_git_output(&output)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    for line in stdout.lines() {
        if let Some(pos) = line.find(' ') {
            let hash = line[..pos].to_string();
            let message = line[pos + 1..].to_string();
            commits.push((hash, message));
        }
    }
    Ok(commits)
}

pub fn get_commit_diff(path: &str, commit: &str) -> Result<(Vec<Hunk>, bool)> {
    let output = Command::new("git")
        .args([
            "show",
            "--no-ext-diff",
            "--format=",
            "-U3",
            commit,
            "--",
            path,
        ])
        .output()?;
    check_git_output(&output)?;

    let diff_text = String::from_utf8_lossy(&output.stdout);

    if diff_text.contains("Binary files") {
        return Ok((Vec::new(), true));
    }

    if diff_text.trim().is_empty() {
        return Ok((Vec::new(), false));
    }

    let hunks = crate::git::parse_diff(&diff_text)?;
    Ok((hunks, false))
}

pub fn get_branch_files(branch: &str) -> Result<Vec<FileChange>> {
    let output = Command::new("git")
        .args(["diff", "--no-ext-diff", "--name-status", "HEAD", branch])
        .output()?;
    check_git_output(&output)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    build_files_from_status(&stdout, |path| get_diff_against_branch(path, Some(branch)))
}

pub fn get_commit_files(commit: &str) -> Result<Vec<FileChange>> {
    let output = Command::new("git")
        .args(["diff-tree", "--no-commit-id", "--name-status", "-r", commit])
        .output()?;
    check_git_output(&output)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    build_files_from_status(&stdout, |path| get_commit_diff(path, commit))
}

pub fn get_range_files(left: &str, right: &str) -> Result<Vec<FileChange>> {
    let output = Command::new("git")
        .args(["diff", "--no-ext-diff", "--name-status", left, right])
        .output()?;
    check_git_output(&output)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    build_files_from_status(&stdout, |path| get_diff_for_range(path, left, right))
}

pub fn get_diff_for_range(path: &str, left: &str, right: &str) -> Result<(Vec<Hunk>, bool)> {
    let output = Command::new("git")
        .args(range_diff_args(left, right, path))
        .output()?;
    check_git_output(&output)?;

    let diff_text = String::from_utf8_lossy(&output.stdout);

    if diff_text.contains("Binary files") {
        return Ok((Vec::new(), true));
    }

    if diff_text.trim().is_empty() {
        return Ok((Vec::new(), false));
    }

    let hunks = crate::git::parse_diff(&diff_text)?;
    Ok((hunks, false))
}

fn range_diff_args(left: &str, right: &str, path: &str) -> Vec<String> {
    vec![
        "diff".to_string(),
        "--no-ext-diff".to_string(),
        "-U3".to_string(),
        left.to_string(),
        right.to_string(),
        "--".to_string(),
        path.to_string(),
    ]
}

fn build_files_from_status<F>(stdout: &str, mut get_diff_fn: F) -> Result<Vec<FileChange>>
where
    F: FnMut(&str) -> Result<(Vec<Hunk>, bool)>,
{
    let mut files = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.is_empty() {
            continue;
        }
        let status_line = parts[0];
        let status = status_line.chars().next().unwrap_or(' ');
        let path = if (status == 'R' || status == 'C') && parts.len() >= 3 {
            parts[2]
        } else if parts.len() >= 2 {
            parts[1]
        } else {
            continue;
        };
        let (hunks, is_binary) = get_diff_fn(path)?;
        files.push(FileChange {
            path: path.to_string(),
            status,
            staged_status: None,
            unstaged_status: None,
            hunks,
            is_binary,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_diff_args_builds_command() {
        let args = range_diff_args("main", "feature", "src/main.rs");
        assert_eq!(
            args,
            vec![
                "diff",
                "--no-ext-diff",
                "-U3",
                "main",
                "feature",
                "--",
                "src/main.rs"
            ]
        );
    }
}
