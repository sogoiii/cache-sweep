# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

**Implemented.** Core functionality complete with TUI, JSON output modes, and multi-profile support.

## What This Is

`cache-sweep` is a Rust CLI tool for finding and deleting dependency/cache folders (node_modules, .venv, target, etc.). TUI-based, inspired by npkill. Uses **tokio** for async and **ratatui** for terminal UI.

## Build Commands

```bash
cargo build --release
cargo run -- --profiles node -d ~/projects
cargo test
./target/release/cache-sweep --json -d . -t node_modules
./target/release/cache-sweep --json-stream -d . -p node,python
```

## Critical Architecture Decisions (v3)

These constraints are **non-negotiable** based on multi-reviewer consensus:

### Streaming (NOT Batching Everything)
- **NO `.collect::<Vec<_>>()`** on filesystem walks
- Stream results immediately via channel as found
- Use `spawn_blocking` with `ignore::WalkBuilder::build_parallel()`

### Channel Architecture
- Results channel: **unbounded**
- Use `.send()` for unbounded (NOT `blocking_send`)
- **Drain channel fully** on each recv (don't cap at N batches)

### Walker Configuration
```rust
ignore::WalkBuilder::new(&root)
    .hidden(false)           // Scan hidden dirs (.pnpm-store, .yarn)
    .follow_links(false)     // SAFETY: never follow symlinks
    .git_ignore(false)       // Don't skip targets
    .build_parallel()
```

### Target Matching
- Return `WalkState::Skip` when target matched (don't descend into node_modules/)
- Batch 50 results per send via `ResultBatcher`

### TUI Event Loop
- Use `crossterm::EventStream` with `tokio::select!`
- Add `biased;` to prioritize keyboard input over results
- Throttle sorting (tick-based, not per-message)
- Terminal cleanup via RAII Drop guard (`TerminalCleanupGuard`)

## File Structure

```
src/
├── main.rs              # Entry, tokio runtime, CLI dispatch
├── cli/
│   ├── mod.rs
│   └── args.rs          # Clap argument definitions
├── scanner/
│   ├── mod.rs
│   ├── walker.rs        # Streaming traversal with build_parallel()
│   ├── batcher.rs       # Result batching (50 per send)
│   └── size.rs          # Async size calculation (semaphore-limited)
├── profiles/
│   ├── mod.rs
│   └── builtin.rs       # 17 built-in profiles (node, python, rust, etc.)
├── risk/
│   ├── mod.rs
│   └── analysis.rs      # Sensitive directory detection
├── delete/
│   ├── mod.rs
│   └── engine.rs        # Async deletion
├── tui/
│   ├── mod.rs
│   ├── app.rs           # Application state
│   ├── cleanup.rs       # Terminal RAII cleanup guard
│   ├── event_loop.rs    # tokio::select! with biased for input priority
│   ├── input.rs         # Keyboard handling
│   ├── ui.rs            # Ratatui rendering
│   └── panels/
│       ├── mod.rs
│       ├── results.rs
│       ├── info.rs
│       ├── options.rs
│       └── help.rs
└── output/
    ├── mod.rs
    ├── json.rs          # Complete JSON output
    └── stream.rs        # Streaming JSON (one object per line)
```

## Key Crates

| Purpose | Crate |
|---------|-------|
| Async runtime | `tokio` (rt-multi-thread, sync, fs, signal) |
| Directory walking | `ignore` (build_parallel) |
| TUI | `ratatui` + `crossterm` (EventStream) |
| CLI | `clap` (derive) |
| Cancellation | `tokio_util::sync::CancellationToken` |
| Errors | `thiserror` + `anyhow` |
| JSON | `serde` + `serde_json` |

## Profiles

17 built-in profiles: node, python, data-science, java, android, swift, dotnet, rust, ruby, elixir, haskell, scala, cpp, unity, unreal, godot, infra

Use `--profiles all` to scan all targets, or combine with `--profiles node,python`
