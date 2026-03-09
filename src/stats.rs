//! Branch statistics computation

use crate::branch::Branch;

/// Aggregated branch statistics for the current repository.
/// Covers all branches visible to deadbranch (protected/excluded already filtered out).
/// The `threshold_days` value drives stale/safe labels but does not affect which
/// branches are counted.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RepoStats {
    pub total: usize,
    pub local: usize,
    pub remote: usize,
    pub merged: usize,
    pub merged_local: usize,
    pub merged_remote: usize,
    pub unmerged: usize,
    pub unmerged_local: usize,
    pub unmerged_remote: usize,
    /// Branches with age_days >= threshold_days
    pub stale: usize,
    pub stale_local: usize,
    pub stale_remote: usize,
    /// Merged AND stale — mirrors `clean`'s default deletion criteria
    pub safe_to_delete: usize,
    pub safe_local: usize,
    pub safe_remote: usize,
    /// Branches with age_days in [0, 7)
    pub age_lt7: usize,
    /// Branches with age_days in [7, 30)
    pub age_7_30: usize,
    /// Branches with age_days in [30, 90)
    pub age_30_90: usize,
    /// Branches with age_days >= 90
    pub age_gt90: usize,
    pub threshold_days: u32,
}

/// Compute statistics from a pre-filtered branch list.
/// `branches` should already have protected/excluded branches removed.
/// `threshold_days` defines the staleness boundary.
pub fn compute_stats(branches: &[Branch], threshold_days: u32) -> RepoStats {
    let mut s = RepoStats {
        threshold_days,
        ..RepoStats::default()
    };

    for branch in branches {
        s.total += 1;

        if branch.is_remote {
            s.remote += 1;
        } else {
            s.local += 1;
        }

        if branch.is_merged {
            s.merged += 1;
            if branch.is_remote {
                s.merged_remote += 1;
            } else {
                s.merged_local += 1;
            }
        } else {
            s.unmerged += 1;
            if branch.is_remote {
                s.unmerged_remote += 1;
            } else {
                s.unmerged_local += 1;
            }
        }

        // Negative age_days (clock-skewed commits) is treated as not-stale, which is correct.
        let is_stale = branch.age_days >= threshold_days as i64;
        if is_stale {
            s.stale += 1;
            if branch.is_remote {
                s.stale_remote += 1;
            } else {
                s.stale_local += 1;
            }
        }

        if branch.is_merged && is_stale {
            s.safe_to_delete += 1;
            if branch.is_remote {
                s.safe_remote += 1;
            } else {
                s.safe_local += 1;
            }
        }

        match branch.age_days {
            d if d < 7 => s.age_lt7 += 1,
            d if d < 30 => s.age_7_30 += 1,
            d if d < 90 => s.age_30_90 += 1,
            _ => s.age_gt90 += 1,
        }
    }

    s
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

    #[test]
    fn test_empty() {
        let stats = compute_stats(&[], 30);
        assert_eq!(
            stats,
            RepoStats {
                threshold_days: 30,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_local_remote_split() {
        let branches = vec![
            test_branch("local1", 10, false, false),
            test_branch("local2", 10, false, false),
            test_branch("origin/remote1", 10, false, true),
        ];
        let stats = compute_stats(&branches, 30);
        assert_eq!(stats.total, 3);
        assert_eq!(stats.local, 2);
        assert_eq!(stats.remote, 1);
    }

    #[test]
    fn test_merged_unmerged_split() {
        let branches = vec![
            test_branch("a", 10, true, false),
            test_branch("b", 10, true, true),
            test_branch("c", 10, false, false),
        ];
        let stats = compute_stats(&branches, 30);
        assert_eq!(stats.merged, 2);
        assert_eq!(stats.merged_local, 1);
        assert_eq!(stats.merged_remote, 1);
        assert_eq!(stats.unmerged, 1);
        assert_eq!(stats.unmerged_local, 1);
        assert_eq!(stats.unmerged_remote, 0);
    }

    #[test]
    fn test_stale_threshold() {
        let branches = vec![
            test_branch("fresh", 29, false, false),
            test_branch("exact", 30, false, false),
            test_branch("old", 60, false, true),
        ];
        let stats = compute_stats(&branches, 30);
        assert_eq!(stats.stale, 2); // age 30 and 60
        assert_eq!(stats.stale_local, 1); // age 30
        assert_eq!(stats.stale_remote, 1); // age 60
    }

    #[test]
    fn test_safe_to_delete_requires_merged_and_stale() {
        let branches = vec![
            test_branch("merged-fresh", 10, true, false), // merged but not stale
            test_branch("unmerged-stale", 40, false, false), // stale but not merged
            test_branch("safe-local", 40, true, false),   // merged + stale ✓
            test_branch("safe-remote", 50, true, true),   // merged + stale ✓
        ];
        let stats = compute_stats(&branches, 30);
        assert_eq!(stats.safe_to_delete, 2);
        assert_eq!(stats.safe_local, 1);
        assert_eq!(stats.safe_remote, 1);
    }

    #[test]
    fn test_age_buckets() {
        let branches = vec![
            test_branch("a", 3, false, false),   // < 7
            test_branch("b", 6, false, false),   // < 7
            test_branch("c", 7, false, false),   // 7–30
            test_branch("d", 29, false, false),  // 7–30
            test_branch("e", 30, false, false),  // 30–90
            test_branch("f", 89, false, false),  // 30–90
            test_branch("g", 90, false, false),  // > 90
            test_branch("h", 200, false, false), // > 90
        ];
        let stats = compute_stats(&branches, 30);
        assert_eq!(stats.age_lt7, 2);
        assert_eq!(stats.age_7_30, 2);
        assert_eq!(stats.age_30_90, 2);
        assert_eq!(stats.age_gt90, 2);
    }
}
