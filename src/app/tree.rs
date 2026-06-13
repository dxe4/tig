use crate::app::mode::{Filter, TreeEntry};
use crate::git::{FileChange, LineType};
use std::collections::{BTreeMap, HashSet};

pub fn build_tree_entries(
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
