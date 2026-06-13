use crate::git::FileChange;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_index: usize,
    pub line_number: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct GlobalMatch {
    pub file_path: String,
    pub line_number: usize,
    pub content: String,
    pub match_start: usize,
    pub match_end: usize,
}

pub mod glob;
pub use glob::glob_to_regex;

pub fn search_content_filtered(
    files: &[FileChange],
    query: &str,
    file_pattern: Option<&str>,
) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    let file_regex = file_pattern.and_then(|p| glob_to_regex(p).ok());

    for (file_idx, file) in files.iter().enumerate() {
        if let Some(ref regex) = file_regex
            && !regex.is_match(&file.path)
        {
            continue;
        }
        let mut diff_line_idx = 0;
        for hunk in &file.hunks {
            if hunk.header.to_lowercase().contains(&query_lower) {
                results.push(SearchResult {
                    file_index: file_idx,
                    line_number: Some(diff_line_idx),
                });
            }
            diff_line_idx += 1;
            for line in &hunk.lines {
                if line.content.to_lowercase().contains(&query_lower) {
                    results.push(SearchResult {
                        file_index: file_idx,
                        line_number: Some(diff_line_idx),
                    });
                }
                diff_line_idx += 1;
            }
        }
    }
    results
}

pub fn search_filename(files: &[FileChange], query: &str) -> Vec<SearchResult> {
    let query_lower = query.to_lowercase();
    files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.path.to_lowercase().contains(&query_lower))
        .map(|(i, _f)| SearchResult {
            file_index: i,
            line_number: None,
        })
        .collect()
}

pub fn global_search(query: &str) -> Result<Vec<GlobalMatch>, regex::Error> {
    let regex = Regex::new(query)?;
    let mut results = Vec::new();

    let output = std::process::Command::new("git")
        .args(["ls-files", "-co", "--exclude-standard"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for path in stdout.lines() {
                search_file(path, &regex, &mut results);
            }
        }
        _ => {
            let repo_root = std::env::current_dir().unwrap_or_default();
            let mut stack = vec![repo_root.clone()];
            while let Some(dir) = stack.pop() {
                let Ok(entries) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with('.') || name == "target" {
                        continue;
                    }
                    if path.is_dir() {
                        stack.push(path);
                    } else if path.is_file() {
                        let rel = path.strip_prefix(&repo_root).unwrap_or(&path);
                        search_file(rel.to_str().unwrap_or(""), &regex, &mut results);
                    }
                }
            }
        }
    }

    results.sort_by(|a, b| {
        a.file_path
            .cmp(&b.file_path)
            .then(a.line_number.cmp(&b.line_number))
    });
    Ok(results)
}

fn search_file(path: &str, regex: &Regex, results: &mut Vec<GlobalMatch>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for (line_num, line) in content.lines().enumerate() {
        for m in regex.find_iter(line) {
            results.push(GlobalMatch {
                file_path: path.to_string(),
                line_number: line_num + 1,
                content: line.to_string(),
                match_start: m.start(),
                match_end: m.end(),
            });
        }
    }
}
