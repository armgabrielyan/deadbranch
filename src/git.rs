//! Git operations - shells out to git CLI for reliability

use std::collections::HashSet;
use std::process::Command;

use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};

use crate::branch::Branch;
use crate::error::DeadbranchError;

/// Check if we're in a git repository
pub fn is_git_repository() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get the default branch (main, master, etc.)
pub fn get_default_branch() -> Result<String> {
    // Try to get from remote HEAD
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .output()
        .context("Failed to run git command")?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout)
            .trim()
            .strip_prefix("origin/")
            .unwrap_or("main")
            .to_string();
        return Ok(branch);
    }

    // Fallback: check if main or master exists
    for branch in &["main", "master"] {
        let output = Command::new("git")
            .args(["rev-parse", "--verify", &format!("refs/heads/{}", branch)])
            .output()
            .context("Failed to run git command")?;

        if output.status.success() {
            return Ok(branch.to_string());
        }
    }

    // Last resort: use main
    Ok("main".to_string())
}

/// Get the current branch name
pub fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .context("Failed to run git command")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get current branch");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Fetch and prune remote branches
pub fn fetch_and_prune() -> Result<()> {
    let output = Command::new("git")
        .args(["fetch", "--prune"])
        .output()
        .context("Failed to run git fetch --prune")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git fetch --prune failed: {}", stderr);
    }

    Ok(())
}

/// List all branches (local and remote)
pub fn list_branches(default_branch: &str) -> Result<Vec<Branch>> {
    // Fetch all merged branches once (instead of per-branch)
    let merged = get_merged_branches(default_branch)?;

    let mut branches = list_local_branches(&merged)?;
    branches.extend(list_remote_branches(default_branch, &merged)?);

    Ok(branches)
}

/// Get the set of all branches merged into the default branch.
/// Called once and shared across local/remote listing for O(1) lookups.
fn get_merged_branches(default_branch: &str) -> Result<HashSet<String>> {
    let output = Command::new("git")
        .args(["branch", "--merged", default_branch, "-a"])
        .output()
        .context("Failed to check merged branches")?;

    if !output.status.success() {
        return Ok(HashSet::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_merged_branches(&stdout))
}

/// Parse `git branch --merged` output into a set of branch names.
/// Handles local branches, current branch marker (`*`), and remote refs
/// (inserting both `remotes/origin/foo` and `origin/foo` forms).
fn parse_merged_branches(stdout: &str) -> HashSet<String> {
    let mut merged = HashSet::new();

    for line in stdout.lines() {
        let name = line.trim().trim_start_matches("* ");
        if name.is_empty() {
            continue;
        }
        merged.insert(name.to_string());
        // Also insert without "remotes/" prefix for remote branch lookups
        if let Some(stripped) = name.strip_prefix("remotes/") {
            merged.insert(stripped.to_string());
        }
    }

    merged
}

/// List local branches with metadata
fn list_local_branches(merged: &HashSet<String>) -> Result<Vec<Branch>> {
    let output = Command::new("git")
        .args([
            "for-each-ref",
            "--format=%(refname:short)|%(authordate:unix)|%(objectname:short)|%(authorname)",
            "refs/heads/",
        ])
        .output()
        .context("Failed to list local branches")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list local branches: {}", stderr);
    }

    let current_branch = get_current_branch().unwrap_or_default();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let now = Utc::now();

    let mut branches = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 4 {
            continue;
        }

        let name = parts[0].to_string();
        let timestamp: i64 = parts[1].parse().unwrap_or(0);
        let sha = parts[2].to_string();
        let author = parts[3].to_string();

        // Skip current branch
        if name == current_branch {
            continue;
        }

        let commit_date = Utc.timestamp_opt(timestamp, 0).unwrap();
        let age_days = (now - commit_date).num_days();
        let is_merged = merged.contains(&name);

        branches.push(Branch {
            name,
            age_days,
            is_merged,
            is_remote: false,
            last_commit_sha: sha,
            last_commit_date: commit_date,
            last_commit_author: author,
        });
    }

    Ok(branches)
}

/// List remote branches with metadata
fn list_remote_branches(default_branch: &str, merged: &HashSet<String>) -> Result<Vec<Branch>> {
    let output = Command::new("git")
        .args([
            "for-each-ref",
            "--format=%(refname:short)|%(authordate:unix)|%(objectname:short)|%(authorname)",
            "refs/remotes/origin/",
        ])
        .output()
        .context("Failed to list remote branches")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list remote branches: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let now = Utc::now();

    let mut branches = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 4 {
            continue;
        }

        let name = parts[0].to_string();
        let timestamp: i64 = parts[1].parse().unwrap_or(0);
        let sha = parts[2].to_string();
        let author = parts[3].to_string();

        // Skip HEAD pointer and default branch
        if name == "origin/HEAD" || name == format!("origin/{}", default_branch) {
            continue;
        }

        let commit_date = Utc.timestamp_opt(timestamp, 0).unwrap();
        let age_days = (now - commit_date).num_days();
        let is_merged = merged.contains(&name);

        branches.push(Branch {
            name,
            age_days,
            is_merged,
            is_remote: true,
            last_commit_sha: sha,
            last_commit_date: commit_date,
            last_commit_author: author,
        });
    }

    Ok(branches)
}

/// Delete a local branch
pub fn delete_local_branch(branch: &str, force: bool) -> Result<()> {
    let flag = if force { "-D" } else { "-d" };

    let output = Command::new("git")
        .args(["branch", flag, branch])
        .output()
        .context("Failed to delete branch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not fully merged") {
            return Err(DeadbranchError::UnmergedBranch(branch.to_string()).into());
        }
        anyhow::bail!("Failed to delete branch '{}': {}", branch, stderr);
    }

    Ok(())
}

/// Batch delete remote branches in a single `git push` command.
///
/// Returns a Vec of `(branch_name, success, optional_error)` in the same
/// order as the input. Uses one network round-trip instead of N.
pub fn delete_remote_branches_batch(
    branches: &[String],
) -> Result<Vec<(String, bool, Option<String>)>> {
    if branches.is_empty() {
        return Ok(Vec::new());
    }

    let names: Vec<&str> = branches
        .iter()
        .map(|b| b.strip_prefix("origin/").unwrap_or(b.as_str()))
        .collect();

    let mut args = vec!["push", "origin", "--delete"];
    args.extend(&names);

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to run git push --delete")?;

    // All succeeded
    if output.status.success() {
        return Ok(branches.iter().map(|b| (b.clone(), true, None)).collect());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(parse_batch_delete_stderr(&stderr, branches, &names))
}

/// Parse `git push --delete` stderr to determine per-branch success/failure.
///
/// `branches` are the original names (e.g. `origin/feat/x`), `names` are the
/// stripped refspec names passed to git (e.g. `feat/x`).
fn parse_batch_delete_stderr(
    stderr: &str,
    branches: &[String],
    names: &[&str],
) -> Vec<(String, bool, Option<String>)> {
    // Connection-level failure: no branches were deleted
    if stderr.contains("Could not resolve host")
        || stderr.contains("unable to access")
        || stderr.contains("Connection refused")
        || stderr.contains("fatal: the remote end hung up")
    {
        let err = stderr.trim().to_string();
        return branches
            .iter()
            .map(|b| (b.clone(), false, Some(err.clone())))
            .collect();
    }

    // Partial failure: determine per-branch status from stderr.
    // Git reports failures as: error: unable to delete '<name>': ...
    // Branches not mentioned in error lines were deleted successfully.
    branches
        .iter()
        .zip(names.iter())
        .map(|(branch, &name)| {
            if stderr.contains(&format!("unable to delete '{}'", name)) {
                let err = stderr
                    .lines()
                    .find(|l| l.contains(name) && l.starts_with("error"))
                    .unwrap_or("remote ref does not exist")
                    .trim()
                    .to_string();
                (branch.clone(), false, Some(err))
            } else {
                (branch.clone(), true, None)
            }
        })
        .collect()
}

/// Get the SHA for a branch (for backup purposes)
pub fn get_branch_sha(branch: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", branch])
        .output()
        .context("Failed to get branch SHA")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get SHA for branch '{}'", branch);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_merged_local_branches() {
        let output = "  feature/auth\n  bugfix/login\n  cleanup/old-stuff\n";
        let merged = parse_merged_branches(output);
        assert!(merged.contains("feature/auth"));
        assert!(merged.contains("bugfix/login"));
        assert!(merged.contains("cleanup/old-stuff"));
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn parse_merged_current_branch_marker() {
        let output = "* main\n  feature/auth\n";
        let merged = parse_merged_branches(output);
        assert!(merged.contains("main"));
        assert!(merged.contains("feature/auth"));
        assert!(!merged.contains("* main"));
    }

    #[test]
    fn parse_merged_remote_branches() {
        let output = "  remotes/origin/feature/auth\n  remotes/origin/bugfix/login\n";
        let merged = parse_merged_branches(output);
        // Both full and stripped forms
        assert!(merged.contains("remotes/origin/feature/auth"));
        assert!(merged.contains("origin/feature/auth"));
        assert!(merged.contains("remotes/origin/bugfix/login"));
        assert!(merged.contains("origin/bugfix/login"));
    }

    #[test]
    fn parse_merged_mixed_local_and_remote() {
        let output = "\
* main
  feature/done
  remotes/origin/feature/done
  remotes/origin/cleanup/old
";
        let merged = parse_merged_branches(output);
        assert!(merged.contains("main"));
        assert!(merged.contains("feature/done"));
        assert!(merged.contains("origin/feature/done"));
        assert!(merged.contains("origin/cleanup/old"));
    }

    #[test]
    fn parse_merged_empty_output() {
        let merged = parse_merged_branches("");
        assert!(merged.is_empty());
    }

    #[test]
    fn parse_merged_blank_lines_ignored() {
        let output = "  feature/auth\n\n  \n  bugfix/login\n";
        let merged = parse_merged_branches(output);
        assert!(merged.contains("feature/auth"));
        assert!(merged.contains("bugfix/login"));
        assert!(!merged.contains(""));
    }

    #[test]
    fn parse_merged_lookup_matches_local_branch() {
        let output = "  feature/auth\n  remotes/origin/feature/auth\n";
        let merged = parse_merged_branches(output);
        // Local branch lookup
        assert!(merged.contains("feature/auth"));
        // Remote branch lookup (as used by list_remote_branches)
        assert!(merged.contains("origin/feature/auth"));
    }

    // ── Batch delete stderr parsing ────────────────────────────────

    #[test]
    fn batch_delete_empty_input() {
        let results = parse_batch_delete_stderr("", &[], &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn batch_delete_all_succeed() {
        // When stderr has no error markers, all branches are considered successful
        let stderr = "To github.com:user/repo.git\n - [deleted]         feat/a\n - [deleted]         feat/b\n";
        let branches = vec!["origin/feat/a".to_string(), "origin/feat/b".to_string()];
        let names = vec!["feat/a", "feat/b"];

        let results = parse_batch_delete_stderr(stderr, &branches, &names);
        assert_eq!(results.len(), 2);
        assert!(results[0].1); // success
        assert!(results[0].2.is_none());
        assert!(results[1].1);
        assert!(results[1].2.is_none());
    }

    #[test]
    fn batch_delete_partial_failure() {
        let stderr = "\
error: unable to delete 'feat/gone': remote ref does not exist
To github.com:user/repo.git
 - [deleted]         feat/ok
 ! [remote rejected] feat/gone (remote ref does not exist)
error: failed to push some refs to 'github.com:user/repo.git'
";
        let branches = vec!["origin/feat/ok".to_string(), "origin/feat/gone".to_string()];
        let names = vec!["feat/ok", "feat/gone"];

        let results = parse_batch_delete_stderr(stderr, &branches, &names);
        assert_eq!(results.len(), 2);

        // feat/ok succeeded (not mentioned in error lines)
        assert!(results[0].1);
        assert!(results[0].2.is_none());

        // feat/gone failed
        assert!(!results[1].1);
        assert!(results[1].2.as_ref().unwrap().contains("unable to delete"));
    }

    #[test]
    fn batch_delete_connection_failure() {
        let stderr = "fatal: unable to access 'https://github.com/user/repo.git/': Could not resolve host: github.com\n";
        let branches = vec!["origin/feat/a".to_string(), "origin/feat/b".to_string()];
        let names = vec!["feat/a", "feat/b"];

        let results = parse_batch_delete_stderr(stderr, &branches, &names);
        assert_eq!(results.len(), 2);

        // All fail with same connection error
        for (_, success, error) in &results {
            assert!(!success);
            assert!(error.as_ref().unwrap().contains("Could not resolve host"));
        }
    }

    #[test]
    fn batch_delete_connection_refused() {
        let stderr = "fatal: Connection refused\n";
        let branches = vec!["origin/feat/x".to_string()];
        let names = vec!["feat/x"];

        let results = parse_batch_delete_stderr(stderr, &branches, &names);
        assert_eq!(results.len(), 1);
        assert!(!results[0].1);
        assert!(results[0]
            .2
            .as_ref()
            .unwrap()
            .contains("Connection refused"));
    }

    #[test]
    fn batch_delete_strips_origin_prefix() {
        // Verify the function works with names that already had origin/ stripped
        let stderr = "error: unable to delete 'cleanup/old': remote ref does not exist\n";
        let branches = vec![
            "origin/feat/new".to_string(),
            "origin/cleanup/old".to_string(),
        ];
        let names = vec!["feat/new", "cleanup/old"];

        let results = parse_batch_delete_stderr(stderr, &branches, &names);
        assert!(results[0].1); // feat/new OK
        assert!(!results[1].1); // cleanup/old failed
        assert_eq!(results[0].0, "origin/feat/new");
        assert_eq!(results[1].0, "origin/cleanup/old");
    }

    #[test]
    fn batch_delete_multiple_failures() {
        let stderr = "\
error: unable to delete 'feat/a': remote ref does not exist
error: unable to delete 'feat/c': remote ref does not exist
To github.com:user/repo.git
 - [deleted]         feat/b
error: failed to push some refs to 'github.com:user/repo.git'
";
        let branches = vec![
            "origin/feat/a".to_string(),
            "origin/feat/b".to_string(),
            "origin/feat/c".to_string(),
        ];
        let names = vec!["feat/a", "feat/b", "feat/c"];

        let results = parse_batch_delete_stderr(stderr, &branches, &names);
        assert!(!results[0].1); // feat/a failed
        assert!(results[1].1); // feat/b succeeded
        assert!(!results[2].1); // feat/c failed
    }
}
