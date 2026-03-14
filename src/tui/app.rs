//! Core state management for the TUI

use std::sync::mpsc;

use ratatui::widgets::TableState;

use crate::branch::{Branch, BranchFilter};

/// Current mode of the TUI
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Browsing and selecting branches
    Browse,
    /// Visual range selection (like Vim's V)
    VisualSelect,
    /// Typing a search/filter query
    Filter,
    /// Confirming deletion
    Confirm,
    /// Snap animation playing + background deletions (non-interactive)
    Snapping,
    /// Showing results summary
    Summary,
}

/// Sort order for the branch list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Sort by branch name (alphabetical)
    Branch,
    /// Sort by age (oldest first)
    Age,
    /// Sort by merge status (merged first)
    Status,
    /// Sort by type (local first)
    Type,
    /// Sort by last commit date (oldest first)
    LastCommit,
    /// Sort by author name (alphabetical)
    Author,
}

impl SortOrder {
    /// Cycle to the next sort order (matches column order: Branch, Age, Status, Type, Last Commit)
    pub fn next(self) -> Self {
        match self {
            SortOrder::Branch => SortOrder::Age,
            SortOrder::Age => SortOrder::Status,
            SortOrder::Status => SortOrder::Type,
            SortOrder::Type => SortOrder::LastCommit,
            SortOrder::LastCommit => SortOrder::Author,
            SortOrder::Author => SortOrder::Branch,
        }
    }

    /// Human-readable label for the current sort order
    pub fn label(self) -> &'static str {
        match self {
            SortOrder::Branch => "Branch",
            SortOrder::Age => "Age",
            SortOrder::Status => "Status",
            SortOrder::Type => "Type",
            SortOrder::LastCommit => "Last Commit",
            SortOrder::Author => "Author",
        }
    }

    /// Default sort direction for this column (true = ascending)
    pub fn default_ascending(self) -> bool {
        match self {
            SortOrder::Branch => true,      // A → Z
            SortOrder::Age => false,        // oldest first
            SortOrder::Status => true,      // merged first
            SortOrder::Type => true,        // local first
            SortOrder::LastCommit => false, // oldest first
            SortOrder::Author => true,      // A → Z
        }
    }
}

/// Result of attempting to delete a single branch
#[derive(Debug, Clone)]
pub struct DeletionResult {
    /// The branch that was deleted (or attempted)
    pub branch: Branch,
    /// Whether deletion succeeded
    pub success: bool,
    /// Error message if deletion failed
    pub error: Option<String>,
}

/// Core application state for the TUI
pub struct App {
    /// Current UI mode
    pub mode: Mode,
    /// All branches (unfiltered)
    pub all_branches: Vec<Branch>,
    /// Indices into all_branches that are currently visible (after filtering)
    pub visible: Vec<usize>,
    /// Selection state for each branch in all_branches (parallel to all_branches)
    pub selected: Vec<bool>,
    /// Current cursor position within visible list
    pub cursor: usize,
    /// Whether --force was passed (allows deleting unmerged branches)
    pub force: bool,
    /// The default branch name (e.g. "main")
    pub default_branch: String,

    /// Current sort order
    pub sort_order: SortOrder,
    /// Whether sort is ascending (false = descending)
    pub sort_ascending: bool,
    /// Filter toggle: only show merged branches
    pub filter_merged_only: bool,
    /// Filter toggle: only show local branches
    pub filter_local_only: bool,
    /// Filter toggle: only show remote branches
    pub filter_remote_only: bool,

    /// Current search query text
    pub search_query: String,
    /// Text typed in the confirm dialog
    pub confirm_input: String,
    /// Results of branch deletions
    pub deletion_results: Vec<DeletionResult>,
    /// Path to the backup file created before deletion
    pub backup_path: Option<String>,
    /// Whether the help overlay is shown
    pub show_help: bool,
    /// Table state for the branch list (manages scroll offset)
    pub table_state: TableState,
    /// Branches remaining to be deleted (for incremental deletion)
    pub pending_deletions: Vec<Branch>,
    /// Whether 'g' was pressed and we're waiting for the second 'g'
    pub pending_g: bool,
    /// Cursor position when V was pressed (visual range anchor)
    pub visual_anchor: usize,
    /// Number of branch rows visible in the table viewport (set during render)
    pub table_visible_rows: usize,
    /// Active snap animation (present only during Snapping mode)
    pub snap_animation: Option<super::snap::SnapAnimation>,
    /// Mapping of branch_index → screen Y, populated during render
    pub branch_screen_positions: Vec<(usize, u16)>,
    /// Channel receiver for background deletion results
    pub deletion_receiver: Option<mpsc::Receiver<DeletionResult>>,
    /// Total number of branches being deleted (for progress tracking)
    pub deletion_total: usize,
}

impl App {
    /// Create a new App with the given branches and initial filter settings.
    ///
    /// Pre-selects all merged branches and seeds filter toggles from the
    /// initial BranchFilter.
    pub fn new(
        all_branches: Vec<Branch>,
        initial_filter: &BranchFilter,
        default_branch: &str,
        force: bool,
    ) -> Self {
        // Pre-select merged branches
        let selected: Vec<bool> = all_branches.iter().map(|b| b.is_merged).collect();

        let mut app = Self {
            mode: Mode::Browse,
            visible: Vec::new(),
            selected,
            cursor: 0,
            force,
            default_branch: default_branch.to_string(),
            sort_order: SortOrder::Age,
            sort_ascending: SortOrder::Age.default_ascending(),
            filter_merged_only: initial_filter.merged_only,
            filter_local_only: initial_filter.local_only,
            filter_remote_only: initial_filter.remote_only,
            search_query: String::new(),
            confirm_input: String::new(),
            deletion_results: Vec::new(),
            backup_path: None,
            show_help: false,
            table_state: TableState::default(),
            pending_deletions: Vec::new(),
            pending_g: false,
            visual_anchor: 0,
            table_visible_rows: 0,
            snap_animation: None,
            branch_screen_positions: Vec::new(),
            deletion_receiver: None,
            deletion_total: 0,
            all_branches,
        };

        app.update_visible();
        app
    }

    /// Re-filter all_branches into visible indices based on current filter
    /// toggles and search query, then sort. When a search query is active,
    /// fuzzy matching is used and results are sorted by relevance instead of
    /// the current column sort.
    pub fn update_visible(&mut self) {
        let filter = BranchFilter {
            min_age_days: 0,
            local_only: self.filter_local_only,
            remote_only: self.filter_remote_only,
            merged_only: self.filter_merged_only,
            protected_branches: Vec::new(),
            exclude_patterns: Vec::new(),
        };

        let query = &self.search_query;

        if query.is_empty() {
            // No search: filter only, then sort by column
            self.visible = self
                .all_branches
                .iter()
                .enumerate()
                .filter(|(_, b)| filter.matches(b))
                .map(|(i, _)| i)
                .collect();
            self.sort_visible();
        } else {
            // Fuzzy search: filter + score, sort by relevance (best first)
            let mut scored: Vec<(usize, isize)> = self
                .all_branches
                .iter()
                .enumerate()
                .filter(|(_, b)| filter.matches(b))
                .filter_map(|(i, b)| {
                    sublime_fuzzy::best_match(query, &b.name).map(|m| (i, m.score()))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.visible = scored.into_iter().map(|(i, _)| i).collect();
        }

        // Clamp cursor to valid range
        if self.visible.is_empty() {
            self.cursor = 0;
        } else if self.cursor >= self.visible.len() {
            self.cursor = self.visible.len() - 1;
        }
    }

    /// Sort the visible indices by the current sort order and direction.
    /// Always groups merged and unmerged branches together.
    pub fn sort_visible(&mut self) {
        let branches = &self.all_branches;
        let sort_order = self.sort_order;
        let ascending = self.sort_ascending;

        self.visible.sort_by(|&a, &b| {
            let ba = &branches[a];
            let bb = &branches[b];

            // Always group: merged first, then unmerged
            let merge_cmp = bb.is_merged.cmp(&ba.is_merged);
            if merge_cmp != std::cmp::Ordering::Equal {
                return merge_cmp;
            }

            // Within each group, sort ascending then flip if descending
            let cmp = match sort_order {
                SortOrder::Branch => ba.name.cmp(&bb.name),
                SortOrder::Age => ba.age_days.cmp(&bb.age_days),
                SortOrder::Status => ba.is_merged.cmp(&bb.is_merged),
                SortOrder::Type => ba.is_remote.cmp(&bb.is_remote),
                SortOrder::LastCommit => bb.last_commit_date.cmp(&ba.last_commit_date),
                SortOrder::Author => ba.last_commit_author.cmp(&bb.last_commit_author),
            };

            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });
    }

    // ── Navigation ──────────────────────────────────────────────────

    /// Move cursor up by one
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor down by one
    pub fn cursor_down(&mut self) {
        if !self.visible.is_empty() && self.cursor < self.visible.len() - 1 {
            self.cursor += 1;
        }
    }

    /// Jump cursor to the first branch
    pub fn jump_to_top(&mut self) {
        self.cursor = 0;
    }

    /// Jump cursor to the last branch
    pub fn jump_to_bottom(&mut self) {
        self.cursor = self.visible.len().saturating_sub(1);
    }

    /// Move cursor down by `n` rows, clamping to the end
    pub fn page_down(&mut self, n: usize) {
        let max = self.visible.len().saturating_sub(1);
        self.cursor = (self.cursor + n).min(max);
    }

    /// Move cursor up by `n` rows, clamping to zero
    pub fn page_up(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
    }

    /// Get the branch currently under the cursor, if any
    #[allow(dead_code)]
    pub fn focused_branch(&self) -> Option<&Branch> {
        self.focused_index().map(|i| &self.all_branches[i])
    }

    /// Get the all_branches index of the currently focused branch
    pub fn focused_index(&self) -> Option<usize> {
        self.visible.get(self.cursor).copied()
    }

    // ── Selection ───────────────────────────────────────────────────

    /// Toggle selection of the focused branch.
    /// Blocks selecting unmerged branches unless force is true.
    pub fn toggle_selection(&mut self) {
        if let Some(idx) = self.focused_index() {
            if self.selected[idx] {
                // Always allow deselection
                self.selected[idx] = false;
            } else {
                // Only allow selecting unmerged branches with force
                let branch = &self.all_branches[idx];
                if branch.is_merged || self.force {
                    self.selected[idx] = true;
                }
            }
        }
    }

    /// Toggle all merged branches in the visible list
    pub fn select_all_merged(&mut self) {
        let all_merged_selected = self
            .visible
            .iter()
            .filter(|&&idx| self.all_branches[idx].is_merged)
            .all(|&idx| self.selected[idx]);

        for &idx in &self.visible {
            if self.all_branches[idx].is_merged {
                self.selected[idx] = !all_merged_selected;
            }
        }
    }

    /// Toggle all visible branches (requires force for unmerged)
    pub fn select_all(&mut self) {
        let all_selectable_selected = self
            .visible
            .iter()
            .filter(|&&idx| self.all_branches[idx].is_merged || self.force)
            .all(|&idx| self.selected[idx]);

        for &idx in &self.visible {
            let branch = &self.all_branches[idx];
            if branch.is_merged || self.force {
                self.selected[idx] = !all_selectable_selected;
            }
        }
    }

    /// Invert selection of all visible selectable branches
    pub fn invert_selection(&mut self) {
        for &idx in &self.visible {
            let branch = &self.all_branches[idx];
            if branch.is_merged || self.force {
                self.selected[idx] = !self.selected[idx];
            }
        }
    }

    /// Deselect all branches
    pub fn deselect_all(&mut self) {
        for s in &mut self.selected {
            *s = false;
        }
    }

    // ── Query methods ───────────────────────────────────────────────

    /// Get all selected branches
    #[allow(dead_code)]
    pub fn selected_branches(&self) -> Vec<&Branch> {
        self.selected
            .iter()
            .enumerate()
            .filter(|(_, &sel)| sel)
            .map(|(i, _)| &self.all_branches[i])
            .collect()
    }

    /// Count of selected branches
    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|&&s| s).count()
    }

    /// Count of selected local branches
    pub fn selected_local_count(&self) -> usize {
        self.selected
            .iter()
            .enumerate()
            .filter(|(i, &sel)| sel && !self.all_branches[*i].is_remote)
            .count()
    }

    /// Count of selected remote branches
    pub fn selected_remote_count(&self) -> usize {
        self.selected
            .iter()
            .enumerate()
            .filter(|(i, &sel)| sel && self.all_branches[*i].is_remote)
            .count()
    }

    /// Whether the current selection requires strict confirmation
    /// (any unmerged or remote branches selected)
    pub fn requires_strict_confirm(&self) -> bool {
        self.selected.iter().enumerate().any(|(i, &sel)| {
            sel && (!self.all_branches[i].is_merged || self.all_branches[i].is_remote)
        })
    }

    /// Remove successfully deleted branches from the branch list and reset
    /// deletion state so the user can return to a clean browse view.
    pub fn apply_deletions_and_reset(&mut self) {
        // Collect names of successfully deleted branches
        let deleted: std::collections::HashSet<String> = self
            .deletion_results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.branch.name.clone())
            .collect();

        if !deleted.is_empty() {
            // Rebuild all_branches and selected, removing deleted ones
            let mut new_branches = Vec::new();
            let mut new_selected = Vec::new();
            for (i, branch) in self.all_branches.iter().enumerate() {
                if !deleted.contains(&branch.name) {
                    new_branches.push(branch.clone());
                    new_selected.push(self.selected[i]);
                }
            }
            self.all_branches = new_branches;
            self.selected = new_selected;
        }

        // Reset deletion state
        self.deletion_results.clear();
        self.pending_deletions.clear();
        self.backup_path = None;
        self.deletion_receiver = None;
        self.deletion_total = 0;
        self.confirm_input.clear();

        // Refresh visible list and fix cursor
        self.update_visible();
        if self.cursor >= self.visible.len() {
            self.cursor = self.visible.len().saturating_sub(1);
        }
    }

    // ── Filter methods ──────────────────────────────────────────────

    /// Cycle to the next sort order and re-sort
    pub fn cycle_sort(&mut self) {
        self.sort_order = self.sort_order.next();
        self.sort_ascending = self.sort_order.default_ascending();
        self.sort_visible();
    }

    /// Toggle the sort direction (ascending/descending) for the current column
    pub fn toggle_sort_direction(&mut self) {
        self.sort_ascending = !self.sort_ascending;
        self.sort_visible();
    }

    /// Toggle the merged-only filter
    pub fn toggle_merged_filter(&mut self) {
        self.filter_merged_only = !self.filter_merged_only;
        self.update_visible();
    }

    /// Toggle the local-only filter (clears remote filter)
    pub fn toggle_local_filter(&mut self) {
        self.filter_local_only = !self.filter_local_only;
        if self.filter_local_only {
            self.filter_remote_only = false;
        }
        self.update_visible();
    }

    /// Toggle the remote-only filter (clears local filter)
    pub fn toggle_remote_filter(&mut self) {
        self.filter_remote_only = !self.filter_remote_only;
        if self.filter_remote_only {
            self.filter_local_only = false;
        }
        self.update_visible();
    }

    // ── Help ────────────────────────────────────────────────────────

    /// Toggle the help overlay
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    // ── Visual Select ──────────────────────────────────────────────

    /// Enter visual range select mode, anchoring at the current cursor
    pub fn enter_visual_select(&mut self) {
        self.visual_anchor = self.cursor;
        self.mode = Mode::VisualSelect;
    }

    /// Get the ordered (min, max) range of the visual selection
    pub fn visual_range(&self) -> (usize, usize) {
        let a = self.visual_anchor;
        let b = self.cursor;
        (a.min(b), a.max(b))
    }

    /// Toggle selection for all branches in the visual range, then return to Browse
    pub fn apply_visual_selection(&mut self) {
        let (lo, hi) = self.visual_range();
        for row in lo..=hi {
            if let Some(&idx) = self.visible.get(row) {
                if self.selected[idx] {
                    self.selected[idx] = false;
                } else {
                    let branch = &self.all_branches[idx];
                    if branch.is_merged || self.force {
                        self.selected[idx] = true;
                    }
                }
            }
        }
        self.mode = Mode::Browse;
    }

    /// Cancel visual select and return to Browse without changing selection
    pub fn cancel_visual_select(&mut self) {
        self.mode = Mode::Browse;
    }

    // ── Fuzzy search ───────────────────────────────────────────────

    /// Get the matched character positions for a branch name against the
    /// current search query. Returns an empty vec when there is no query
    /// or no match.
    pub fn fuzzy_match_positions(&self, branch_name: &str) -> Vec<usize> {
        if self.search_query.is_empty() {
            return Vec::new();
        }
        sublime_fuzzy::best_match(&self.search_query, branch_name)
            .map(|m| m.matched_indices().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_branch(name: &str, age_days: i64, is_merged: bool, is_remote: bool) -> Branch {
        Branch {
            name: name.to_string(),
            age_days,
            is_merged,
            is_remote,
            last_commit_sha: "abc123".to_string(),
            last_commit_date: Utc::now(),
            last_commit_author: "testuser".to_string(),
        }
    }

    fn default_filter() -> BranchFilter {
        BranchFilter::default()
    }

    fn sample_branches() -> Vec<Branch> {
        vec![
            test_branch("feature/old-merged", 60, true, false),
            test_branch("feature/old-unmerged", 45, false, false),
            test_branch("origin/feature/remote-merged", 30, true, true),
            test_branch("feature/new-merged", 10, true, false),
            test_branch("feature/new-unmerged", 5, false, false),
        ]
    }

    #[test]
    fn test_new_pre_selects_merged() {
        let branches = sample_branches();
        let app = App::new(branches, &default_filter(), "main", false);

        assert!(app.selected[0]); // merged
        assert!(!app.selected[1]); // unmerged
        assert!(app.selected[2]); // merged remote
        assert!(app.selected[3]); // merged
        assert!(!app.selected[4]); // unmerged
    }

    #[test]
    fn test_new_seeds_filter_toggles() {
        let filter = BranchFilter {
            merged_only: true,
            local_only: true,
            ..Default::default()
        };
        let app = App::new(sample_branches(), &filter, "main", false);

        assert!(app.filter_merged_only);
        assert!(app.filter_local_only);
        assert!(!app.filter_remote_only);
    }

    #[test]
    fn test_visible_shows_all_by_default() {
        let branches = sample_branches();
        let count = branches.len();
        let app = App::new(branches, &default_filter(), "main", false);
        assert_eq!(app.visible.len(), count);
    }

    #[test]
    fn test_filter_merged_only() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.toggle_merged_filter();

        for &idx in &app.visible {
            assert!(app.all_branches[idx].is_merged);
        }
    }

    #[test]
    fn test_filter_local_clears_remote() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.filter_remote_only = true;
        app.toggle_local_filter();

        assert!(app.filter_local_only);
        assert!(!app.filter_remote_only);
    }

    #[test]
    fn test_filter_remote_clears_local() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.filter_local_only = true;
        app.toggle_remote_filter();

        assert!(app.filter_remote_only);
        assert!(!app.filter_local_only);
    }

    #[test]
    fn test_cursor_navigation() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        assert_eq!(app.cursor, 0);

        app.cursor_down();
        assert_eq!(app.cursor, 1);

        app.cursor_up();
        assert_eq!(app.cursor, 0);

        // Should not go below 0
        app.cursor_up();
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn test_cursor_does_not_exceed_visible() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        for _ in 0..100 {
            app.cursor_down();
        }
        assert_eq!(app.cursor, app.visible.len() - 1);
    }

    #[test]
    fn test_toggle_selection_merged() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        // First visible branch is unmerged (sorted: unmerged first)
        // Find a merged branch
        let merged_pos = app
            .visible
            .iter()
            .position(|&idx| app.all_branches[idx].is_merged)
            .unwrap();

        app.cursor = merged_pos;
        let idx = app.focused_index().unwrap();

        // Deselect (was pre-selected)
        app.toggle_selection();
        assert!(!app.selected[idx]);

        // Re-select
        app.toggle_selection();
        assert!(app.selected[idx]);
    }

    #[test]
    fn test_toggle_selection_blocks_unmerged_without_force() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        // Find an unmerged branch
        let unmerged_pos = app
            .visible
            .iter()
            .position(|&idx| !app.all_branches[idx].is_merged)
            .unwrap();

        app.cursor = unmerged_pos;
        let idx = app.focused_index().unwrap();
        assert!(!app.selected[idx]);

        app.toggle_selection();
        assert!(!app.selected[idx]); // Should still be unselected
    }

    #[test]
    fn test_toggle_selection_allows_unmerged_with_force() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", true);
        let unmerged_pos = app
            .visible
            .iter()
            .position(|&idx| !app.all_branches[idx].is_merged)
            .unwrap();

        app.cursor = unmerged_pos;
        let idx = app.focused_index().unwrap();

        app.toggle_selection();
        assert!(app.selected[idx]);
    }

    #[test]
    fn test_select_all_merged() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.deselect_all();
        app.select_all_merged();

        for (i, branch) in app.all_branches.iter().enumerate() {
            if branch.is_merged {
                assert!(app.selected[i]);
            } else {
                assert!(!app.selected[i]);
            }
        }
    }

    #[test]
    fn test_select_all_with_force() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", true);
        app.deselect_all();
        app.select_all();

        for &sel in &app.selected {
            assert!(sel);
        }
    }

    #[test]
    fn test_select_all_without_force_skips_unmerged() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.deselect_all();
        app.select_all();

        for (i, branch) in app.all_branches.iter().enumerate() {
            if branch.is_merged {
                assert!(app.selected[i]);
            } else {
                assert!(!app.selected[i]);
            }
        }
    }

    #[test]
    fn test_deselect_all() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.deselect_all();
        assert_eq!(app.selected_count(), 0);
    }

    #[test]
    fn test_select_all_merged_toggles_off_when_all_selected() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        // Merged branches are pre-selected by App::new
        assert_eq!(app.selected_count(), 3);

        // Toggle off: all merged already selected → deselect merged
        app.select_all_merged();
        assert_eq!(app.selected_count(), 0);

        // Toggle on: none selected → select merged
        app.select_all_merged();
        assert_eq!(app.selected_count(), 3);
    }

    #[test]
    fn test_select_all_toggles_off_when_all_selected() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", true);
        app.select_all();
        let total = app.all_branches.len();
        assert_eq!(app.selected_count(), total);

        // Toggle off
        app.select_all();
        assert_eq!(app.selected_count(), 0);

        // Toggle on
        app.select_all();
        assert_eq!(app.selected_count(), total);
    }

    #[test]
    fn test_invert_selection_all_merged_selected_to_none() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        // All merged are pre-selected
        assert_eq!(app.selected_count(), 3);
        app.invert_selection();
        // All merged should now be deselected, unmerged unchanged (still false)
        assert_eq!(app.selected_count(), 0);
    }

    #[test]
    fn test_invert_selection_none_to_all_merged() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.deselect_all();
        assert_eq!(app.selected_count(), 0);
        app.invert_selection();
        // Only merged branches should be selected
        assert_eq!(app.selected_count(), 3);
        for (i, branch) in app.all_branches.iter().enumerate() {
            if branch.is_merged {
                assert!(app.selected[i]);
            } else {
                assert!(!app.selected[i]);
            }
        }
    }

    #[test]
    fn test_invert_selection_skips_unmerged_without_force() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.deselect_all();
        app.invert_selection();
        // Unmerged branches should remain unselected
        for (i, branch) in app.all_branches.iter().enumerate() {
            if !branch.is_merged {
                assert!(!app.selected[i]);
            }
        }
    }

    #[test]
    fn test_invert_selection_with_force_includes_unmerged() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", true);
        app.deselect_all();
        app.invert_selection();
        // All branches (merged + unmerged) should be selected
        assert_eq!(app.selected_count(), app.all_branches.len());
    }

    #[test]
    fn test_selected_count() {
        let app = App::new(sample_branches(), &default_filter(), "main", false);
        // 3 merged branches are pre-selected
        assert_eq!(app.selected_count(), 3);
    }

    #[test]
    fn test_selected_local_and_remote_counts() {
        let app = App::new(sample_branches(), &default_filter(), "main", false);
        assert_eq!(app.selected_local_count(), 2); // 2 merged local
        assert_eq!(app.selected_remote_count(), 1); // 1 merged remote
    }

    #[test]
    fn test_requires_strict_confirm_with_remote() {
        let app = App::new(sample_branches(), &default_filter(), "main", false);
        // Has a remote branch selected
        assert!(app.requires_strict_confirm());
    }

    #[test]
    fn test_requires_strict_confirm_local_merged_only() {
        let branches = vec![
            test_branch("feature/a", 30, true, false),
            test_branch("feature/b", 20, true, false),
        ];
        let app = App::new(branches, &default_filter(), "main", false);
        // All selected are local and merged
        assert!(!app.requires_strict_confirm());
    }

    #[test]
    fn test_cycle_sort() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        assert_eq!(app.sort_order, SortOrder::Age);

        app.cycle_sort();
        assert_eq!(app.sort_order, SortOrder::Status);

        app.cycle_sort();
        assert_eq!(app.sort_order, SortOrder::Type);

        app.cycle_sort();
        assert_eq!(app.sort_order, SortOrder::LastCommit);

        app.cycle_sort();
        assert_eq!(app.sort_order, SortOrder::Author);

        app.cycle_sort();
        assert_eq!(app.sort_order, SortOrder::Branch);

        app.cycle_sort();
        assert_eq!(app.sort_order, SortOrder::Age);
    }

    #[test]
    fn test_search_query_filters() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.search_query = "remote".to_string();
        app.update_visible();

        assert_eq!(app.visible.len(), 1);
        assert!(app.all_branches[app.visible[0]].name.contains("remote"));
    }

    #[test]
    fn test_focused_branch() {
        let app = App::new(sample_branches(), &default_filter(), "main", false);
        assert!(app.focused_branch().is_some());
    }

    #[test]
    fn test_focused_branch_empty() {
        let app = App::new(Vec::new(), &default_filter(), "main", false);
        assert!(app.focused_branch().is_none());
    }

    #[test]
    fn test_sort_order_labels() {
        assert_eq!(SortOrder::Branch.label(), "Branch");
        assert_eq!(SortOrder::Age.label(), "Age");
        assert_eq!(SortOrder::Status.label(), "Status");
        assert_eq!(SortOrder::Type.label(), "Type");
        assert_eq!(SortOrder::LastCommit.label(), "Last Commit");
        assert_eq!(SortOrder::Author.label(), "Author");
    }

    #[test]
    fn test_requires_strict_confirm_with_unmerged() {
        let branches = vec![
            test_branch("feature/old", 45, true, false),
            test_branch("bugfix/stale", 60, false, false),
        ];
        let mut app = App::new(branches, &BranchFilter::default(), "main", true);
        app.deselect_all();
        app.selected[1] = true; // unmerged local
        assert!(app.requires_strict_confirm());
    }

    #[test]
    fn test_cycle_sort_changes_order() {
        // zebra is older (50d), alpha is newer (10d)
        // Age sort (oldest first) = zebra, alpha
        // Branch sort (alphabetical) = alpha, zebra
        let branches = vec![
            test_branch("zebra", 50, true, false),
            test_branch("alpha", 10, true, false),
        ];
        let app_age = App::new(branches.clone(), &BranchFilter::default(), "main", false);
        let order_age: Vec<_> = app_age
            .visible
            .iter()
            .map(|&i| app_age.all_branches[i].name.as_str())
            .collect();
        assert_eq!(order_age, vec!["zebra", "alpha"]);

        // Cycle: Age -> Status -> Type -> LastCommit -> Branch
        let mut app_branch = App::new(branches, &BranchFilter::default(), "main", false);
        app_branch.sort_order = SortOrder::Branch;
        app_branch.sort_ascending = SortOrder::Branch.default_ascending();
        app_branch.update_visible();
        let order_branch: Vec<_> = app_branch
            .visible
            .iter()
            .map(|&i| app_branch.all_branches[i].name.as_str())
            .collect();
        assert_eq!(order_branch, vec!["alpha", "zebra"]);

        assert_ne!(order_age, order_branch);
    }

    #[test]
    fn test_jump_to_top() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.cursor = 3;
        app.jump_to_top();
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn test_jump_to_bottom() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.jump_to_bottom();
        assert_eq!(app.cursor, app.visible.len() - 1);
    }

    #[test]
    fn test_jump_to_bottom_empty() {
        let mut app = App::new(Vec::new(), &default_filter(), "main", false);
        app.jump_to_bottom();
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn test_page_down() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.cursor = 0;
        app.page_down(2);
        assert_eq!(app.cursor, 2);
    }

    #[test]
    fn test_page_down_clamps() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.cursor = 0;
        app.page_down(100);
        assert_eq!(app.cursor, app.visible.len() - 1);
    }

    #[test]
    fn test_page_up() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.cursor = 3;
        app.page_up(2);
        assert_eq!(app.cursor, 1);
    }

    #[test]
    fn test_page_up_clamps() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.cursor = 1;
        app.page_up(100);
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn test_page_down_empty() {
        let mut app = App::new(Vec::new(), &default_filter(), "main", false);
        app.page_down(5);
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn test_page_up_empty() {
        let mut app = App::new(Vec::new(), &default_filter(), "main", false);
        app.page_up(5);
        assert_eq!(app.cursor, 0);
    }

    // ── Visual Select tests ────────────────────────────────────────

    #[test]
    fn test_visual_range_anchor_below_cursor() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.cursor = 3;
        app.enter_visual_select();
        app.cursor = 1; // move cursor above anchor
        let (lo, hi) = app.visual_range();
        assert_eq!(lo, 1);
        assert_eq!(hi, 3);
    }

    #[test]
    fn test_visual_range_anchor_above_cursor() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.cursor = 1;
        app.enter_visual_select();
        app.cursor = 3;
        let (lo, hi) = app.visual_range();
        assert_eq!(lo, 1);
        assert_eq!(hi, 3);
    }

    #[test]
    fn test_visual_range_single_row() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.cursor = 2;
        app.enter_visual_select();
        let (lo, hi) = app.visual_range();
        assert_eq!(lo, 2);
        assert_eq!(hi, 2);
    }

    #[test]
    fn test_apply_visual_selection_toggles_range() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.deselect_all();

        // Visual select rows 0..=2
        app.cursor = 0;
        app.enter_visual_select();
        app.cursor = 2;
        app.apply_visual_selection();

        // Should have toggled on the merged branches in the range
        assert_eq!(app.mode, Mode::Browse);
        let selected_count: usize = (0..=2)
            .filter_map(|row| app.visible.get(row).copied())
            .filter(|&idx| app.selected[idx])
            .count();
        assert!(selected_count > 0);
    }

    #[test]
    fn test_apply_visual_selection_respects_force() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.deselect_all();

        // Select all rows
        app.cursor = 0;
        app.enter_visual_select();
        app.cursor = app.visible.len() - 1;
        app.apply_visual_selection();

        // Without force, unmerged branches should remain unselected
        for &idx in &app.visible {
            if !app.all_branches[idx].is_merged {
                assert!(!app.selected[idx]);
            }
        }
    }

    #[test]
    fn test_apply_visual_selection_with_force() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", true);
        app.deselect_all();

        app.cursor = 0;
        app.enter_visual_select();
        app.cursor = app.visible.len() - 1;
        app.apply_visual_selection();

        // With force, all branches in range should be selected
        for &idx in &app.visible {
            assert!(app.selected[idx]);
        }
    }

    #[test]
    fn test_cancel_visual_select_preserves_selection() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        let before: Vec<bool> = app.selected.clone();

        app.enter_visual_select();
        app.cursor_down();
        app.cancel_visual_select();

        assert_eq!(app.mode, Mode::Browse);
        assert_eq!(app.selected, before);
    }

    // ── Fuzzy search tests ─────────────────────────────────────────

    #[test]
    fn test_fuzzy_search_filters_branches() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.search_query = "remote".to_string();
        app.update_visible();
        assert!(!app.visible.is_empty());
        // The remote branch should match
        assert!(app.all_branches[app.visible[0]].name.contains("remote"));
    }

    #[test]
    fn test_fuzzy_search_partial_match() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        // "featold" should fuzzy-match "feature/old-merged" and "feature/old-unmerged"
        app.search_query = "featold".to_string();
        app.update_visible();
        assert!(!app.visible.is_empty());
        for &idx in &app.visible {
            assert!(app.all_branches[idx].name.contains("old"));
        }
    }

    #[test]
    fn test_fuzzy_match_positions_empty_query() {
        let app = App::new(sample_branches(), &default_filter(), "main", false);
        assert!(app.fuzzy_match_positions("feature/foo").is_empty());
    }

    #[test]
    fn test_fuzzy_match_positions_with_query() {
        let mut app = App::new(sample_branches(), &default_filter(), "main", false);
        app.search_query = "foo".to_string();
        let positions = app.fuzzy_match_positions("feature/foo");
        assert!(!positions.is_empty());
    }

    #[test]
    fn test_fuzzy_search_sorts_by_relevance() {
        let branches = vec![
            test_branch("fix/unrelated-thing", 10, true, false),
            test_branch("feature/auth-refactor", 20, true, false),
            test_branch("auth-fix", 15, true, false),
        ];
        let mut app = App::new(branches, &default_filter(), "main", false);
        app.search_query = "auth".to_string();
        app.update_visible();

        // Both "auth" branches should match; "unrelated" should be excluded
        let names: Vec<&str> = app
            .visible
            .iter()
            .map(|&i| app.all_branches[i].name.as_str())
            .collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"auth-fix"));
        assert!(names.contains(&"feature/auth-refactor"));
        assert!(!names.contains(&"fix/unrelated-thing"));
    }
}
