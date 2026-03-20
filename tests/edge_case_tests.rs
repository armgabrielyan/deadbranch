//! Additional integration tests for edge cases and git operations

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// Helper to create a test git repository with a commit
fn create_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Initialize git repo with explicit main branch
    StdCommand::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Set git config (required for commits)
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

    // Create initial commit on main branch
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

#[test]
#[allow(deprecated)]
fn test_list_merged_branches_only() {
    let repo = create_test_repo();

    // Create and merge a branch
    fs::write(repo.path().join("feature.txt"), "feature").unwrap();
    StdCommand::new("git")
        .args(["checkout", "-b", "merged-feature"])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add feature"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // Make it old
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_timestamp = now - (45 * 86400);
    let date = format!("@{}", old_timestamp);
    StdCommand::new("git")
        .args(["commit", "--amend", "--no-edit", "--date", &date])
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(&repo)
        .output()
        .unwrap();

    // Merge it
    StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["merge", "merged-feature", "--no-ff", "-m", "Merge feature"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // Test --merged flag
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["list", "--merged"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("merged-feature"));
}

#[test]
#[allow(deprecated)]
fn test_list_shows_age_information() {
    let repo = create_test_repo();

    // Create an old branch
    fs::write(repo.path().join("test.txt"), "test").unwrap();
    StdCommand::new("git")
        .args(["checkout", "-b", "old-branch"])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Test"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // Make it 50 days old
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_timestamp = now - (50 * 86400);
    let date = format!("@{}", old_timestamp);
    StdCommand::new("git")
        .args(["commit", "--amend", "--no-edit", "--date", &date])
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(&repo)
        .output()
        .unwrap();

    StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo)
        .output()
        .unwrap();

    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("old-branch"))
        .stdout(predicate::str::contains("days").or(predicate::str::contains("day")));
}

#[test]
#[allow(deprecated)]
fn test_multiple_old_branches() {
    let repo = create_test_repo();

    // Create multiple old branches
    for i in 1..=3 {
        let branch_name = format!("feature-{}", i);
        fs::write(
            repo.path().join(format!("file{}.txt", i)),
            format!("content {}", i),
        )
        .unwrap();
        StdCommand::new("git")
            .args(["checkout", "-b", &branch_name])
            .current_dir(&repo)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["add", "."])
            .current_dir(&repo)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["commit", "-m", &format!("Add {}", branch_name)])
            .current_dir(&repo)
            .output()
            .unwrap();

        // Make it old
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let old_timestamp = now - ((40 + i * 5) as u64 * 86400);
        let date = format!("@{}", old_timestamp);
        StdCommand::new("git")
            .args(["commit", "--amend", "--no-edit", "--date", &date])
            .env("GIT_COMMITTER_DATE", &date)
            .current_dir(&repo)
            .output()
            .unwrap();

        StdCommand::new("git")
            .args(["checkout", "main"])
            .current_dir(&repo)
            .output()
            .unwrap();
    }

    // Should show all 3 branches
    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("feature-1"))
        .stdout(predicate::str::contains("feature-2"))
        .stdout(predicate::str::contains("feature-3"));
}

#[test]
#[allow(deprecated)]
fn test_current_branch_excluded() {
    let repo = create_test_repo();

    // Create and stay on a branch
    StdCommand::new("git")
        .args(["checkout", "-b", "current-branch"])
        .current_dir(&repo)
        .output()
        .unwrap();

    fs::write(repo.path().join("test.txt"), "test").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Test"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // Make it old
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_timestamp = now - (50 * 86400);
    let date = format!("@{}", old_timestamp);
    StdCommand::new("git")
        .args(["commit", "--amend", "--no-edit", "--date", &date])
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(&repo)
        .output()
        .unwrap();

    // Current branch should not be listed
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["list", "--days", "1"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("current-branch").not());
}

#[test]
#[allow(deprecated)]
fn test_list_shows_merged_status() {
    let repo = create_test_repo();

    // Create a merged branch
    fs::write(repo.path().join("merged.txt"), "merged").unwrap();
    StdCommand::new("git")
        .args(["checkout", "-b", "merged-branch"])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Merged content"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // Make it old
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_timestamp = now - (45 * 86400);
    let date = format!("@{}", old_timestamp);
    StdCommand::new("git")
        .args(["commit", "--amend", "--no-edit", "--date", &date])
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(&repo)
        .output()
        .unwrap();

    StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["merge", "merged-branch", "--no-ff", "-m", "Merge"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // List should show merged status (either via icon or text)
    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("merged-branch"));
}

#[test]
#[allow(deprecated)]
fn test_squash_merged_branch_detected_as_merged() {
    let repo = create_test_repo();

    // Create a branch with a commit
    StdCommand::new("git")
        .args(["checkout", "-b", "squash-feature"])
        .current_dir(&repo)
        .output()
        .unwrap();
    fs::write(repo.path().join("squash.txt"), "squash content").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add squash feature"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // Make it old enough to show in list
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_timestamp = now - (45 * 86400);
    let date = format!("@{}", old_timestamp);
    StdCommand::new("git")
        .args(["commit", "--amend", "--no-edit", "--date", &date])
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(&repo)
        .output()
        .unwrap();

    // Squash-merge the branch into main
    StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["merge", "--squash", "squash-feature"])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Squash merge squash-feature"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // The squash-merged branch should appear in --merged list
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["list", "--merged"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("squash-feature"));
}

#[test]
#[allow(deprecated)]
fn test_rebase_merged_branch_detected_as_merged() {
    let repo = create_test_repo();

    // Create a branch with a commit
    StdCommand::new("git")
        .args(["checkout", "-b", "rebase-feature"])
        .current_dir(&repo)
        .output()
        .unwrap();
    fs::write(repo.path().join("rebase.txt"), "rebase content").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add rebase feature"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // Make it old enough to show in list
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_timestamp = now - (45 * 86400);
    let date = format!("@{}", old_timestamp);
    StdCommand::new("git")
        .args(["commit", "--amend", "--no-edit", "--date", &date])
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(&repo)
        .output()
        .unwrap();

    // Cherry-pick the commit onto main (simulates rebase-merge)
    StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["cherry-pick", "rebase-feature"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // The cherry-picked branch should appear in --merged list
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["list", "--merged"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("rebase-feature"));
}

#[test]
#[allow(deprecated)]
fn test_truly_unmerged_branch_not_detected_as_merged() {
    let repo = create_test_repo();

    // Create a branch with a commit but never merge it
    StdCommand::new("git")
        .args(["checkout", "-b", "unmerged-feature"])
        .current_dir(&repo)
        .output()
        .unwrap();
    fs::write(repo.path().join("unmerged.txt"), "unmerged content").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add unmerged feature"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // Make it old enough to show in list
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_timestamp = now - (45 * 86400);
    let date = format!("@{}", old_timestamp);
    StdCommand::new("git")
        .args(["commit", "--amend", "--no-edit", "--date", &date])
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(&repo)
        .output()
        .unwrap();

    StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // The unmerged branch should NOT appear in --merged list
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["list", "--merged"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("unmerged-feature").not());
}
