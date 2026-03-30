//! Additional integration tests for edge cases and git operations

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;

use common::create_test_repo;

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
    common::create_branch(repo.path(), "squash-feature");
    common::make_branch_old(repo.path(), "squash-feature", 45);

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
    common::create_branch(repo.path(), "rebase-feature");
    common::make_branch_old(repo.path(), "rebase-feature", 45);

    // Cherry-pick onto main (simulates rebase-merge)
    StdCommand::new("git")
        .args(["cherry-pick", "rebase-feature"])
        .current_dir(&repo)
        .output()
        .unwrap();

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
    common::create_branch(repo.path(), "unmerged-feature");
    common::make_branch_old(repo.path(), "unmerged-feature", 45);

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["list", "--merged"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("unmerged-feature").not());
}

#[test]
#[allow(deprecated)]
fn test_clean_deletes_squash_merged_branch() {
    let repo = create_test_repo();
    common::create_branch(repo.path(), "squash-clean-test");
    common::make_branch_old(repo.path(), "squash-clean-test", 45);

    StdCommand::new("git")
        .args(["merge", "--squash", "squash-clean-test"])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Squash merge squash-clean-test"])
        .current_dir(&repo)
        .output()
        .unwrap();

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("squash-clean-test"));

    let out = StdCommand::new("git")
        .args(["branch", "--list", "squash-clean-test"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("squash-clean-test"),
        "squash-merged branch should have been deleted"
    );
}

#[test]
#[allow(deprecated)]
fn test_clean_deletes_rebase_merged_branch() {
    let repo = create_test_repo();
    common::create_branch(repo.path(), "rebase-clean-test");
    common::make_branch_old(repo.path(), "rebase-clean-test", 45);

    // Cherry-pick onto main (simulates rebase-merge)
    StdCommand::new("git")
        .args(["cherry-pick", "rebase-clean-test"])
        .current_dir(&repo)
        .output()
        .unwrap();

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("rebase-clean-test"));

    let out = StdCommand::new("git")
        .args(["branch", "--list", "rebase-clean-test"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("rebase-clean-test"),
        "rebase-merged branch should have been deleted"
    );
}
