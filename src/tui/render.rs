//! All screen rendering for the TUI

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::app::{App, Mode, SortOrder};

// ── Unicode characters ──────────────────────────────────────────────

const CURSOR: &str = "\u{25b6}"; // ▶
const DOT: &str = "\u{00b7}"; // ·
const BLOCK: &str = "\u{2588}"; // █
const SHADE: &str = "\u{2591}"; // ░
const CHECK: &str = "\u{2713}"; // ✓
const CROSS: &str = "\u{2717}"; // ✗
const WARN: &str = "\u{26a0}"; // ⚠

// ── Colours ─────────────────────────────────────────────────────────

const GREEN: Color = Color::Green;
const YELLOW: Color = Color::Yellow;
const RED: Color = Color::Red;
const CYAN: Color = Color::Cyan;
const BLUE: Color = Color::Blue;
const GRAY: Color = Color::DarkGray;
const WHITE: Color = Color::White;

// ── Main draw dispatch ──────────────────────────────────────────────

/// Top-level draw function: dispatches to mode-specific renderers.
pub fn draw(frame: &mut Frame, app: &App) {
    match app.mode {
        Mode::Browse | Mode::Filter => draw_browse(frame, app),
        Mode::Confirm => draw_confirm(frame, app),
        Mode::Executing => draw_executing(frame, app),
        Mode::Summary => draw_summary(frame, app),
    }

    if app.show_help {
        draw_help_overlay(frame);
    }
}

// ── Browse mode ─────────────────────────────────────────────────────

fn draw_browse(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Min(1),    // branch list
        Constraint::Length(3), // status bar (2 lines + border)
    ])
    .split(area);

    draw_header(frame, app, chunks[0]);
    draw_branch_list(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let total = app.all_branches.len();
    let visible = app.visible.len();

    let branch_count = if visible < total {
        format!("{} of {} branches", visible, total)
    } else {
        format!("{} branches", total)
    };

    let mut parts: Vec<Span> = vec![
        Span::styled(
            " deadbranch clean",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                " \u{2500}\u{2500} {} \u{2500}\u{2500} {}",
                app.default_branch, branch_count
            ),
            Style::default().fg(GRAY),
        ),
    ];

    // Active filters
    let mut filters = Vec::new();
    if app.filter_merged_only {
        filters.push("merged");
    }
    if app.filter_local_only {
        filters.push("local");
    }
    if app.filter_remote_only {
        filters.push("remote");
    }
    if !filters.is_empty() {
        parts.push(Span::styled(
            format!("  [{}]", filters.join(", ")),
            Style::default().fg(CYAN),
        ));
    }

    // Sort order
    if app.sort_order != SortOrder::Age {
        parts.push(Span::styled(
            format!("  sort:{}", app.sort_order.label()),
            Style::default().fg(GRAY),
        ));
    }

    // Filter mode: show query
    if app.mode == Mode::Filter {
        parts.push(Span::styled("  filter: ", Style::default().fg(YELLOW)));
        parts.push(Span::styled(&app.search_query, Style::default().fg(WHITE)));
        parts.push(Span::styled(BLOCK, Style::default().fg(YELLOW)));
    } else if !app.search_query.is_empty() {
        parts.push(Span::styled(
            format!("  /{}", app.search_query),
            Style::default().fg(GRAY),
        ));
    }

    let header = Paragraph::new(Line::from(parts));
    frame.render_widget(header, area);
}

fn draw_branch_list(frame: &mut Frame, app: &App, area: Rect) {
    if app.visible.is_empty() {
        let msg = if app.search_query.is_empty() {
            "No branches match current filters"
        } else {
            "No branches match search query"
        };
        let paragraph = Paragraph::new(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(GRAY),
        )));
        frame.render_widget(paragraph, area);
        return;
    }

    let list_height = area.height as usize;
    let mut lines: Vec<Line> = Vec::new();

    // Build all lines with their row indices (for scrolling)
    let mut row_lines: Vec<(Option<usize>, Line)> = Vec::new();
    let mut last_was_merged: Option<bool> = None;

    for (row_idx, &branch_idx) in app.visible.iter().enumerate() {
        let branch = &app.all_branches[branch_idx];

        // Section headers
        if last_was_merged != Some(branch.is_merged) {
            if branch.is_merged {
                row_lines.push((
                    None,
                    Line::from(Span::styled(
                        "  MERGED (safe to delete)",
                        Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                    )),
                ));
            } else {
                let label = if app.force {
                    "  UNMERGED (review carefully)"
                } else {
                    "  UNMERGED (use --force to unlock)"
                };
                row_lines.push((
                    None,
                    Line::from(Span::styled(
                        label,
                        Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
                    )),
                ));
            }
            last_was_merged = Some(branch.is_merged);
        }

        // Branch row
        let is_focused = row_idx == app.cursor;
        let is_locked = !branch.is_merged && !app.force;
        let is_selected = app.selected[branch_idx];

        let cursor_span = if is_focused {
            Span::styled(format!(" {} ", CURSOR), Style::default().fg(WHITE))
        } else {
            Span::raw("   ")
        };

        let checkbox_span = if is_locked {
            Span::styled(format!("{}{}", SHADE, SHADE), Style::default().fg(GRAY))
        } else if is_selected {
            Span::styled("[x]", Style::default().fg(GREEN))
        } else {
            Span::styled("[ ]", Style::default().fg(GRAY))
        };

        let name_style = if is_locked {
            Style::default().fg(GRAY)
        } else if is_focused {
            Style::default().fg(WHITE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(WHITE)
        };

        // Truncate name to 30 chars
        let display_name = if branch.name.len() > 30 {
            format!("{}..", &branch.name[..28])
        } else {
            branch.name.clone()
        };
        let name_span = Span::styled(format!(" {:<30}", display_name), name_style);

        let age_span = Span::styled(
            format!("{:>4}d", branch.age_days),
            Style::default().fg(GRAY),
        );

        let status_span = if branch.is_merged {
            Span::styled(" merged  ", Style::default().fg(GREEN))
        } else {
            Span::styled(" unmerged", Style::default().fg(YELLOW))
        };

        let type_span = if branch.is_remote {
            Span::styled(" remote", Style::default().fg(BLUE))
        } else {
            Span::styled(" local ", Style::default().fg(CYAN))
        };

        let date_str = branch.last_commit_date.format("%Y-%m-%d").to_string();
        let date_span = Span::styled(format!("  {}", date_str), Style::default().fg(GRAY));

        let line = Line::from(vec![
            cursor_span,
            checkbox_span,
            name_span,
            age_span,
            status_span,
            type_span,
            date_span,
        ]);

        row_lines.push((Some(row_idx), line));
    }

    // Compute scroll offset to keep cursor visible
    let scroll_offset = compute_scroll_offset(app, &row_lines, list_height);

    for (_, line) in row_lines.into_iter().skip(scroll_offset).take(list_height) {
        lines.push(line);
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Compute the scroll offset so the cursor row stays visible.
fn compute_scroll_offset(
    app: &App,
    row_lines: &[(Option<usize>, Line)],
    list_height: usize,
) -> usize {
    // Find the line index of the cursor
    let cursor_line = row_lines
        .iter()
        .position(|(row_idx, _)| *row_idx == Some(app.cursor))
        .unwrap_or(0);

    let current_offset = app.scroll_offset;

    if cursor_line < current_offset {
        // Cursor is above the visible area
        cursor_line
    } else if cursor_line >= current_offset + list_height {
        // Cursor is below the visible area
        cursor_line.saturating_sub(list_height - 1)
    } else {
        current_offset
    }
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::TOP);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    let lines_area = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(inner);

    // Line 1: keybinding hints
    let hints = if app.mode == Mode::Filter {
        Line::from(vec![
            Span::styled(" Enter", Style::default().fg(CYAN)),
            Span::styled(" apply  ", Style::default().fg(GRAY)),
            Span::styled("Esc", Style::default().fg(CYAN)),
            Span::styled(" clear  ", Style::default().fg(GRAY)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" j/k", Style::default().fg(CYAN)),
            Span::styled(" move  ", Style::default().fg(GRAY)),
            Span::styled("Space", Style::default().fg(CYAN)),
            Span::styled(" select  ", Style::default().fg(GRAY)),
            Span::styled("a", Style::default().fg(CYAN)),
            Span::styled(" merged  ", Style::default().fg(GRAY)),
            Span::styled("d", Style::default().fg(CYAN)),
            Span::styled(" delete  ", Style::default().fg(GRAY)),
            Span::styled("/", Style::default().fg(CYAN)),
            Span::styled(" filter  ", Style::default().fg(GRAY)),
            Span::styled("s", Style::default().fg(CYAN)),
            Span::styled(" sort  ", Style::default().fg(GRAY)),
            Span::styled("?", Style::default().fg(CYAN)),
            Span::styled(" help  ", Style::default().fg(GRAY)),
            Span::styled("q", Style::default().fg(CYAN)),
            Span::styled(" quit", Style::default().fg(GRAY)),
        ])
    };
    frame.render_widget(Paragraph::new(hints), lines_area[0]);

    // Line 2: selection info
    let count = app.selected_count();
    let selection_line = if count == 0 {
        Line::from(Span::styled(
            " No branches selected",
            Style::default().fg(GRAY),
        ))
    } else {
        let local = app.selected_local_count();
        let remote = app.selected_remote_count();
        Line::from(Span::styled(
            format!(
                " Selected: {} branches ({} local, {} remote)",
                count, local, remote
            ),
            Style::default().fg(WHITE),
        ))
    };
    frame.render_widget(Paragraph::new(selection_line), lines_area[1]);
}

// ── Confirm mode ────────────────────────────────────────────────────

fn draw_confirm(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let block = Block::default()
        .title(" Confirm Deletion ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(YELLOW));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Gather selected branches grouped by merge status
    let mut safe: Vec<&crate::branch::Branch> = Vec::new();
    let mut dangerous: Vec<&crate::branch::Branch> = Vec::new();
    for (i, &sel) in app.selected.iter().enumerate() {
        if sel {
            let branch = &app.all_branches[i];
            if branch.is_merged {
                safe.push(branch);
            } else {
                dangerous.push(branch);
            }
        }
    }

    // Safe section
    if !safe.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {} Safe to delete ({}):", CHECK, safe.len()),
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        )));
        for branch in &safe {
            let type_label = if branch.is_remote { "remote" } else { "local" };
            let sha_short = if branch.last_commit_sha.len() >= 7 {
                &branch.last_commit_sha[..7]
            } else {
                &branch.last_commit_sha
            };
            lines.push(Line::from(vec![
                Span::styled(format!("    {} ", CHECK), Style::default().fg(GREEN)),
                Span::styled(&branch.name, Style::default().fg(WHITE)),
                Span::styled(
                    format!("  {}  {}d  {}", type_label, branch.age_days, sha_short),
                    Style::default().fg(GRAY),
                ),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Dangerous section
    if !dangerous.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {} Unmerged branches ({}):", WARN, dangerous.len()),
            Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
        )));
        for branch in &dangerous {
            let type_label = if branch.is_remote { "remote" } else { "local" };
            let sha_short = if branch.last_commit_sha.len() >= 7 {
                &branch.last_commit_sha[..7]
            } else {
                &branch.last_commit_sha
            };
            lines.push(Line::from(vec![
                Span::styled(format!("    {} ", CROSS), Style::default().fg(RED)),
                Span::styled(&branch.name, Style::default().fg(YELLOW)),
                Span::styled(
                    format!("  {}  {}d  {}", type_label, branch.age_days, sha_short),
                    Style::default().fg(GRAY),
                ),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Summary
    let total = safe.len() + dangerous.len();
    let remote_count: usize = safe
        .iter()
        .chain(dangerous.iter())
        .filter(|b| b.is_remote)
        .count();
    lines.push(Line::from(Span::styled(
        format!(
            "  {} branches will be deleted{}",
            total,
            if remote_count > 0 {
                format!(" ({} remote)", remote_count)
            } else {
                String::new()
            }
        ),
        Style::default().fg(WHITE),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  A backup will be created automatically",
        Style::default().fg(GRAY),
    )));
    lines.push(Line::from(Span::styled(
        "  Restore with: deadbranch backup restore <branch-name>",
        Style::default().fg(GRAY),
    )));
    lines.push(Line::from(""));

    // Confirmation prompt
    if app.requires_strict_confirm() {
        lines.push(Line::from(vec![
            Span::styled(
                "  Type 'yes' to confirm, Esc to go back: ",
                Style::default().fg(YELLOW),
            ),
            Span::styled(&app.confirm_input, Style::default().fg(WHITE)),
            Span::styled(BLOCK, Style::default().fg(YELLOW)),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "  Press Enter or y to confirm, Esc to go back",
            Style::default().fg(GRAY),
        )));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ── Executing mode ──────────────────────────────────────────────────

fn draw_executing(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let block = Block::default()
        .title(" Deleting branches ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CYAN));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref path) = app.backup_path {
        lines.push(Line::from(Span::styled(
            format!("  Backup: {}", path),
            Style::default().fg(GRAY),
        )));
        lines.push(Line::from(""));
    }

    let total = app.selected_count();
    for result in &app.deletion_results {
        let (icon, color) = if result.success {
            (CHECK, GREEN)
        } else {
            (CROSS, RED)
        };
        let mut spans = vec![
            Span::styled(format!("  {} ", icon), Style::default().fg(color)),
            Span::styled(&result.branch.name, Style::default().fg(WHITE)),
        ];
        if let Some(ref err) = result.error {
            spans.push(Span::styled(format!("  {}", err), Style::default().fg(RED)));
        }
        lines.push(Line::from(spans));
    }

    let completed = app.deletion_results.len();
    if completed < total {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {} of {} completed", completed, total),
            Style::default().fg(GRAY),
        )));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ── Summary mode ────────────────────────────────────────────────────

fn draw_summary(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let block = Block::default()
        .title(" Done ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    let successes = app.deletion_results.iter().filter(|r| r.success).count();
    let failures = app.deletion_results.iter().filter(|r| !r.success).count();

    lines.push(Line::from(Span::styled(
        format!("  {} deleted {} {} failed", successes, DOT, failures),
        Style::default().fg(if failures > 0 { YELLOW } else { GREEN }),
    )));
    lines.push(Line::from(""));

    // List failures
    if failures > 0 {
        lines.push(Line::from(Span::styled(
            "  Failures:",
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        )));
        for result in app.deletion_results.iter().filter(|r| !r.success) {
            let err_msg = result.error.as_deref().unwrap_or("unknown error");
            lines.push(Line::from(vec![
                Span::styled(format!("    {} ", CROSS), Style::default().fg(RED)),
                Span::styled(&result.branch.name, Style::default().fg(WHITE)),
                Span::styled(format!(": {}", err_msg), Style::default().fg(RED)),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Restore info
    if successes > 0 {
        lines.push(Line::from(Span::styled(
            "  To restore:",
            Style::default().fg(GRAY),
        )));
        lines.push(Line::from(Span::styled(
            "    deadbranch backup restore <branch-name>",
            Style::default().fg(GRAY),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Press any key to exit",
        Style::default().fg(GRAY),
    )));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ── Help overlay ────────────────────────────────────────────────────

fn draw_help_overlay(frame: &mut Frame) {
    let area = frame.area();

    // Center the overlay
    let width = 50.min(area.width.saturating_sub(4));
    let height = 22.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let overlay_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, overlay_area);

    let block = Block::default()
        .title(" Help ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CYAN));
    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);

    let help_lines = vec![
        Line::from(Span::styled(
            " Navigation",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        help_line("j / Down", "Move down"),
        help_line("k / Up", "Move up"),
        Line::from(""),
        Line::from(Span::styled(
            " Selection",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        help_line("Space", "Toggle selection"),
        help_line("a", "Select all merged"),
        help_line("A", "Select all (force mode)"),
        help_line("n", "Deselect all"),
        Line::from(""),
        Line::from(Span::styled(
            " Actions",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        help_line("d", "Delete selected"),
        help_line("q / Esc", "Quit"),
        Line::from(""),
        Line::from(Span::styled(
            " Filtering",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        help_line("/", "Search filter"),
        help_line("s", "Cycle sort order"),
        help_line("m", "Toggle merged filter"),
        help_line("l", "Toggle local filter"),
        help_line("R", "Toggle remote filter"),
        Line::from(""),
        Line::from(Span::styled(
            " Use --force to select unmerged branches",
            Style::default().fg(GRAY),
        )),
    ];

    let paragraph = Paragraph::new(help_lines);
    frame.render_widget(paragraph, inner);
}

/// Build a single help line with key and description.
fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:>12}", key), Style::default().fg(YELLOW)),
        Span::styled(format!("  {}", desc), Style::default().fg(WHITE)),
    ])
}
