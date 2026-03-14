//! Terminal setup and event loop

use std::io;
use std::panic;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::branch::Branch;

use super::app::{App, DeletionResult, Mode};
use super::render;

type Term = Terminal<CrosstermBackend<io::Stdout>>;

/// Set up the terminal for TUI rendering: raw mode, alternate screen, and
/// a panic hook that restores the terminal on crash.
fn setup_terminal() -> Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Install a panic hook that restores the terminal before printing the panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state.
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

/// Entry point: set up the terminal, run the event loop, then restore.
pub fn run(app: &mut App) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_loop(&mut terminal, app);
    restore_terminal()?;
    result
}

/// Main event loop: draw, poll for events, dispatch to mode-specific handlers.
fn run_loop(terminal: &mut Term, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| render::draw(frame, app))?;

        // Process deletions: local one-per-frame, remote batched in one push
        if app.mode == Mode::Executing && !app.execution_done {
            if app.pending_deletions.is_empty() && app.deletion_results.is_empty() {
                // First frame: create backup and populate pending_deletions
                prepare_deletions(app);
            }
            if let Some(branch) = app.pending_deletions.first().cloned() {
                if branch.is_remote {
                    // All remaining are remote (locals are processed first).
                    // Batch delete in a single git push for one network round-trip.
                    let remote_branches: Vec<Branch> = app.pending_deletions.drain(..).collect();
                    let names: Vec<String> =
                        remote_branches.iter().map(|b| b.name.clone()).collect();

                    match crate::git::delete_remote_branches_batch(&names) {
                        Ok(results) => {
                            for ((_, success, error), branch) in
                                results.into_iter().zip(remote_branches)
                            {
                                app.deletion_results.push(DeletionResult {
                                    branch,
                                    success,
                                    error,
                                });
                            }
                        }
                        Err(e) => {
                            let err_msg = e.to_string();
                            for branch in remote_branches {
                                app.deletion_results.push(DeletionResult {
                                    branch,
                                    success: false,
                                    error: Some(err_msg.clone()),
                                });
                            }
                        }
                    }
                } else {
                    // Local: delete one per frame for progressive UI
                    app.pending_deletions.remove(0);
                    let result = crate::git::delete_local_branch(&branch.name, app.force);
                    app.deletion_results.push(DeletionResult {
                        branch,
                        success: result.is_ok(),
                        error: result.err().map(|e| e.to_string()),
                    });
                }
            }
            if app.pending_deletions.is_empty() {
                app.execution_done = true;
            }
        }

        // Transition from Executing to Summary when done
        if app.mode == Mode::Executing && app.execution_done {
            app.mode = Mode::Summary;
            continue;
        }

        // Wait for the first event, then drain any already-queued events
        // without blocking. This prevents mouse scroll flooding while keeping
        // keyboard input responsive (no lag on key repeat).
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        loop {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    // Ctrl+C always exits
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        return Ok(());
                    }

                    match app.mode {
                        Mode::Browse => {
                            if handle_browse_key(app, key) {
                                return Ok(());
                            }
                        }
                        Mode::VisualSelect => handle_visual_select_key(app, key),
                        Mode::Filter => handle_filter_key(app, key),
                        Mode::Confirm => {
                            if handle_confirm_key(app, key) {
                                return Ok(());
                            }
                        }
                        Mode::Executing => {
                            // No input during execution
                        }
                        Mode::Summary => {
                            if key.code == KeyCode::Esc {
                                app.apply_deletions_and_reset();
                                app.mode = Mode::Browse;
                            } else {
                                return Ok(());
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse(app, mouse);
                }
                _ => {} // Resize, FocusGained, etc. — ignored
            }
            // Drain remaining queued events without blocking
            if !event::poll(Duration::ZERO)? {
                break;
            }
        }
    }
}

/// Handle key events in Browse mode. Returns true if the app should exit.
fn handle_browse_key(app: &mut App, key: KeyEvent) -> bool {
    // If help is showing, any key dismisses it
    if app.show_help {
        app.show_help = false;
        return false;
    }

    // Handle pending 'g' for gg (jump to top)
    if app.pending_g {
        app.pending_g = false;
        if key.code == KeyCode::Char('g') {
            app.jump_to_top();
            return false;
        }
        // Any other key: cancel pending g, fall through to normal handling
    }

    // Ctrl-key bindings (must precede plain char arms)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('d') => {
                let half = app.table_visible_rows / 2;
                app.page_down(half.max(1));
                return false;
            }
            KeyCode::Char('u') => {
                let half = app.table_visible_rows / 2;
                app.page_up(half.max(1));
                return false;
            }
            KeyCode::Char('f') => {
                app.page_down(app.table_visible_rows.max(1));
                return false;
            }
            KeyCode::Char('b') => {
                app.page_up(app.table_visible_rows.max(1));
                return false;
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Up | KeyCode::Char('k') => app.cursor_up(),
        KeyCode::Down | KeyCode::Char('j') => app.cursor_down(),
        KeyCode::Char('g') => app.pending_g = true,
        KeyCode::Char('G') => app.jump_to_bottom(),
        KeyCode::Char(' ') => app.toggle_selection(),
        KeyCode::Char('a') => app.select_all_merged(),
        KeyCode::Char('A') => app.select_all(),
        KeyCode::Char('n') => app.deselect_all(),
        KeyCode::Char('i') => app.invert_selection(),
        KeyCode::Char('V') => app.enter_visual_select(),
        KeyCode::Char('d') => {
            if app.selected_count() > 0 {
                app.confirm_input.clear();
                app.mode = Mode::Confirm;
            }
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Filter;
        }
        KeyCode::Char('s') => app.cycle_sort(),
        KeyCode::Char('S') => app.toggle_sort_direction(),
        KeyCode::Char('m') => app.toggle_merged_filter(),
        KeyCode::Char('l') => app.toggle_local_filter(),
        KeyCode::Char('R') => app.toggle_remote_filter(),
        KeyCode::Char('?') => app.toggle_help(),
        _ => {}
    }

    false
}

/// Handle key events in VisualSelect mode.
fn handle_visual_select_key(app: &mut App, key: KeyEvent) {
    // Handle pending 'g' for gg (jump to top)
    if app.pending_g {
        app.pending_g = false;
        if key.code == KeyCode::Char('g') {
            app.jump_to_top();
            return;
        }
    }

    // Ctrl-key bindings
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('d') => {
                let half = app.table_visible_rows / 2;
                app.page_down(half.max(1));
                return;
            }
            KeyCode::Char('u') => {
                let half = app.table_visible_rows / 2;
                app.page_up(half.max(1));
                return;
            }
            KeyCode::Char('f') => {
                app.page_down(app.table_visible_rows.max(1));
                return;
            }
            KeyCode::Char('b') => {
                app.page_up(app.table_visible_rows.max(1));
                return;
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.cursor_up(),
        KeyCode::Down | KeyCode::Char('j') => app.cursor_down(),
        KeyCode::Char('g') => app.pending_g = true,
        KeyCode::Char('G') => app.jump_to_bottom(),
        KeyCode::Char(' ') => app.apply_visual_selection(),
        KeyCode::Esc => app.cancel_visual_select(),
        _ => {}
    }
}

/// Handle mouse events (scroll wheel in Browse and VisualSelect modes).
fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    if app.mode != Mode::Browse && app.mode != Mode::VisualSelect {
        return;
    }
    match mouse.kind {
        MouseEventKind::ScrollUp => app.cursor_up(),
        MouseEventKind::ScrollDown => app.cursor_down(),
        _ => {}
    }
}

/// Handle key events in Filter mode.
fn handle_filter_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.search_query.clear();
            app.update_visible();
            app.mode = Mode::Browse;
        }
        KeyCode::Enter => {
            app.mode = Mode::Browse;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.update_visible();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.update_visible();
        }
        _ => {}
    }
}

/// Handle key events in Confirm mode. Returns true if the app should exit.
fn handle_confirm_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.confirm_input.clear();
            app.mode = Mode::Browse;
        }
        KeyCode::Enter => {
            if app.requires_strict_confirm() {
                if app.confirm_input == "yes" {
                    app.mode = Mode::Executing;
                }
            } else {
                app.mode = Mode::Executing;
            }
        }
        KeyCode::Char('y') if !app.requires_strict_confirm() => {
            app.mode = Mode::Executing;
        }
        KeyCode::Char(c) if app.requires_strict_confirm() => {
            app.confirm_input.push(c);
        }
        KeyCode::Backspace => {
            app.confirm_input.pop();
        }
        _ => {}
    }

    false
}

/// Prepare for incremental deletion: collect selected branches (local first,
/// remote second), create a backup, and populate `pending_deletions`.
fn prepare_deletions(app: &mut App) {
    // Collect selected branches
    let selected: Vec<Branch> = app
        .selected
        .iter()
        .enumerate()
        .filter(|(_, &s)| s)
        .map(|(i, _)| app.all_branches[i].clone())
        .collect();

    let local: Vec<_> = selected.iter().filter(|b| !b.is_remote).cloned().collect();
    let remote: Vec<_> = selected.iter().filter(|b| b.is_remote).cloned().collect();

    // Create backup for all selected branches
    let all_to_backup: Vec<_> = local.iter().chain(remote.iter()).cloned().collect();
    if !all_to_backup.is_empty() {
        match crate::create_backup_file(&all_to_backup) {
            Ok(path) => app.backup_path = Some(path),
            Err(e) => app.backup_path = Some(format!("backup failed: {}", e)),
        }
    }

    // Fetch and prune if any remote branches are selected
    if !remote.is_empty() {
        let _ = crate::git::fetch_and_prune();
    }

    // Populate pending_deletions: local first, then remote
    app.pending_deletions = local.into_iter().chain(remote).collect();
}
