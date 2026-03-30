//! Shared test helpers for integration tests

use std::fs;
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// Create a test git repository with an initial commit on `main`
pub fn create_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    StdCommand::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    fs::write(temp_dir.path().join("README.md"), "# Test repo").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    temp_dir
}

/// Create a branch with a commit, then return to `main`
pub fn create_branch(repo_dir: &std::path::Path, branch_name: &str) {
    StdCommand::new("git")
        .args(["checkout", "-b", branch_name])
        .current_dir(repo_dir)
        .output()
        .unwrap();
    fs::write(
        repo_dir.join("test.txt"),
        format!("Content for {}", branch_name),
    )
    .unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", &format!("Add {} content", branch_name)])
        .current_dir(repo_dir)
        .output()
        .unwrap();

    // Return to main (fall back to master if needed)
    let out = StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(repo_dir)
        .output()
        .unwrap();
    if !out.status.success() {
        StdCommand::new("git")
            .args(["checkout", "master"])
            .current_dir(repo_dir)
            .output()
            .unwrap();
    }
}

/// Amend the HEAD commit on `branch_name` to appear `days_old` days old
pub fn make_branch_old(repo_dir: &std::path::Path, branch_name: &str, days_old: u32) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_timestamp = now - (days_old as u64 * 86400);

    StdCommand::new("git")
        .args(["checkout", branch_name])
        .current_dir(repo_dir)
        .output()
        .unwrap();

    let date = format!("@{}", old_timestamp);
    let out = StdCommand::new("git")
        .args(["commit", "--amend", "--no-edit", "--date", &date])
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(repo_dir)
        .output()
        .unwrap();
    if !out.status.success() {
        eprintln!(
            "Warning: git commit --amend failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // Return to main (fall back to master if needed)
    let checkout = StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(repo_dir)
        .output()
        .unwrap();
    if !checkout.status.success() {
        StdCommand::new("git")
            .args(["checkout", "master"])
            .current_dir(repo_dir)
            .output()
            .unwrap();
    }
}
