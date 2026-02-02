use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::app::{App, SortOrder};
use super::cleanup::TerminalCleanupGuard;
use super::input::{handle_key, Action};
use super::ui;
use crate::cli::Args;
use crate::delete::delete_directory;
use crate::scanner::{calculate_size, start_scan};

enum Command {
    Delete(usize),
    DeleteBatch(Vec<usize>),
}

#[allow(clippy::too_many_lines)] // Event loop is inherently complex; splitting would obscure flow
pub async fn run(args: &Args, cancel_token: CancellationToken) -> Result<()> {
    let (_guard, mut terminal) = TerminalCleanupGuard::new()?;
    let sort_order = SortOrder::from_str(&args.sort);
    let mut app = App::new(args.exclude_sensitive, sort_order);

    // Set visible height based on terminal
    app.visible_height = terminal.size()?.height.saturating_sub(8) as usize;

    // Start scanner
    let root = args.effective_directory();
    let targets = args.effective_targets();
    let excludes = args.exclude.clone().unwrap_or_default();
    let mut result_rx = start_scan(
        root,
        targets,
        excludes,
        args.follow_links,
        args.respect_ignore,
        cancel_token.clone(),
    );

    // Command channel for deletions
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(10);

    // Size calculation queue: (index, size, file_count)
    let (size_tx, mut size_rx) = mpsc::unbounded_channel::<(usize, u64, u64)>();

    let mut event_stream = EventStream::new();
    let mut tick = tokio::time::interval(Duration::from_millis(100));

    let dry_run = args.dry_run;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Use biased to prioritize keyboard input
        tokio::select! {
            biased;

            // Keyboard events (highest priority)
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                        match handle_key(key, &mut app) {
                            Action::Quit => break,
                            Action::Delete => {
                                if let Some(idx) = app.current_index() {
                                    if let Some(item) = app.results.get(idx) {
                                        if !item.is_deleted && !item.is_deleting {
                                            cmd_tx.send(Command::Delete(idx)).await.ok();
                                        }
                                    }
                                }
                            }
                            Action::DeleteSelected => {
                                let indices: Vec<usize> = app.selected_indices.iter().copied().collect();
                                if !indices.is_empty() {
                                    cmd_tx.send(Command::DeleteBatch(indices)).await.ok();
                                }
                            }
                            Action::OpenInExplorer => {
                                if let Some(idx) = app.current_index() {
                                    if let Some(item) = app.results.get(idx) {
                                        let path = &item.scan_result.path;
                                        #[cfg(target_os = "macos")]
                                        let _ = std::process::Command::new("open").arg(path).spawn();
                                        #[cfg(target_os = "linux")]
                                        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
                                        #[cfg(target_os = "windows")]
                                        let _ = std::process::Command::new("explorer").arg(path).spawn();
                                    }
                                }
                            }
                            Action::Continue => {}
                        }
                    }
                    Some(Ok(Event::Resize(_, height))) => {
                        app.visible_height = height.saturating_sub(8) as usize;
                    }
                    _ => {}
                }
            }

            // Size updates
            Some((idx, size, file_count)) = size_rx.recv() => {
                app.update_size(idx, size, file_count);
            }

            // Deletion commands
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    Command::Delete(idx) => {
                        if let Some(item) = app.results.get(idx) {
                            let path = item.scan_result.path.clone();
                            let size = item.scan_result.size.unwrap_or(0);
                            app.mark_deleting(idx);

                            let result = delete_directory(&path, dry_run).await;
                            if result.success {
                                app.mark_deleted(idx, size);
                            } else if let Some(err) = result.error {
                                app.add_error(format!("{}: {}", path.display(), err));
                            }
                        }
                    }
                    Command::DeleteBatch(indices) => {
                        for idx in indices {
                            if let Some(item) = app.results.get(idx) {
                                let path = item.scan_result.path.clone();
                                let size = item.scan_result.size.unwrap_or(0);
                                app.mark_deleting(idx);

                                let result = delete_directory(&path, dry_run).await;
                                if result.success {
                                    app.mark_deleted(idx, size);
                                } else if let Some(err) = result.error {
                                    app.add_error(format!("{}: {}", path.display(), err));
                                }
                            }
                        }
                    }
                }
            }

            // Scan results - drain fully
            batch = result_rx.recv() => {
                match batch {
                    Some(results) => {
                        let start_idx = app.results.len();
                        app.add_results(results);

                        // Queue size calculations for new results
                        let size_tx = size_tx.clone();
                        for idx in start_idx..app.results.len() {
                            let path = app.results[idx].scan_result.path.clone();
                            let tx = size_tx.clone();
                            tokio::spawn(async move {
                                let (size, file_count) = calculate_size(&path).await;
                                tx.send((idx, size, file_count)).ok();
                            });
                        }

                        // Drain ALL available batches (don't cap)
                        while let Ok(more_results) = result_rx.try_recv() {
                            let start_idx = app.results.len();
                            app.add_results(more_results);

                            for idx in start_idx..app.results.len() {
                                let path = app.results[idx].scan_result.path.clone();
                                let tx = size_tx.clone();
                                tokio::spawn(async move {
                                    let (size, file_count) = calculate_size(&path).await;
                                    tx.send((idx, size, file_count)).ok();
                                });
                            }
                        }
                    }
                    None => {
                        // Scanner finished
                        app.scan_complete();
                    }
                }
            }

            // Tick for animations and throttled operations
            _ = tick.tick() => {
                app.on_tick();
            }
        }

        // Check cancellation
        if cancel_token.is_cancelled() {
            break;
        }
    }

    Ok(())
}
