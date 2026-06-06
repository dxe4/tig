#!/bin/bash
# Script to create sample diffs for testing tig-review

set -e

cd "$(dirname "$0")"

echo "Creating diffs with multiple hunks..."

# Modify multi-hunk.txt - changes are far apart so git creates separate hunks
sed -i \
    -e 's/Line 11: Original line eleven./Line 11: CHANGED - this was originally line eleven./' \
    -e 's/Line 12: Original line twelve./Line 12: Another changed line in first hunk./' \
    -e 's/Line 24: Original line twenty-four./Line 24: CHANGED - this was originally line twenty-four./' \
    -e 's/Line 25: Original line twenty-five./Line 25: Also changed in the second hunk./' \
    -e 's/Line 37: Original line thirty-seven./Line 37: CHANGED - this was originally line thirty-seven./' \
    -e 's/Line 38: Original line thirty-eight./Line 38: Also changed in the third hunk./' \
    -e 's/Line 50: Original line fifty./Line 50: CHANGED - this was originally line fifty./' \
    -e 's/Line 51: Original line fifty-one./Line 51: Also changed in the fourth hunk./' \
    demo/multi-hunk.txt

# Modify config.toml with separated changes
cat > demo/config.toml << 'EOF'
[app]
name = "tig-review"
version = "0.2.0"
author = "Harry"
description = "A minimal TUI for reviewing git changes"

[ui]
theme = "gruvbox"
show_line_numbers = true
show_whitespace = false

[keybindings]
quit = "q"
copy = "y"
search = "/"
next_result = "n"
prev_result = "N"
EOF

# Modify README.md with separated changes
cat > demo/README.md << 'EOF'
# Demo Project

This is a test file for demonstrating the tig-review TUI application.
It has been updated to show various diff types.

## Features

- Tree view navigation with collapsible directories
- Hunk selection and copying
- Case-insensitive search by content or filename
- Vim-style keybindings

## Installation

```bash
cargo build --release
```

## Usage

Run the app with `cargo run --release`.
Use `h`/`j`/`k`/`l` to navigate, `/` to search, `y` to copy hunks.
EOF

# Add a new untracked file
cat > demo/CHANGELOG.md << 'EOF'
# Changelog

## 0.2.0

- Added tree view for file navigation
- Added hunk-level navigation with J/K
- Added search highlighting in diff view
- Added case-insensitive search
EOF

# Create a nested directory with a new file
mkdir -p demo/subdir
cat > demo/subdir/nested.txt << 'EOF'
This is a nested file inside a subdirectory.
It helps test the tree view rendering.
EOF

echo "Done! Run './target/release/tig-review' to see the diffs."
echo ""
echo "multi-hunk.txt has 4 separate hunks for testing hunk navigation."
