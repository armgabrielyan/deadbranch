//! All screen rendering for the TUI

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, Wrap};
use ratatui::Frame;

use crate::branch::AgeSeverity;

use super::app::{App, Mode};

// ── Unicode characters ──────────────────────────────────────────────

const CURSOR: &str = ">";
const DOT: &str = "\u{00b7}"; // ·
const BLOCK: &str = "\u{2588}"; // █
const SHADE: &str = "\u{2591}"; // ░
const CHECK: &str = "\u{2713}"; // ✓
const CROSS: &str = "\u{2717}"; // ✗
const WARN: &str = "\u{26a0}"; // ⚠
const SEP: &str = "\u{2502}"; // │

// ── Colours ─────────────────────────────────────────────────────────

const GREEN: Color = Color::Green;
const YELLOW: Color = Color::Yellow;
const RED: Color = Color::Red;
const CYAN: Color = Color::Cyan;
const BLUE: Color = Color::Blue;
const GRAY: Color = Color::DarkGray;
const WHITE: Color = Color::White;

// ── Age color gradient ──────────────────────────────────────────────

fn age_color(age_days: i64) -> Color {
    match AgeSeverity::from_days(age_days) {
        AgeSeverity::Fresh => GREEN,
        AgeSeverity::Moderate => YELLOW,
        AgeSeverity::Stale => RED,
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Create a thin vertical separator cell.
fn sep_cell<'a>() -> Cell<'a> {
    Cell::from(SEP).style(Style::default().fg(GRAY))
}

/// Truncate a string to `max_len` characters, appending ".." if truncated.
/// Uses char-level counting instead of byte slicing for UTF-8 safety.
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.chars().count() > max_len {
        let truncated: String = name.chars().take(max_len - 2).collect();
        format!("{}..", truncated)
    } else {
        name.to_string()
    }
}

/// Build a styled Line for a branch name with fuzzy match positions highlighted.
///
/// Non-matching chars use `base_style`; matching chars get yellow bold.
/// The name is truncated to `max_len` before highlighting.
fn highlight_matches<'a>(
    name: &str,
    positions: &[usize],
    base_style: Style,
    max_len: usize,
) -> Line<'a> {
    let display = truncate_name(name, max_len);
    if positions.is_empty() {
        return Line::from(Span::styled(display, base_style));
    }

    let match_style = base_style.fg(YELLOW).add_modifier(Modifier::BOLD);
    let display_len = display.chars().count();
    let mut spans: Vec<Span<'a>> = Vec::new();
    let chars: Vec<char> = display.chars().collect();

    // Walk chars, grouping consecutive match/non-match runs into spans
    let mut i = 0;
    while i < display_len {
        let is_match = positions.contains(&i);
        let start = i;
        while i < display_len && positions.contains(&i) == is_match {
            i += 1;
        }
        let run: String = chars[start..i].iter().collect();
        let style = if is_match { match_style } else { base_style };
        spans.push(Span::styled(run, style));
    }

    Line::from(spans)
}

/// Compute a centered rectangle for dialog boxes.
/// Uses percentage of terminal width (min 50 chars) and fits content height.
fn centered_dialog(area: Rect, content_lines: u16) -> Rect {
    let width = (area.width * 6 / 10).max(50).min(area.width);
    let height = (content_lines + 4).min(area.height); // +4 for border + padding
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    Rect::new(area.x + x, area.y + y, width, height)
}

/// Render a styled footer hint line at the bottom of a dialog.
fn draw_footer_hints(frame: &mut Frame, area: Rect, hints: Vec<(&str, &str)>) {
    let spans: Vec<Span> = hints
        .into_iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(format!(" {}", key), Style::default().fg(CYAN)),
                Span::styled(format!(" {} ", desc), Style::default().fg(GRAY)),
            ]
        })
        .collect();
    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

// ── Main draw dispatch ──────────────────────────────────────────────

/// Top-level draw function: dispatches to mode-specific renderers.
pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.mode {
        Mode::Browse | Mode::Filter | Mode::VisualSelect => draw_browse(frame, app),
        Mode::Confirm => draw_confirm(frame, app),
        Mode::Executing => draw_executing(frame, app),
        Mode::Summary => draw_summary(frame, app),
    }

    if app.show_help {
        draw_help_overlay(frame);
    }
}

// ── Browse mode ─────────────────────────────────────────────────────

fn draw_browse(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(1), // spacer
        Constraint::Min(1),    // branch list
        Constraint::Length(3), // status bar (2 lines + border)
    ])
    .split(area);

    draw_header(frame, app, chunks[0]);
    // chunks[1] is the spacer — left empty
    draw_branch_list(frame, app, chunks[2]);
    draw_status_bar(frame, app, chunks[3]);
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
            "deadbranch clean",
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

    // Sort order + direction
    let arrow = if app.sort_ascending {
        "\u{2191}"
    } else {
        "\u{2193}"
    }; // ↑ or ↓
    parts.push(Span::styled(
        format!("  sort:{} {}", app.sort_order.label(), arrow),
        Style::default().fg(GRAY),
    ));

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

    let header = Paragraph::new(Line::from(parts)).alignment(Alignment::Center);
    frame.render_widget(header, area);
}

fn draw_branch_list(frame: &mut Frame, app: &mut App, area: Rect) {
    // Center table at ~70% of terminal width
    let table_width = (area.width * 7 / 10).max(60);
    let margin = (area.width.saturating_sub(table_width)) / 2;
    let area = Rect {
        x: area.x + margin,
        width: table_width,
        ..area
    };

    // Store how many data rows are visible (subtract header + horizontal rule)
    app.table_visible_rows = area.height.saturating_sub(2) as usize;

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

    // Build table rows and track cursor-to-table-row mapping
    let num_cols = 12; // selector + # + separators + data columns
    let mut rows: Vec<Row> = Vec::new();
    let mut cursor_table_row: usize = 0;
    let mut last_was_merged: Option<bool> = None;

    for (row_idx, &branch_idx) in app.visible.iter().enumerate() {
        let branch = &app.all_branches[branch_idx];

        // Section header when merge status changes (skip during search — relevance trumps grouping)
        if app.search_query.is_empty() && last_was_merged != Some(branch.is_merged) {
            let (label, color) = if branch.is_merged {
                ("MERGED (safe to delete)", GREEN)
            } else if app.force {
                ("UNMERGED (review carefully)", YELLOW)
            } else {
                ("UNMERGED (use --force to unlock)", YELLOW)
            };
            // Top margin via bottom_margin on previous row, or empty spacer row
            if !rows.is_empty() {
                // Add a blank spacer row before section header
                rows.push(Row::new(vec![Cell::from(""); num_cols]));
            }
            rows.push(
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(""),
                    sep_cell(),
                    Cell::from(Span::styled(
                        label,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ])
                .bottom_margin(0),
            );
            last_was_merged = Some(branch.is_merged);
        }

        // Track which table row the cursor points to
        if row_idx == app.cursor {
            cursor_table_row = rows.len();
        }

        // Detect if this row is inside a visual selection range
        let in_visual_range = app.mode == Mode::VisualSelect && {
            let (lo, hi) = app.visual_range();
            row_idx >= lo && row_idx <= hi
        };

        // Build the selector cell (cursor + checkbox)
        let is_focused = row_idx == app.cursor;
        let is_locked = !branch.is_merged && !app.force;
        let is_selected = app.selected[branch_idx];

        let selector = if is_locked {
            if is_focused {
                Line::from(vec![
                    Span::styled(format!("{} ", CURSOR), Style::default().fg(WHITE)),
                    Span::styled(format!("{}{}", SHADE, SHADE), Style::default().fg(GRAY)),
                ])
            } else {
                Line::from(Span::styled(
                    format!("  {}{}", SHADE, SHADE),
                    Style::default().fg(GRAY),
                ))
            }
        } else if is_focused {
            let cb = if is_selected { "[x]" } else { "[ ]" };
            let cb_color = if is_selected { GREEN } else { GRAY };
            Line::from(vec![
                Span::styled(format!("{} ", CURSOR), Style::default().fg(WHITE)),
                Span::styled(cb, Style::default().fg(cb_color)),
            ])
        } else {
            let cb = if is_selected { "[x]" } else { "[ ]" };
            let cb_color = if is_selected { GREEN } else { GRAY };
            Line::from(Span::styled(
                format!("  {}", cb),
                Style::default().fg(cb_color),
            ))
        };

        // Neovim-style line number: absolute on cursor, relative distance elsewhere
        let line_num = if is_focused {
            format!("{:>3}", row_idx + 1)
        } else {
            let distance = (row_idx as isize - app.cursor as isize).unsigned_abs();
            format!("{:>3}", distance)
        };
        let line_num_style = if is_focused {
            Style::default().fg(YELLOW)
        } else {
            Style::default().fg(GRAY)
        };

        let name_style = if is_locked {
            Style::default().fg(GRAY)
        } else if is_focused {
            Style::default().fg(WHITE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(WHITE)
        };

        let (status_text, status_color) = if branch.is_merged {
            ("merged", GREEN)
        } else {
            ("unmerged", YELLOW)
        };

        let (type_text, type_color) = if branch.is_remote {
            ("remote", BLUE)
        } else {
            ("local", CYAN)
        };

        let date_str = branch.last_commit_date.format("%Y-%m-%d").to_string();

        let match_positions = app.fuzzy_match_positions(&branch.name);
        let mut row = Row::new(vec![
            Cell::from(selector),
            Cell::from(line_num).style(line_num_style),
            sep_cell(),
            Cell::from(highlight_matches(
                &branch.name,
                &match_positions,
                name_style,
                60,
            )),
            sep_cell(),
            Cell::from(format!("{}d", branch.age_days))
                .style(Style::default().fg(age_color(branch.age_days))),
            sep_cell(),
            Cell::from(status_text).style(Style::default().fg(status_color)),
            sep_cell(),
            Cell::from(type_text).style(Style::default().fg(type_color)),
            Cell::from(date_str).style(Style::default().fg(GRAY)),
            Cell::from(branch.last_commit_author.as_str()).style(Style::default().fg(WHITE)),
        ]);

        if in_visual_range {
            row = row.style(Style::default().bg(Color::Indexed(236)));
        }

        rows.push(row);
    }

    // Column widths: data columns with thin separator columns between them
    let widths = [
        Constraint::Length(5),  // selector: "▶ [x]"
        Constraint::Length(3),  // line number: "  1" / " 42"
        Constraint::Length(1),  // separator │
        Constraint::Fill(3),    // branch name: 75% of remaining space
        Constraint::Length(1),  // separator │
        Constraint::Length(6),  // age: "1234d"
        Constraint::Length(1),  // separator │
        Constraint::Length(8),  // status: "unmerged"
        Constraint::Length(1),  // separator │
        Constraint::Length(6),  // type: "remote"
        Constraint::Length(11), // date: "Last Commit" / "2026-01-27"
        Constraint::Fill(1),    // author: 25% of remaining space
    ];

    // Header row
    let header_style = Style::default().fg(GRAY).add_modifier(Modifier::BOLD);
    let sep_header = Cell::from(SEP).style(Style::default().fg(GRAY));
    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("#").style(header_style),
        sep_header.clone(),
        Cell::from("Branch").style(header_style),
        sep_header.clone(),
        Cell::from("Age").style(header_style),
        sep_header.clone(),
        Cell::from("Status").style(header_style),
        sep_header,
        Cell::from("Type").style(header_style),
        Cell::from("Last Commit").style(header_style),
        Cell::from("Author").style(header_style),
    ]);

    // Horizontal rule row between header and data
    let hr_style = Style::default().fg(GRAY);
    let hr = |width: u16| Cell::from("\u{2500}".repeat(width as usize)).style(hr_style);
    let hr_row = Row::new(vec![
        hr(5),                                  // selector
        hr(3),                                  // line number
        Cell::from("\u{253c}").style(hr_style), // ┼
        hr(200),                                // branch (clipped by Fill constraint)
        Cell::from("\u{253c}").style(hr_style), // ┼
        hr(6),                                  // age
        Cell::from("\u{253c}").style(hr_style), // ┼
        hr(8),                                  // status
        Cell::from("\u{253c}").style(hr_style), // ┼
        hr(6),                                  // type
        hr(11),                                 // date
        hr(200),                                // author (clipped by Fill constraint)
    ]);

    // Insert the horizontal rule as the first data row, shifting cursor mapping
    rows.insert(0, hr_row);
    cursor_table_row += 1;

    let table = Table::new(rows, widths).header(header).column_spacing(1);

    // Update table state for auto-scrolling.
    // When cursor is near the top of the viewport, back up the offset
    // to keep section headers visible above the first branch in each group.
    app.table_state.select(Some(cursor_table_row));
    let offset = app.table_state.offset();
    if cursor_table_row > 0 && cursor_table_row <= offset + 1 {
        *app.table_state.offset_mut() = cursor_table_row.saturating_sub(2);
    }
    frame.render_stateful_widget(table, area, &mut app.table_state);
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
    let hints = if app.mode == Mode::VisualSelect {
        Line::from(vec![
            Span::styled(
                " VISUAL",
                Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default().fg(GRAY)),
            Span::styled("j/k", Style::default().fg(CYAN)),
            Span::styled(" extend  ", Style::default().fg(GRAY)),
            Span::styled("Space", Style::default().fg(CYAN)),
            Span::styled(" toggle range  ", Style::default().fg(GRAY)),
            Span::styled("Esc", Style::default().fg(CYAN)),
            Span::styled(" cancel", Style::default().fg(GRAY)),
        ])
    } else if app.mode == Mode::Filter {
        Line::from(vec![
            Span::styled(" Enter", Style::default().fg(CYAN)),
            Span::styled(" apply  ", Style::default().fg(GRAY)),
            Span::styled("Esc", Style::default().fg(CYAN)),
            Span::styled(" clear  ", Style::default().fg(GRAY)),
        ])
    } else {
        {
            let mut hints = vec![
                Span::styled(" j/k", Style::default().fg(CYAN)),
                Span::styled(" move  ", Style::default().fg(GRAY)),
                Span::styled("Space", Style::default().fg(CYAN)),
                Span::styled(" select  ", Style::default().fg(GRAY)),
                Span::styled("a", Style::default().fg(CYAN)),
                Span::styled(" merged  ", Style::default().fg(GRAY)),
            ];
            if app.force {
                hints.push(Span::styled("A", Style::default().fg(CYAN)));
                hints.push(Span::styled(" all  ", Style::default().fg(GRAY)));
            } else {
                hints.push(Span::styled("A", Style::default().fg(GRAY)));
                hints.push(Span::styled(
                    " (needs --force)  ",
                    Style::default().fg(GRAY),
                ));
            }
            hints.extend([
                Span::styled("d", Style::default().fg(CYAN)),
                Span::styled(" delete  ", Style::default().fg(GRAY)),
                Span::styled("/", Style::default().fg(CYAN)),
                Span::styled(" filter  ", Style::default().fg(GRAY)),
                Span::styled("s/S", Style::default().fg(CYAN)),
                Span::styled(" sort  ", Style::default().fg(GRAY)),
                Span::styled("?", Style::default().fg(CYAN)),
                Span::styled(" help  ", Style::default().fg(GRAY)),
                Span::styled("q", Style::default().fg(CYAN)),
                Span::styled(" quit", Style::default().fg(GRAY)),
            ]);
            Line::from(hints)
        }
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

    // Count selected branches by category
    let mut merged = 0usize;
    let mut unmerged = 0usize;
    let mut local = 0usize;
    let mut remote = 0usize;
    for (i, &sel) in app.selected.iter().enumerate() {
        if sel {
            let b = &app.all_branches[i];
            if b.is_merged {
                merged += 1;
            } else {
                unmerged += 1;
            }
            if b.is_remote {
                remote += 1;
            } else {
                local += 1;
            }
        }
    }
    let total = merged + unmerged;
    let has_risk = remote > 0 || unmerged > 0;

    // Fixed compact dialog height
    let content_height = if has_risk { 12 } else { 10 };

    let dialog = centered_dialog(area, content_height);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .title(" Confirm Deletion ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(YELLOW));
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let content_area = chunks[0];
    let footer_area = chunks[1];

    let mut lines: Vec<Line> = Vec::new();

    // Hero count
    lines.push(Line::from(""));
    lines.push(
        Line::from(Span::styled(
            format!(
                "{} branch{} selected",
                total,
                if total == 1 { "" } else { "es" }
            ),
            Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
    );
    lines.push(Line::from(""));

    // Breakdown line: merged/unmerged · local/remote
    let mut parts: Vec<Span> = Vec::new();
    if merged > 0 {
        parts.push(Span::styled(
            format!("{} merged", merged),
            Style::default().fg(GREEN),
        ));
    }
    if unmerged > 0 {
        if !parts.is_empty() {
            parts.push(Span::styled(
                format!("  {}  ", DOT),
                Style::default().fg(GRAY),
            ));
        }
        parts.push(Span::styled(
            format!("{} unmerged", unmerged),
            Style::default().fg(YELLOW),
        ));
    }
    parts.push(Span::styled(
        format!("  {}  ", DOT),
        Style::default().fg(GRAY),
    ));
    if local > 0 {
        parts.push(Span::styled(
            format!("{} local", local),
            Style::default().fg(CYAN),
        ));
    }
    if remote > 0 {
        if local > 0 {
            parts.push(Span::styled(
                format!("  {}  ", DOT),
                Style::default().fg(GRAY),
            ));
        }
        parts.push(Span::styled(
            format!("{} remote", remote),
            Style::default().fg(BLUE),
        ));
    }
    lines.push(Line::from(parts).alignment(Alignment::Center));
    lines.push(Line::from(""));

    // Separator with inline backup note
    let label = format!(" {} backup auto-created ", CHECK);
    let label_len = label.chars().count();
    let avail = inner.width.saturating_sub(4) as usize;
    let side = avail.saturating_sub(label_len) / 2;
    let hr = "\u{2500}";
    lines.push(
        Line::from(vec![
            Span::styled(hr.repeat(side), Style::default().fg(GRAY)),
            Span::styled(label, Style::default().fg(GRAY)),
            Span::styled(hr.repeat(side), Style::default().fg(GRAY)),
        ])
        .alignment(Alignment::Center),
    );
    lines.push(Line::from(""));

    // Warning callout (only when risky)
    if has_risk {
        let mut warn_parts = vec![Span::styled(
            format!("{}  ", WARN),
            Style::default().fg(YELLOW),
        )];
        let mut reasons = Vec::new();
        if unmerged > 0 {
            reasons.push("unmerged");
        }
        if remote > 0 {
            reasons.push("remote");
        }
        warn_parts.push(Span::styled(
            format!("Includes {} branches", reasons.join(" & ")),
            Style::default().fg(YELLOW),
        ));
        lines.push(Line::from(warn_parts).alignment(Alignment::Center));
        lines.push(Line::from(""));
    }

    // Input field (only for strict confirm — simple confirm uses footer only)
    if app.requires_strict_confirm() {
        lines.push(
            Line::from(vec![
                Span::styled("> ", Style::default().fg(GRAY)),
                Span::styled(&app.confirm_input, Style::default().fg(WHITE)),
                Span::styled(BLOCK, Style::default().fg(WHITE)),
            ])
            .alignment(Alignment::Center),
        );
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, content_area);

    let hints = if app.requires_strict_confirm() {
        vec![("Type 'yes' + Enter", "confirm"), ("Esc", "back")]
    } else {
        vec![("Enter/y", "confirm"), ("Esc", "back")]
    };
    draw_footer_hints(frame, footer_area, hints);
}

// ── Executing mode ──────────────────────────────────────────────────

fn draw_executing(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let total = app.selected_count();
    let completed = app.deletion_results.len();

    // Use most of the terminal height (capped by centered_dialog)
    let content_height = area.height.saturating_sub(6);

    let dialog = centered_dialog(area, content_height);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .title(" Deleting Branches ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CYAN));
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    // Split: content area on top, gauge at bottom
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);
    let content_area = chunks[0];
    let gauge_area = chunks[1];

    let mut lines: Vec<Line> = Vec::new();

    let backup_lines: u16 = if app.backup_path.is_some() { 2 } else { 0 };
    if let Some(ref path) = app.backup_path {
        lines.push(Line::from(Span::styled(
            format!("  Backup: {}", path),
            Style::default().fg(GRAY),
        )));
        lines.push(Line::from(""));
    }

    // Show only the tail of results that fit the content area,
    // so the list scrolls like a log during large deletions.
    let max_result_lines = content_area.height.saturating_sub(backup_lines + 1) as usize; // +1 for pending line
    let skip = app.deletion_results.len().saturating_sub(max_result_lines);
    if skip > 0 {
        lines.push(Line::from(Span::styled(
            format!("  ... {} more above", skip),
            Style::default().fg(GRAY),
        )));
    }
    for result in app.deletion_results.iter().skip(skip) {
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

    // Show pending branches with dimmed style
    let pending_count = total.saturating_sub(completed);
    if pending_count > 0 {
        lines.push(Line::from(Span::styled(
            format!("  {} deleting...", DOT),
            Style::default().fg(GRAY),
        )));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, content_area);

    // Progress gauge
    let ratio = if total > 0 {
        completed as f64 / total as f64
    } else {
        0.0
    };
    let gauge_label = Span::styled(
        format!(" {}/{} ", completed, total),
        Style::default().fg(WHITE),
    );
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(CYAN))
        .ratio(ratio)
        .label(gauge_label)
        .use_unicode(true);
    frame.render_widget(gauge, gauge_area);
}

// ── Summary mode ────────────────────────────────────────────────────

fn draw_summary(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let successes = app.deletion_results.iter().filter(|r| r.success).count();
    let failures = app.deletion_results.iter().filter(|r| !r.success).count();

    // Compute content height
    let failure_lines: u16 = if failures > 0 { failures as u16 + 2 } else { 0 };
    let restore_lines: u16 = if successes > 0 { 2 } else { 0 };
    let content_height = 3 + failure_lines + restore_lines + 1;

    let dialog = centered_dialog(area, content_height);
    frame.render_widget(Clear, dialog);

    let border_color = if failures > 0 { YELLOW } else { GREEN };
    let block = Block::default()
        .title(" Done ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    // Split: content + footer
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let content_area = chunks[0];
    let footer_area = chunks[1];

    let mut lines: Vec<Line> = Vec::new();

    // Prominent centered result line
    lines.push(Line::from("")); // top padding
    if failures == 0 {
        lines.push(
            Line::from(vec![
                Span::styled(
                    format!("{} ", CHECK),
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} deleted", successes),
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
            ])
            .alignment(Alignment::Center),
        );
    } else {
        lines.push(
            Line::from(vec![
                Span::styled(
                    format!("{} ", CHECK),
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} deleted", successes),
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {}  ", DOT), Style::default().fg(GRAY)),
                Span::styled(
                    format!("{} ", CROSS),
                    Style::default().fg(RED).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} failed", failures),
                    Style::default().fg(RED).add_modifier(Modifier::BOLD),
                ),
            ])
            .alignment(Alignment::Center),
        );
    }
    lines.push(Line::from("")); // spacing after hero

    // List failures
    if failures > 0 {
        lines.push(Line::from(Span::styled(
            "  Failed:",
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

    // Restore hint — inline with separator, same pattern as confirm dialog
    if successes > 0 {
        let label = " deadbranch backup restore <name> ";
        let label_len = label.chars().count();
        let avail = inner.width.saturating_sub(4) as usize;
        let side = avail.saturating_sub(label_len) / 2;
        let hr = "\u{2500}";
        lines.push(
            Line::from(vec![
                Span::styled(hr.repeat(side), Style::default().fg(GRAY)),
                Span::styled(label, Style::default().fg(GRAY)),
                Span::styled(hr.repeat(side), Style::default().fg(GRAY)),
            ])
            .alignment(Alignment::Center),
        );
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, content_area);

    // Footer
    draw_footer_hints(
        frame,
        footer_area,
        vec![("Esc", "back"), ("any key", "exit")],
    );
}

// ── Help overlay ────────────────────────────────────────────────────

fn draw_help_overlay(frame: &mut Frame) {
    let area = frame.area();

    // Center the overlay
    let width = 50.min(area.width.saturating_sub(4));
    let height = 30.min(area.height.saturating_sub(4));
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
        help_line("gg", "Jump to top"),
        help_line("G", "Jump to bottom"),
        help_line("Ctrl+d/u", "Half-page down/up"),
        help_line("Ctrl+f/b", "Full-page down/up"),
        help_line("Scroll", "Mouse scroll"),
        Line::from(""),
        Line::from(Span::styled(
            " Selection",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        help_line("Space", "Toggle selection"),
        help_line("V", "Visual range select"),
        help_line("a", "Toggle all merged"),
        help_line("A", "Toggle all (force mode)"),
        help_line("n", "Deselect all"),
        help_line("i", "Invert selection"),
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
        help_line("s", "Cycle sort column"),
        help_line("S", "Reverse sort direction"),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Style {
        Style::default().fg(WHITE)
    }

    #[test]
    fn highlight_no_positions_returns_single_span() {
        let line = highlight_matches("feature/foo", &[], base(), 60);
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "feature/foo");
    }

    #[test]
    fn highlight_consecutive_positions() {
        // Positions 8,9,10 = "foo" in "feature/foo"
        let line = highlight_matches("feature/foo", &[8, 9, 10], base(), 60);
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content, "feature/");
        assert_eq!(line.spans[1].content, "foo");
        assert!(line.spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn highlight_scattered_positions() {
        // "f_a_u_e" matching positions 0, 2, 4, 6 in "feature/"
        let line = highlight_matches("feature/", &[0, 2, 4, 6], base(), 60);
        // Should alternate: match(f), non(e), match(a), non(t), match(u), non(r), match(e), non(/)
        assert!(line.spans.len() > 1);
        // First span is "f" (matched)
        assert_eq!(line.spans[0].content, "f");
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn highlight_match_at_start() {
        let line = highlight_matches("foo-bar", &[0, 1, 2], base(), 60);
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content, "foo");
        assert_eq!(line.spans[1].content, "-bar");
    }

    #[test]
    fn highlight_match_at_end() {
        let line = highlight_matches("bar-foo", &[4, 5, 6], base(), 60);
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content, "bar-");
        assert_eq!(line.spans[1].content, "foo");
    }

    #[test]
    fn highlight_truncation() {
        let line = highlight_matches("very-long-branch-name", &[0, 1], base(), 10);
        // Should be truncated to 10 chars: "very-lon.."
        assert!(line.spans.iter().map(|s| s.content.len()).sum::<usize>() <= 10);
    }

    #[test]
    fn age_color_green_for_fresh_branches() {
        assert_eq!(age_color(0), GREEN);
        assert_eq!(age_color(30), GREEN);
    }

    #[test]
    fn age_color_yellow_for_moderate_branches() {
        assert_eq!(age_color(31), YELLOW);
        assert_eq!(age_color(90), YELLOW);
    }

    #[test]
    fn age_color_red_for_stale_branches() {
        assert_eq!(age_color(91), RED);
        assert_eq!(age_color(365), RED);
    }
}
