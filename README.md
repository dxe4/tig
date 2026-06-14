# tig-review

A terminal UI for reviewing git diffs.

## Usage

```bash
# Working tree diff (default)
tig-review

# Show a single commit's diff
tig-review <commit-hash>

# Show diff between two refs
tig-review <left>:<right>
```

Examples:

```bash
tig-review abc123
tig-review main:feature-branch
tig-review HEAD~3:HEAD
```

## Controls

| Key | Action |
|-----|--------|
| `q` | Quit |
| `j`/`k` or `↓`/`↑` | Navigate |
| `h`/`l` or `←`/`→` | Switch focus / toggle directories |
| `b` | Select or clear branch comparison |
| `c` | Select or clear commit comparison |
| `r` | Reload files |
| `/` | Search file content |
| `f` | Search file names |
| `?` | Show help |
