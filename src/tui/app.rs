//! Core state management for the TUI

use std::cell::Cell;

use crate::branch::{Branch, BranchFilter};

/// Current mode of the TUI
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Browsing and selecting branches
    Browse,
    /// Typing a search/filter query
    Filter,
    /// Confirming deletion
    Confirm,
    /// Executing deletions
    Executing,
    /// Showing results summary
    Summary,
}

/// Sort order for the branch list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Sort by age (oldest first)
    Age,
    /// Sort by name (alphabetical)
    Name,
    /// Sort by merge status (merged first)
    Status,
}

impl SortOrder {
    /// Cycle to the next sort order
    pub fn next(self) -> Self {
        match self {
            SortOrder::Age => SortOrder::Name,
            SortOrder::Name => SortOrder::Status,
            SortOrder::Status => SortOrder::Age,
        }
    }

    /// Human-readable label for the current sort order
    pub fn label(self) -> &'static str {
        match self {
            SortOrder::Age => "Age",
            SortOrder::Name => "Name",
            SortOrder::Status => "Status",
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
    /// Whether execution has finished
    pub execution_done: bool,
    /// Whether the help overlay is shown
    pub show_help: bool,
    /// Scroll offset for the branch list
    pub scroll_offset: Cell<usize>,
    /// Branches remaining to be deleted (for incremental deletion)
    pub pending_deletions: Vec<Branch>,
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
            filter_merged_only: initial_filter.merged_only,
            filter_local_only: initial_filter.local_only,
            filter_remote_only: initial_filter.remote_only,
            search_query: String::new(),
            confirm_input: String::new(),
            deletion_results: Vec::new(),
            backup_path: None,
            execution_done: false,
            show_help: false,
            scroll_offset: Cell::new(0),
            pending_deletions: Vec::new(),
            all_branches,
        };

        app.update_visible();
        app
    }

    /// Re-filter all_branches into visible indices based on current filter
    /// toggles and search query, then sort.
    pub fn update_visible(&mut self) {
        let filter = BranchFilter {
            min_age_days: 0,
            local_only: self.filter_local_only,
            remote_only: self.filter_remote_only,
            merged_only: self.filter_merged_only,
            protected_branches: Vec::new(),
            exclude_patterns: Vec::new(),
        };

        let query = self.search_query.to_lowercase();

        self.visible = self
            .all_branches
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                if !filter.matches(b) {
                    return false;
                }
                if !query.is_empty() && !b.name.to_lowercase().contains(&query) {
                    return false;
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        self.sort_visible();

        // Clamp cursor to valid range
        if self.visible.is_empty() {
            self.cursor = 0;
        } else if self.cursor >= self.visible.len() {
            self.cursor = self.visible.len() - 1;
        }
    }

    /// Sort the visible indices by the current sort order.
    /// Always groups merged and unmerged branches together.
    pub fn sort_visible(&mut self) {
        let branches = &self.all_branches;
        let sort_order = self.sort_order;

        self.visible.sort_by(|&a, &b| {
            let ba = &branches[a];
            let bb = &branches[b];

            // Always group: merged first, then unmerged
            let merge_cmp = bb.is_merged.cmp(&ba.is_merged);
            if merge_cmp != std::cmp::Ordering::Equal {
                return merge_cmp;
            }

            // Within each group, sort by the chosen order
            match sort_order {
                SortOrder::Age => bb.age_days.cmp(&ba.age_days),
                SortOrder::Name => ba.name.cmp(&bb.name),
                SortOrder::Status => {
                    // Within merged/unmerged group, sort by age as secondary
                    bb.age_days.cmp(&ba.age_days)
                }
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

    /// Select all merged branches in the visible list
    pub fn select_all_merged(&mut self) {
        for &idx in &self.visible {
            if self.all_branches[idx].is_merged {
                self.selected[idx] = true;
            }
        }
    }

    /// Select all visible branches (requires force for unmerged)
    pub fn select_all(&mut self) {
        for &idx in &self.visible {
            let branch = &self.all_branches[idx];
            if branch.is_merged || self.force {
                self.selected[idx] = true;
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

    // ── Filter methods ──────────────────────────────────────────────

    /// Cycle to the next sort order and re-sort
    pub fn cycle_sort(&mut self) {
        self.sort_order = self.sort_order.next();
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
        assert_eq!(app.sort_order, SortOrder::Name);

        app.cycle_sort();
        assert_eq!(app.sort_order, SortOrder::Status);

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
        assert_eq!(SortOrder::Age.label(), "Age");
        assert_eq!(SortOrder::Name.label(), "Name");
        assert_eq!(SortOrder::Status.label(), "Status");
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
        // Name sort = alpha, zebra
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

        let mut app_name = App::new(branches, &BranchFilter::default(), "main", false);
        app_name.cycle_sort(); // Age -> Name
        let order_name: Vec<_> = app_name
            .visible
            .iter()
            .map(|&i| app_name.all_branches[i].name.as_str())
            .collect();
        assert_eq!(order_name, vec!["alpha", "zebra"]);

        assert_ne!(order_age, order_name);
    }
}
