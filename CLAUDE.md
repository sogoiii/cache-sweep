# CLAUDE.md

Guidance for Claude Code when working with this repository.

## Project Overview

`cache-sweep` - Rust CLI for finding/deleting dependency and cache folders (node_modules, .venv, target, etc.). TUI-based, inspired by npkill.

**Stack:** tokio (async), ratatui (TUI), clap (CLI), ignore (parallel walking)

## Commands

```bash
cargo build --release
cargo run -- -d ~/projects              # TUI mode
cargo run -- --json -d . -p node        # JSON output
cargo test                               # Run tests
cargo clippy                             # Lint check
```

## File Structure

```
src/
├── main.rs              # Entry point, tokio runtime
├── cli/
│   └── args.rs          # Clap definitions
├── scanner/
│   ├── walker.rs        # Parallel traversal (build_parallel)
│   ├── batcher.rs       # Result batching (50/send)
│   └── size.rs          # Async size calculation
├── profiles/
│   └── builtin.rs       # 17 profiles (node, python, rust, etc.)
├── risk/
│   └── analysis.rs      # Sensitive directory detection
├── delete/
│   └── engine.rs        # Async deletion
├── tui/
│   ├── app.rs           # Application state (CRITICAL: stable indices)
│   ├── event_loop.rs    # tokio::select! with biased
│   ├── input.rs         # Mode-based key handling
│   ├── ui.rs            # Ratatui rendering
│   ├── cleanup.rs       # Terminal RAII guard
│   ├── analytics.rs     # Real-time aggregation
│   ├── widgets/
│   │   └── gradient_bar.rs  # Dual progress bar
│   └── panels/
│       ├── results.rs
│       ├── info.rs
│       └── analytics.rs
└── output/
    ├── json.rs          # Complete JSON
    └── stream.rs        # Streaming JSON (NDJSON)
```

## Architecture Constraints (Non-Negotiable)

### Streaming Results
- **NO `.collect::<Vec<_>>()`** on filesystem walks
- Stream via unbounded channel, drain fully on each recv
- Use `spawn_blocking` with `ignore::WalkBuilder::build_parallel()`

### Stable Indices Pattern
- **NEVER reorder `results` vec** - indices used by async size calculations
- Sort `filtered_indices` (indices into results), not results themselves
- Size updates use raw index, display uses filtered order

### Walker Config
```rust
WalkBuilder::new(&root)
    .hidden(false)           // Scan hidden dirs
    .follow_links(false)     // SAFETY: never follow symlinks
    .git_ignore(false)       // Don't skip targets
    .build_parallel()
```

### Event Loop
- `crossterm::EventStream` + `tokio::select!` with `biased;` for input priority
- Tick-based debouncing via `needs_sort`/`needs_filter` flags
- Terminal cleanup via RAII (`TerminalCleanupGuard`)

## Coding Standards

### Clippy
- **Required:** `pedantic`, `perf`, `nursery` warnings enabled
- **Forbidden:** `unsafe_code`
- When bypassing lint, add explanatory `#[allow(clippy::...)]` comment:
```rust
#[allow(clippy::struct_excessive_bools)] // TUI state naturally tracks multiple boolean flags
pub struct App { ... }

#[allow(clippy::too_many_lines)] // Event loop is inherently complex; splitting would obscure flow
pub async fn run(...) { ... }
```

### Testing
- Unit tests in same file under `#[cfg(test)] mod tests`
- Helper functions for test setup (e.g., `fn app_with_items(...)`)
- Test edge cases: empty, boundary, wraparound
- Name tests descriptively: `test_<function>_<scenario>`

### Widget Pattern (Builder)
```rust
pub struct MyWidget { ... }

impl MyWidget {
    pub const fn new() -> Self { ... }
    pub const fn some_prop(mut self, val: T) -> Self { self.prop = val; self }
}

impl Widget for MyWidget {
    fn render(self, area: Rect, buf: &mut Buffer) { ... }
}
```

### Mode-Based Input
- Separate handler per mode: `handle_normal_key`, `handle_search_key`, etc.
- Return `Action` enum (Continue, Quit, Delete, etc.)
- Panel-specific handling before mode dispatch

### State Management
- Deferred operations: set `needs_*` flag, process on tick
- Never mutate + rebuild in same call
- `rebuild_display_indices()` handles filter + sort + cursor adjustment

### Error Handling
- `thiserror` for custom errors, `anyhow` for propagation
- Collect errors in `app.errors` for display
- Use `.ok()` for fire-and-forget channel sends

### Const Functions
- Use `const fn` for constructors and simple getters when possible
- Add `#[allow(clippy::missing_const_for_fn)]` comment if `&mut self` prevents it

## Key Crates

| Purpose | Crate |
|---------|-------|
| Async | `tokio` (rt-multi-thread, sync, fs, time, signal) |
| Walking | `ignore` (build_parallel) |
| TUI | `ratatui` + `crossterm` (EventStream) |
| CLI | `clap` (derive) |
| Cancel | `tokio_util::sync::CancellationToken` |
| Errors | `thiserror` + `anyhow` |
| JSON | `serde` + `serde_json` |
| Sizes | `bytesize` |

## Profiles

17 built-in: node, python, data-science, java, android, swift, dotnet, rust, ruby, elixir, haskell, scala, cpp, unity, unreal, godot, infra

Default: `--profiles all` (scans all targets)
