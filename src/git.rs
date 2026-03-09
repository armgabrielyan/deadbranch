//! Git operations - shells out to git CLI for reliability

use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use std::process::Command;

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
    let mut branches = Vec::new();

    // Get local branches
    let local_branches = list_local_branches(default_branch)?;
    branches.extend(local_branches);

    // Get remote branches
    let remote_branches = list_remote_branches(default_branch)?;
    branches.extend(remote_branches);

    Ok(branches)
}

/// List local branches with metadata
fn list_local_branches(default_branch: &str) -> Result<Vec<Branch>> {
    // Format: refname:short, authordate:unix, objectname:short, authorname
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
        let is_merged = check_branch_merged(&name, default_branch)?;

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
fn list_remote_branches(default_branch: &str) -> Result<Vec<Branch>> {
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
        let is_merged = check_branch_merged(&name, default_branch)?;

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

/// Check if a branch is merged into the default branch
fn check_branch_merged(branch: &str, default_branch: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["branch", "--merged", default_branch, "-a"])
        .output()
        .context("Failed to check merged branches")?;

    if !output.status.success() {
        // If the command fails, assume not merged
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let line = line.trim().trim_start_matches("* ");
        // Handle both local and remote branch names
        if line == branch || line == format!("remotes/{}", branch) {
            return Ok(true);
        }
    }

    Ok(false)
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

/// Delete a remote branch
pub fn delete_remote_branch(branch: &str) -> Result<()> {
    // Extract the branch name without origin/ prefix
    let branch_name = branch.strip_prefix("origin/").unwrap_or(branch);

    let output = Command::new("git")
        .args(["push", "origin", "--delete", branch_name])
        .output()
        .context("Failed to delete remote branch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to delete remote branch '{}': {}", branch, stderr);
    }

    Ok(())
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
