use crate::git::{Hunk, HunkLine, LineType};
use anyhow::Result;

pub fn parse_diff(text: &str) -> Result<Vec<Hunk>> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<Hunk> = None;
    let mut in_hunk = false;

    for line in text.lines() {
        if line.starts_with("@@") {
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }
            current_hunk = Some(Hunk {
                header: line.to_string(),
                lines: Vec::new(),
            });
            in_hunk = true;
        } else if in_hunk && let Some(ref mut hunk) = current_hunk {
            let first = line.chars().next();
            let line_type = match first {
                Some('+') => LineType::Added,
                Some('-') => LineType::Removed,
                Some(' ') => LineType::Context,
                Some('\\') => LineType::NoNewline,
                _ => LineType::Context,
            };
            hunk.lines.push(HunkLine {
                content: line.to_string(),
                line_type,
            });
        }
    }

    if let Some(hunk) = current_hunk {
        hunks.push(hunk);
    }

    Ok(hunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_hunk() {
        let text = "@@ -1,3 +1,3 @@\n context\n-removed\n+added\n";
        let hunks = parse_diff(text).unwrap();
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].header, "@@ -1,3 +1,3 @@");
        assert_eq!(hunks[0].lines.len(), 3);
        assert_eq!(hunks[0].lines[0].line_type, LineType::Context);
        assert_eq!(hunks[0].lines[1].line_type, LineType::Removed);
        assert_eq!(hunks[0].lines[2].line_type, LineType::Added);
    }

    #[test]
    fn parse_binary_marker_returns_empty() {
        let text = "Binary files differ\n";
        let hunks = parse_diff(text).unwrap();
        assert!(hunks.is_empty());
    }

    #[test]
    fn parse_no_newline_marker() {
        let text = "@@ -1,1 +1,1 @@\n context\n\\ No newline at end of file\n";
        let hunks = parse_diff(text).unwrap();
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].lines[1].line_type, LineType::NoNewline);
    }
}
