//! Branch struct and filtering logic

use chrono::{DateTime, Utc};

/// Age severity for color coding across UIs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeSeverity {
    /// 0-30 days → green
    Fresh,
    /// 31-90 days → yellow
    Moderate,
    /// 91+ days → red
    Stale,
}

impl AgeSeverity {
    /// Determine severity from age in days
    pub fn from_days(age_days: i64) -> Self {
        match age_days {
            0..=30 => AgeSeverity::Fresh,
            31..=90 => AgeSeverity::Moderate,
            _ => AgeSeverity::Stale,
        }
    }
}

/// Represents a git branch with metadata
#[derive(Debug, Clone)]
pub struct Branch {
    /// Branch name (e.g., "feature/old-api" or "origin/feature/old-api")
    pub name: String,
    /// Days since last commit
    pub age_days: i64,
    /// Whether the branch is merged into the default branch
    pub is_merged: bool,
    /// Whether this is a remote branch
    pub is_remote: bool,
    /// SHA of the last commit
    pub last_commit_sha: String,
    /// Date of the last commit
    pub last_commit_date: DateTime<Utc>,
    /// Author of the last commit
    pub last_commit_author: String,
}

impl Branch {
    /// Check if this branch matches any protected pattern
    pub fn is_protected(&self, protected_branches: &[String]) -> bool {
        let name = self.short_name();
        protected_branches.iter().any(|p| p == name)
    }

    /// Check if this branch matches any exclude pattern (glob-style)
    /// Supports: "wip/*", "*/draft", "feature/*/temp", etc.
    pub fn matches_exclude_pattern(&self, patterns: &[String]) -> bool {
        let name = self.short_name();
        patterns
            .iter()
            .any(|pattern| Self::glob_match(pattern, name))
    }

    /// Simple glob matching: supports * as wildcard
    fn glob_match(pattern: &str, text: &str) -> bool {
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() == 1 {
            // No wildcard, exact match
            return pattern == text;
        }

        let mut remaining = text;

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 {
                // First part must be at the start
                if !remaining.starts_with(part) {
                    return false;
                }
                remaining = &remaining[part.len()..];
            } else if i == parts.len() - 1 {
                // Last part must be at the end
                if !remaining.ends_with(part) {
                    return false;
                }
                remaining = "";
            } else {
                // Middle parts can be anywhere
                if let Some(pos) = remaining.find(part) {
                    remaining = &remaining[pos + part.len()..];
                } else {
                    return false;
                }
            }
        }

        true
    }

    /// Get the short name (without origin/ prefix for remote branches)
    pub fn short_name(&self) -> &str {
        if self.is_remote {
            self.name.strip_prefix("origin/").unwrap_or(&self.name)
        } else {
            &self.name
        }
    }

    /// Get the age severity category for color coding
    pub fn age_severity(&self) -> AgeSeverity {
        AgeSeverity::from_days(self.age_days)
    }

    /// Format age in a human-readable way
    pub fn format_age(&self) -> String {
        if self.age_days == 1 {
            "1 day".to_string()
        } else {
            format!("{} days", self.age_days)
        }
    }
}

/// Filter options for listing branches
#[derive(Debug, Clone, Default)]
pub struct BranchFilter {
    /// Minimum age in days
    pub min_age_days: u32,
    /// Only show local branches
    pub local_only: bool,
    /// Only show remote branches
    pub remote_only: bool,
    /// Only show merged branches
    pub merged_only: bool,
    /// Protected branch names to exclude
    pub protected_branches: Vec<String>,
    /// Glob patterns to exclude (e.g., "wip/*", "*/draft")
    pub exclude_patterns: Vec<String>,
}

impl BranchFilter {
    /// Check if a branch passes this filter
    pub fn matches(&self, branch: &Branch) -> bool {
        // Check age
        if branch.age_days < self.min_age_days as i64 {
            return false;
        }

        // Check local/remote filter
        if self.local_only && branch.is_remote {
            return false;
        }
        if self.remote_only && !branch.is_remote {
            return false;
        }

        // Check merged filter
        if self.merged_only && !branch.is_merged {
            return false;
        }

        // Exclude protected branches
        if branch.is_protected(&self.protected_branches) {
            return false;
        }

        // Exclude branches matching exclude patterns
        if branch.matches_exclude_pattern(&self.exclude_patterns) {
            return false;
        }

        true
    }
}

/// Sort branches: merged first, then by age (oldest first)
pub fn sort_branches(branches: &mut [Branch]) {
    branches.sort_by(|a, b| {
        // First: merged before unmerged (actionable items first)
        match (a.is_merged, b.is_merged) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            // Then: oldest first (higher age_days first)
            _ => b.age_days.cmp(&a.age_days),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Helper to create a test branch
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

    #[test]
    fn test_branch_short_name() {
        let local = test_branch("feature/test", 10, false, false);
        assert_eq!(local.short_name(), "feature/test");

        let remote = test_branch("origin/feature/test", 10, false, true);
        assert_eq!(remote.short_name(), "feature/test");
    }

    #[test]
    fn test_branch_format_age() {
        let one_day = test_branch("test", 1, false, false);
        assert_eq!(one_day.format_age(), "1 day");

        let multiple_days = test_branch("test", 42, false, false);
        assert_eq!(multiple_days.format_age(), "42 days");
    }

    #[test]
    fn test_branch_is_protected() {
        let branch = test_branch("feature/test", 10, false, false);
        let protected = vec!["main".to_string(), "develop".to_string()];
        assert!(!branch.is_protected(&protected));

        let main_branch = test_branch("main", 10, false, false);
        assert!(main_branch.is_protected(&protected));

        // Test remote branch protection
        let remote_main = test_branch("origin/main", 10, false, true);
        assert!(remote_main.is_protected(&protected));
    }

    #[test]
    fn test_glob_match_exact() {
        assert!(Branch::glob_match("main", "main"));
        assert!(!Branch::glob_match("main", "develop"));
    }

    #[test]
    fn test_glob_match_prefix() {
        assert!(Branch::glob_match("wip/*", "wip/test"));
        assert!(Branch::glob_match("wip/*", "wip/feature/test"));
        assert!(!Branch::glob_match("wip/*", "feature/wip"));
    }

    #[test]
    fn test_glob_match_suffix() {
        assert!(Branch::glob_match("*/draft", "feature/draft"));
        assert!(Branch::glob_match("*/draft", "test/feature/draft"));
        assert!(!Branch::glob_match("*/draft", "draft/feature"));
    }

    #[test]
    fn test_glob_match_middle() {
        assert!(Branch::glob_match("feature/*/temp", "feature/test/temp"));
        assert!(Branch::glob_match("feature/*/temp", "feature/foo/bar/temp"));
        assert!(!Branch::glob_match("feature/*/temp", "feature/temp"));
    }

    #[test]
    fn test_glob_match_multiple_wildcards() {
        assert!(Branch::glob_match("*/*/test", "a/b/test"));
        assert!(Branch::glob_match("*/test/*", "a/test/b"));
        assert!(Branch::glob_match("*test*", "mytest123"));
    }

    #[test]
    fn test_branch_matches_exclude_pattern() {
        let branch = test_branch("wip/feature", 10, false, false);
        let patterns = vec!["wip/*".to_string(), "*/draft".to_string()];
        assert!(branch.matches_exclude_pattern(&patterns));

        let draft_branch = test_branch("feature/draft", 10, false, false);
        assert!(draft_branch.matches_exclude_pattern(&patterns));

        let normal_branch = test_branch("feature/test", 10, false, false);
        assert!(!normal_branch.matches_exclude_pattern(&patterns));
    }

    #[test]
    fn test_filter_by_age() {
        let filter = BranchFilter {
            min_age_days: 30,
            ..Default::default()
        };

        let old_branch = test_branch("old", 45, false, false);
        assert!(filter.matches(&old_branch));

        let new_branch = test_branch("new", 15, false, false);
        assert!(!filter.matches(&new_branch));

        let exact_age = test_branch("exact", 30, false, false);
        assert!(filter.matches(&exact_age));
    }

    #[test]
    fn test_filter_local_only() {
        let filter = BranchFilter {
            local_only: true,
            ..Default::default()
        };

        let local = test_branch("feature", 45, false, false);
        assert!(filter.matches(&local));

        let remote = test_branch("origin/feature", 45, false, true);
        assert!(!filter.matches(&remote));
    }

    #[test]
    fn test_filter_remote_only() {
        let filter = BranchFilter {
            remote_only: true,
            ..Default::default()
        };

        let local = test_branch("feature", 45, false, false);
        assert!(!filter.matches(&local));

        let remote = test_branch("origin/feature", 45, false, true);
        assert!(filter.matches(&remote));
    }

    #[test]
    fn test_filter_merged_only() {
        let filter = BranchFilter {
            merged_only: true,
            ..Default::default()
        };

        let merged = test_branch("feature", 45, true, false);
        assert!(filter.matches(&merged));

        let unmerged = test_branch("feature", 45, false, false);
        assert!(!filter.matches(&unmerged));
    }

    #[test]
    fn test_filter_protected_branches() {
        let filter = BranchFilter {
            protected_branches: vec!["main".to_string(), "develop".to_string()],
            ..Default::default()
        };

        let feature = test_branch("feature", 45, false, false);
        assert!(filter.matches(&feature));

        let main = test_branch("main", 45, false, false);
        assert!(!filter.matches(&main));

        let develop = test_branch("develop", 45, false, false);
        assert!(!filter.matches(&develop));
    }

    #[test]
    fn test_filter_exclude_patterns() {
        let filter = BranchFilter {
            exclude_patterns: vec!["wip/*".to_string(), "*/draft".to_string()],
            ..Default::default()
        };

        let feature = test_branch("feature/test", 45, false, false);
        assert!(filter.matches(&feature));

        let wip = test_branch("wip/feature", 45, false, false);
        assert!(!filter.matches(&wip));

        let draft = test_branch("feature/draft", 45, false, false);
        assert!(!filter.matches(&draft));
    }

    #[test]
    fn test_filter_combined() {
        let filter = BranchFilter {
            min_age_days: 30,
            merged_only: true,
            local_only: true,
            remote_only: false,
            protected_branches: vec!["main".to_string()],
            exclude_patterns: vec!["wip/*".to_string()],
        };

        // Should match: old, merged, local, not protected, not WIP
        let good = test_branch("feature/old", 45, true, false);
        assert!(filter.matches(&good));

        // Too young
        let too_young = test_branch("feature/new", 15, true, false);
        assert!(!filter.matches(&too_young));

        // Not merged
        let unmerged = test_branch("feature/unmerged", 45, false, false);
        assert!(!filter.matches(&unmerged));

        // Remote
        let remote = test_branch("origin/feature", 45, true, true);
        assert!(!filter.matches(&remote));

        // Protected
        let protected = test_branch("main", 45, true, false);
        assert!(!filter.matches(&protected));

        // WIP
        let wip = test_branch("wip/feature", 45, true, false);
        assert!(!filter.matches(&wip));
    }

    #[test]
    fn test_sort_branches_by_merge_status() {
        let mut branches = vec![
            test_branch("merged1", 20, true, false),
            test_branch("unmerged1", 30, false, false),
            test_branch("merged2", 10, true, false),
            test_branch("unmerged2", 40, false, false),
        ];

        sort_branches(&mut branches);

        // Merged branches should come first
        assert!(branches[0].is_merged);
        assert!(branches[1].is_merged);
        assert!(!branches[2].is_merged);
        assert!(!branches[3].is_merged);
    }

    #[test]
    fn test_sort_branches_by_age_within_merged_status() {
        let mut branches = vec![
            test_branch("unmerged_newer", 20, false, false),
            test_branch("unmerged_older", 40, false, false),
            test_branch("merged_newer", 10, true, false),
            test_branch("merged_older", 30, true, false),
        ];

        sort_branches(&mut branches);

        // Within merged: oldest first (30 before 10)
        assert_eq!(branches[0].name, "merged_older");
        assert_eq!(branches[1].name, "merged_newer");

        // Within unmerged: oldest first (40 before 20)
        assert_eq!(branches[2].name, "unmerged_older");
        assert_eq!(branches[3].name, "unmerged_newer");
    }

    #[test]
    fn test_age_severity_fresh() {
        let branch = test_branch("test", 0, false, false);
        assert_eq!(branch.age_severity(), AgeSeverity::Fresh);
        let branch = test_branch("test", 30, false, false);
        assert_eq!(branch.age_severity(), AgeSeverity::Fresh);
    }

    #[test]
    fn test_age_severity_moderate() {
        let branch = test_branch("test", 31, false, false);
        assert_eq!(branch.age_severity(), AgeSeverity::Moderate);
        let branch = test_branch("test", 90, false, false);
        assert_eq!(branch.age_severity(), AgeSeverity::Moderate);
    }

    #[test]
    fn test_age_severity_stale() {
        let branch = test_branch("test", 91, false, false);
        assert_eq!(branch.age_severity(), AgeSeverity::Stale);
        let branch = test_branch("test", 365, false, false);
        assert_eq!(branch.age_severity(), AgeSeverity::Stale);
    }
}
