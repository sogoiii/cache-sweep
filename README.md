# cache-sweep

A fast, interactive terminal tool for finding and deleting dependency and cache folders to reclaim disk space. Think of it as a smarter `rm -rf node_modules` that works across all your projects at once.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Why?

Over time, development projects accumulate massive cache and dependency folders:
- `node_modules` can easily reach 500MB+ per project
- Python's `.venv` and `__pycache__` add up quickly
- Rust's `target` folders often exceed 1GB each
- Build outputs like `.next`, `dist`, and `build` consume space silently

If you have dozens of projects, you might be losing **tens of gigabytes** to folders you don't actively need.

**cache-sweep** helps you:
- Find all these folders across your entire system
- See exactly how much space each one uses
- Delete them safely with a single keypress
- Filter by project type (Node, Python, Rust, etc.)

## Installation

### From Source

```bash
git clone https://github.com/sogoiii/cache-sweep.git
cd cache-sweep
cargo build --release
# Binary will be at ./target/release/cache-sweep
```

### Add to PATH (optional)

```bash
# macOS/Linux
cp ./target/release/cache-sweep /usr/local/bin/

# Or add to your shell config
export PATH="$PATH:/path/to/cache-sweep/target/release"
```

## Quick Start

```bash
# Scan current directory and subdirectories
cache-sweep

# Scan your entire home directory
cache-sweep -f

# Scan a specific folder
cache-sweep -d ~/projects

# Only look for Node.js caches
cache-sweep -p node

# Only look for Python and Rust caches
cache-sweep -p python,rust
```

## Command-Line Options

### Directory Options

| Flag | Description | Example |
|------|-------------|---------|
| `-d, --directory <PATH>` | Start scanning from this directory | `cache-sweep -d ~/code` |
| `-f, --full` | Scan from your home directory | `cache-sweep -f` |

### Filtering Options

| Flag | Description | Example |
|------|-------------|---------|
| `-p, --profiles <LIST>` | Only scan for specific project types (comma-separated) | `cache-sweep -p node,python` |
| `-t, --targets <LIST>` | Search for specific folder names (overrides profiles) | `cache-sweep -t node_modules,.cache` |
| `-E, --exclude <LIST>` | Skip folders by name (not path) | `cache-sweep -E my_project,old_app` |

### Display Options

| Flag | Description | Example |
|------|-------------|---------|
| `-s, --sort <TYPE>` | Sort results by: `size` (default), `path`, or `age` | `cache-sweep -s age` |

### Output Modes

| Flag | Description | Use Case |
|------|-------------|----------|
| `--json` | Output all results as a single JSON object | Scripting, analysis |
| `--json-stream` | Stream results as newline-delimited JSON | Piping to other tools |

### Safety Options

| Flag | Description |
|------|-------------|
| `--dry-run` | Simulate deletions in TUI without actually deleting |
| `-X, --show-protected` | Include sensitive system directories in results (hidden by default) |
| `--follow-links` | Follow symbolic links (disabled by default for safety) |
| `--respect-ignore` | Honor `.gitignore` files (disabled by default to find everything) |

### Other

| Flag | Description |
|------|-------------|
| `-h, --help` | Show help message |
| `-v, --version` | Show version |

## Available Profiles

Profiles are predefined sets of folder names to search for. Use `-p` to select one or more:

| Profile | What it finds |
|---------|---------------|
| `node` | `node_modules`, `.npm`, `.yarn`, `.pnpm-store`, `.next`, `.nuxt`, `.turbo`, `dist`, `build`, `.parcel-cache`, `.cache` |
| `python` | `.venv`, `venv`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `*.egg-info`, `.tox`, `.nox` |
| `rust` | `target` |
| `java` | `target`, `.gradle`, `build` |
| `android` | `.gradle`, `build`, `.cxx` |
| `swift` | `.build`, `DerivedData`, `.swiftpm` |
| `dotnet` | `bin`, `obj`, `packages` |
| `ruby` | `vendor/bundle`, `.bundle` |
| `elixir` | `_build`, `deps`, `.elixir_ls` |
| `haskell` | `.stack-work`, `dist-newstyle` |
| `scala` | `target`, `.bloop`, `.metals`, `.bsp` |
| `cpp` | `build`, `cmake-build-*`, `.cache` |
| `unity` | `Library`, `Temp`, `Obj`, `Logs` |
| `unreal` | `Intermediate`, `Saved`, `DerivedDataCache` |
| `godot` | `.godot`, `.import` |
| `data-science` | `.ipynb_checkpoints`, `mlruns`, `wandb`, `.dvc` |
| `infra` | `.terraform`, `.terragrunt-cache`, `.pulumi` |

Use `all` to scan for everything: `cache-sweep -p all` (this is the default).

## Using the Interactive TUI

When you run `cache-sweep` without `--json` flags, you get an interactive terminal interface.

### Main Screen Layout

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  cache-sweep | 3 results | 3.8 GiB potential | 0 B freed | sort:SIZE         │
└──────────────────────────────────────────────────────────────────────────────┘
All (3) | target (1) | node_modules (2)                         ← Tab to filter
┌─ Results - SPACE to delete ──────────────────────────────────────────────────┐
│  Path                                                      Last_mod     Size │
│ >/Users/you/projects/webapp/node_modules                        1d  1.2 GiB  │
│  /Users/you/projects/api/node_modules                           3d  892 MiB  │
│  /Users/you/code/rust-tool/target                               0d  654 MiB  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
Tab/⇧Tab:switch | ↑/↓:nav | /:search | s:sort | v:multi | SPACE:del | q:quit
```

### Keyboard Controls

#### Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Move cursor up |
| `↓` / `j` | Move cursor down |
| `Page Up` / `u` | Move up one page |
| `Page Down` / `d` | Move down one page |
| `Home` | Jump to first item |
| `End` | Jump to last item |
| `Tab` | Next filter tab (by folder type) |
| `Shift+Tab` | Previous filter tab |

#### Actions

| Key | Action |
|-----|--------|
| `Space` or `Delete` | Delete the selected folder |
| `/` | Enter search mode (filter by path) |
| `s` | Cycle sort order: Size → Path → Age |
| `v` | Enter multi-select mode |
| `a` | Open analytics panel |
| `q` or `Esc` | Quit |

#### Multi-Select Mode

Press `v` to enter multi-select mode for batch deletions:

| Key | Action |
|-----|--------|
| `Space` | Toggle selection on current item |
| `a` | Select all / Deselect all |
| `Enter` | Delete all selected (asks for confirmation) |
| `v` or `Esc` | Exit multi-select mode |

#### Panels

| Key | Action |
|-----|--------|
| `←` / `h` | Switch to results panel |
| `→` / `l` | Switch to info panel (shows details about selected item) |
| `a` | Toggle analytics panel (shows breakdown by type) |
| `o` | Open selected folder in file explorer (from info panel) |

### Understanding the Display

**Progress Bars:**
- Left bar: Scanning progress (cyan while scanning, green when done)
- Right bar: Size calculation progress (shows how many folders have been measured)

**Colors:**
- Green sizes: Small folders (< 100 MiB)
- Yellow sizes: Medium folders (100 MiB - 500 MiB)
- Red sizes: Large folders (> 500 MiB)
- Cyan highlight: Currently selected item
- Yellow background: Item marked for multi-select

**Age Indicators:**
- Green: Recently modified (< 1 month)
- Yellow: Moderately old (1-6 months)
- Red: Stale (> 6 months) — good candidates for deletion!

## Example Workflows

### Workflow 1: Quick cleanup of old Node projects

```bash
# Find all node_modules folders, sorted by age (oldest first)
cache-sweep -p node -s age
```

Then in the TUI:
1. Old projects appear at the top
2. Press `Space` on each one you want to delete
3. Or press `v` for multi-select, then `a` to select all, then `Enter`

### Workflow 2: Free up space on a full disk

```bash
# Scan everything from home, largest first
cache-sweep -f
```

Then in the TUI:
1. Largest folders appear at the top
2. Review each one and press `Space` to delete
3. Watch the "Freed" counter grow

### Workflow 3: Clean up before backing up

```bash
# See what would be cleaned without deleting
cache-sweep -f --dry-run
```

Or in TUI mode, just review without pressing `Space`.

### Workflow 4: Get JSON for scripting

```bash
# Get JSON output for processing
cache-sweep -d /workspace --json | jq '.results[].path'
```

### Workflow 5: Find Rust targets specifically

```bash
cache-sweep -t target -d ~/rust-projects
```

## JSON Output

### Complete JSON (`--json`)

```bash
cache-sweep -d ~/projects --json
```

Output:
```json
{
  "results": [
    {
      "path": "/Users/you/projects/app/node_modules",
      "size": 524288000,
      "file_count": 45231,
      "modified": "2024-01-15T10:30:00Z"
    }
  ],
  "total_size": 524288000,
  "total_count": 1
}
```

### Streaming JSON (`--json-stream`)

```bash
cache-sweep -d ~/projects --json-stream
```

Output (one object per line):
```json
{"path":"/Users/you/projects/app/node_modules","size":524288000}
{"path":"/Users/you/projects/api/node_modules","size":312000000}
```

Useful for piping:
```bash
cache-sweep --json-stream | jq -r 'select(.size > 100000000) | .path'
```

## Safety Features

1. **Sensitive directories are protected** — Folders inside system paths (`/Applications`, `~/.config`, `~/.vscode`, `~/Library`, etc.) **cannot be deleted**. Attempting to delete them shows a blocking modal. This prevents accidentally breaking your OS or installed applications.

2. **No symlink following** — By default, symbolic links are not followed to prevent accidentally deleting linked system directories.

3. **Sensitive directories hidden by default** — Folders in system paths are hidden from results. Use `-X` to show them if needed.

4. **Dry run mode** — Use `--dry-run` to see what would be deleted without actually deleting anything.

5. **Confirmation for batch deletes** — Multi-select deletions always ask for confirmation (press `y` to confirm).

6. **Visual feedback** — Deleted items are immediately removed from the list and the "Freed" counter updates in real-time.

### What counts as "sensitive"?

Directories are marked sensitive (shown with ⚠️) if they're inside:
- System paths: `/Applications`, `/Library`, `/System`, `Program Files`, `AppData`
- User config: `~/.config`, `~/.local/share`, `~/.vscode`
- Known apps: VS Code, Discord, Slack, Obsidian, Notion, 1Password, etc.

## Tips

- **Start with your projects folder**, not your entire home directory, for faster scans
- **Use profiles** to focus on what you care about: `-p node` is much faster than scanning everything
- **Sort by age** (`-s age`) to find forgotten projects that are safe to clean
- **Check the info panel** (press `→`) to see the full path and project name before deleting
- **Use analytics** (press `a`) to see which folder types are using the most space

## Troubleshooting

**"Permission denied" errors**
- Some system folders can't be scanned. Use `-x` to skip sensitive directories.

**Scan is slow**
- Large directories with many files take time. The progress bar shows scanning status.
- Use `-p` to limit to specific profiles instead of scanning everything.

**Can't find expected folders**
- By default, `.gitignore` is not respected. Check if the folder exists with `ls`.
- Hidden folders (starting with `.`) are scanned by default.

## License

MIT License - see [LICENSE](LICENSE) file.

## Contributing

Contributions welcome! Please open an issue or PR on GitHub.
