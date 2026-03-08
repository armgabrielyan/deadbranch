//! UI utilities - output formatting, prompts, tables

use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, Table};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::time::Duration;

use crate::backup::format_bytes;
use crate::backup::BackupInfo;
use crate::backup::{
    BackupBranchEntry, BackupStats, BackupToDelete, CleanResult, RestoreError, RestoreResult,
    SkippedLine,
};
use crate::branch::{AgeSeverity, Branch};
use crate::stats::RepoStats;

/// Generic pluralization helper
pub fn pluralize<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

/// Helper to pluralize "branch" correctly
pub fn pluralize_branch(count: usize) -> &'static str {
    pluralize(count, "branch", "branches")
}

/// Helper to pluralize "Branch" correctly (capitalized)
pub fn pluralize_branch_cap(count: usize) -> &'static str {
    pluralize(count, "Branch", "Branches")
}

/// Create a spinner with a message
pub fn spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner
}

/// Finish spinner with success
pub fn spinner_success(spinner: &ProgressBar, message: &str) {
    spinner.finish_and_clear();
    println!("{} {}", style("✅").green(), message);
}

/// Finish spinner with warning
pub fn spinner_warn(spinner: &ProgressBar, message: &str) {
    spinner.finish_and_clear();
    println!("{} {}", style("⚠️").yellow(), message);
}

/// Display a list of branches in a table
pub fn display_branches(branches: &[Branch], title: &str) {
    if branches.is_empty() {
        println!("{}", style("No stale branches found.").dim());
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("#").add_attribute(Attribute::Bold),
        Cell::new("Branch").add_attribute(Attribute::Bold),
        Cell::new("Age").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new("Type").add_attribute(Attribute::Bold),
        Cell::new("Last Commit").add_attribute(Attribute::Bold),
        Cell::new("Author").add_attribute(Attribute::Bold),
    ]);

    for (i, branch) in branches.iter().enumerate() {
        let status = if branch.is_merged {
            Cell::new("merged").fg(Color::Green)
        } else {
            Cell::new("unmerged").fg(Color::Yellow)
        };

        let branch_type = if branch.is_remote {
            Cell::new("remote").fg(Color::Blue)
        } else {
            Cell::new("local").fg(Color::Cyan)
        };

        table.add_row(vec![
            Cell::new((i + 1).to_string()).fg(Color::DarkGrey),
            Cell::new(&branch.name),
            Cell::new(branch.format_age()).fg(match branch.age_severity() {
                AgeSeverity::Fresh => Color::Green,
                AgeSeverity::Moderate => Color::Yellow,
                AgeSeverity::Stale => Color::Red,
            }),
            status,
            branch_type,
            Cell::new(branch.last_commit_date.format("%Y-%m-%d").to_string()).fg(Color::DarkGrey),
            Cell::new(&branch.last_commit_author),
        ]);
    }

    println!("\n{}", style(title).bold());
    println!("{table}\n");
}

/// Ask for confirmation with nice themed UI
pub fn confirm(prompt: &str, default: bool) -> bool {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .wait_for_newline(true)
        .interact()
        .unwrap_or(false)
}

/// Ask for confirmation to delete local branches with visual summary
pub fn confirm_local_deletion(branches: &[Branch]) -> bool {
    let total = branches.len();
    let merged_count = branches.iter().filter(|b| b.is_merged).count();
    let unmerged_count = total - merged_count;
    let branch_word = pluralize_branch(total);

    // Build a descriptive prompt
    let summary = if unmerged_count > 0 {
        format!(
            "{} {} local {} ({} merged, {} unmerged)?",
            style("Delete").red().bold(),
            style(total).yellow().bold(),
            branch_word,
            style(merged_count).green(),
            style(unmerged_count).yellow()
        )
    } else {
        format!(
            "{} {} local {} (all merged)?",
            style("Delete").yellow().bold(),
            style(total).cyan().bold(),
            branch_word
        )
    };

    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(summary)
        .default(false)
        .wait_for_newline(true)
        .interact()
        .unwrap_or(false)
}

/// Display success message
pub fn success(message: &str) {
    println!("{} {}", style("✅").green().bold(), message);
}

/// Display warning message
pub fn warning(message: &str) {
    println!("{} {}", style("⚠️").yellow().bold(), message);
}

/// Display error message
pub fn error(message: &str) {
    eprintln!("{} {}", style("❌").red().bold(), message);
}

/// Display info message
pub fn info(message: &str) {
    println!("{} {}", style("ℹ️").blue().bold(), message);
}

/// Print a grouped dry-run summary instead of listing every command.
pub fn print_dry_run_summary(total: usize, local_safe: usize, local_force: usize, remote: usize) {
    println!(
        "\n{}\n",
        style("[DRY RUN] No branches will be deleted.")
            .yellow()
            .bold()
    );
    println!(
        "Would execute {} {}:",
        style(total).bold(),
        pluralize(total, "command", "commands")
    );
    if local_safe > 0 {
        println!(
            "  {}  ({} local {})",
            style("git branch -d").dim(),
            local_safe,
            pluralize(local_safe, "branch", "branches")
        );
    }
    if local_force > 0 {
        println!(
            "  {}  ({} local unmerged {})",
            style("git branch -D").dim(),
            local_force,
            pluralize(local_force, "branch", "branches")
        );
    }
    if remote > 0 {
        println!(
            "  {}  ({} remote {})",
            style("git push origin --delete").dim(),
            remote,
            pluralize(remote, "branch", "branches")
        );
    }
    println!("\nRun without {} to execute.", style("--dry-run").bold());
}

/// Display remote deletion warning and get confirmation
/// Returns true if user confirms, false otherwise
pub fn confirm_remote_deletion(branches: &[Branch]) -> bool {
    let count = branches.len();
    let branch_word = pluralize_branch(count);

    println!();
    println!(
        "{}",
        style(format!(
            "⚠️  WARNING: You are about to delete remote {}!",
            branch_word
        ))
        .yellow()
        .bold()
    );
    println!();
    println!("This action:");
    println!("  • {} easily", style("Cannot be undone").red());
    println!("  • Will {} all team members", style("affect").red());
    println!(
        "  • Removes {} from origin {}",
        branch_word,
        style("permanently").red()
    );
    println!();

    // Simple confirmation text with just the count
    let expected = format!("delete {} remote {}", count, branch_word);
    println!(
        "To confirm, type exactly: {}",
        style(format!("\"{}\"", expected)).yellow()
    );
    println!();

    let term = console::Term::stdout();
    let _ = term.show_cursor();

    let input: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Type confirmation")
        .allow_empty(true)
        .interact_on(&term)
        .unwrap_or_default();

    input.trim() == expected
}

/// Display configuration in a table
pub fn display_config(
    default_days: u32,
    protected_branches: &[String],
    exclude_patterns: &[String],
    default_branch: Option<&str>,
    config_path: &str,
) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("Section").add_attribute(Attribute::Bold),
        Cell::new("Setting").add_attribute(Attribute::Bold),
        Cell::new("Value").add_attribute(Attribute::Bold),
    ]);

    // General section
    table.add_row(vec![
        Cell::new("general").fg(Color::Yellow),
        Cell::new("default_days"),
        Cell::new(default_days.to_string()).fg(Color::Cyan),
    ]);

    // Branches section
    table.add_row(vec![
        Cell::new("branches").fg(Color::Yellow),
        Cell::new("default_branch"),
        Cell::new(default_branch.unwrap_or("(auto-detect)")).fg(Color::Cyan),
    ]);

    let protected_display = if protected_branches.is_empty() {
        "(none)".to_string()
    } else {
        protected_branches.join(", ")
    };
    table.add_row(vec![
        Cell::new("branches").fg(Color::Yellow),
        Cell::new("protected"),
        Cell::new(protected_display).fg(Color::Cyan),
    ]);

    let exclude_display = if exclude_patterns.is_empty() {
        "(none)".to_string()
    } else {
        exclude_patterns.join(", ")
    };
    table.add_row(vec![
        Cell::new("branches").fg(Color::Yellow),
        Cell::new("exclude_patterns"),
        Cell::new(exclude_display).fg(Color::Cyan),
    ]);

    println!("\n{}", style("Configuration:").bold());
    println!("{table}");
    println!(
        "{} {}",
        style("Config file:").dim(),
        style(config_path).dim()
    );
    println!();
}

/// Display backups for a single repository
pub fn display_repo_backups(repo_name: &str, backups: &[BackupInfo]) {
    if backups.is_empty() {
        println!(
            "{}",
            style(format!("No backups found for '{}'.", repo_name)).dim()
        );
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("#").add_attribute(Attribute::Bold),
        Cell::new("Backup").add_attribute(Attribute::Bold),
        Cell::new("Age").add_attribute(Attribute::Bold),
        Cell::new("Branches").add_attribute(Attribute::Bold),
    ]);

    for (i, backup) in backups.iter().enumerate() {
        table.add_row(vec![
            Cell::new((i + 1).to_string()).fg(Color::DarkGrey),
            Cell::new(backup.filename()),
            Cell::new(backup.format_age()).fg(Color::Cyan),
            Cell::new(backup.branch_count.to_string()).fg(Color::Yellow),
        ]);
    }

    println!(
        "\n{}",
        style(format!("Backups for '{}':", repo_name)).bold()
    );
    println!("{table}");

    // Show restore hint
    println!();
    println!("{}", style("To restore a branch:").dim());
    println!(
        "  {}",
        style("deadbranch backup restore <branch-name>").dim()
    );
    println!(
        "  {}",
        style("deadbranch backup restore <branch-name> --from <backup-file>").dim()
    );
    println!();
}

/// Display all backups as a summary grouped by repository
pub fn display_all_backups(all_backups: &HashMap<String, Vec<BackupInfo>>) {
    if all_backups.is_empty() {
        println!("{}", style("No backups found.").dim());
        return;
    }

    // Sort repositories alphabetically
    let mut repos: Vec<_> = all_backups.keys().collect();
    repos.sort();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("#").add_attribute(Attribute::Bold),
        Cell::new("Repository").add_attribute(Attribute::Bold),
        Cell::new("Backups").add_attribute(Attribute::Bold),
        Cell::new("Latest").add_attribute(Attribute::Bold),
        Cell::new("Oldest").add_attribute(Attribute::Bold),
    ]);

    let mut total_backups = 0;

    for (i, repo_name) in repos.iter().enumerate() {
        let backups = &all_backups[*repo_name];
        total_backups += backups.len();

        // Backups are already sorted newest first
        let latest_age = backups.first().map(|b| b.format_age()).unwrap_or_default();
        let oldest_age = backups.last().map(|b| b.format_age()).unwrap_or_default();

        table.add_row(vec![
            Cell::new((i + 1).to_string()).fg(Color::DarkGrey),
            Cell::new(repo_name.as_str()).fg(Color::Yellow),
            Cell::new(backups.len().to_string()).fg(Color::Yellow),
            Cell::new(latest_age).fg(Color::Cyan),
            Cell::new(oldest_age).fg(Color::DarkGrey),
        ]);
    }

    println!("\n{}", style("All backups:").bold());
    println!("{table}");

    // Summary
    println!(
        "\n{} {} {} across {} {}",
        style("Total:").dim(),
        style(total_backups).cyan(),
        pluralize(total_backups, "backup", "backups"),
        style(repos.len()).cyan(),
        pluralize(repos.len(), "repository", "repositories")
    );

    // Hint
    println!();
    println!("{}", style("To see details for a repository:").dim());
    println!("  {}", style("deadbranch backup list --repo <name>").dim());
    println!(
        "  {}",
        style("deadbranch backup list --current  (for current repo)").dim()
    );
    println!();
}

/// Display restore success message
pub fn display_restore_success(result: &RestoreResult) {
    let short_sha = &result.commit_sha[..8.min(result.commit_sha.len())];
    let renamed = result.original_name != result.restored_name;
    let overwrote = result.overwrote_existing;

    let suffix = if overwrote {
        format!(" {}", style("(overwrote existing)").dim())
    } else {
        String::new()
    };

    if renamed {
        // Restored with different name (--as flag)
        println!(
            "{} Restored branch '{}' as '{}' at commit {}{}",
            style("✅").green().bold(),
            style(&result.original_name).cyan(),
            style(&result.restored_name).cyan().bold(),
            style(short_sha).yellow(),
            suffix
        );
    } else {
        // Normal restore (same name)
        println!(
            "{} Restored branch '{}' at commit {}{}",
            style("✅").green().bold(),
            style(&result.restored_name).cyan().bold(),
            style(short_sha).yellow(),
            suffix
        );
    }
}

/// Display restore error with helpful suggestions
pub fn display_restore_error(err: &RestoreError, branch_name: &str) {
    match err {
        RestoreError::BranchExists { branch_name } => {
            error(&format!("Branch '{}' already exists", branch_name));
            println!();
            println!("To overwrite it, use {}:", style("--force").yellow());
            println!(
                "  {}",
                style(format!("deadbranch backup restore {} --force", branch_name)).dim()
            );
            println!();
            println!("To restore with a different name:");
            println!(
                "  {}",
                style(format!(
                    "deadbranch backup restore {} --as {}-restored",
                    branch_name, branch_name
                ))
                .dim()
            );
        }

        RestoreError::CommitNotFound {
            branch_name,
            commit_sha,
        } => {
            let short_sha = &commit_sha[..8.min(commit_sha.len())];
            error(&format!(
                "Cannot restore '{}': commit {} no longer exists",
                branch_name, short_sha
            ));
            println!("  {}", style("(Git may have garbage collected it)").dim());
            println!();
            println!(
                "{}",
                style("Tip: Try restoring from an older backup with --from").dim()
            );
            println!(
                "     {}",
                style("Run 'git fsck --unreachable' to check for dangling commits").dim()
            );
        }

        RestoreError::BranchNotInBackup {
            branch_name: _,
            available_branches,
            skipped_lines,
        } => {
            error(&format!("Branch '{}' not found in backup", branch_name));
            println!();

            // Show warning about skipped/corrupted lines first
            if !skipped_lines.is_empty() {
                display_skipped_lines(skipped_lines);
            }

            if !available_branches.is_empty() {
                display_available_branches(available_branches);
            } else if !skipped_lines.is_empty() {
                // No valid entries and we have skipped lines - the backup might be corrupted
                println!(
                    "{}",
                    style("No valid branch entries found in backup.").yellow()
                );
                println!();
                println!(
                    "{}",
                    style("The backup file may be corrupted. Try a different backup:").dim()
                );
                println!("  {}", style("deadbranch backup list --current").dim());
            }
        }

        RestoreError::NoBackupsFound { repo_name } => {
            error(&format!("No backups found for repository '{}'", repo_name));
            println!();
            println!(
                "  {} Backups are created automatically when running 'deadbranch clean'.",
                style("↪").dim()
            );
        }

        RestoreError::BackupCorrupted { message } => {
            error("Backup file is corrupted or invalid format");
            println!("  {}", style(message).dim());
            println!();
            println!("Try a different backup:");
            println!("  {}", style("deadbranch backup list --current").dim());
        }

        RestoreError::Other(e) => {
            error(&format!("Failed to restore branch: {}", e));
        }
    }
}

/// Display available branches in a table format
fn display_available_branches(branches: &[BackupBranchEntry]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("Branch").add_attribute(Attribute::Bold),
        Cell::new("Commit").add_attribute(Attribute::Bold),
    ]);

    // Show up to 10 branches
    let display_count = branches.len().min(10);
    for entry in branches.iter().take(display_count) {
        let short_sha = &entry.commit_sha[..8.min(entry.commit_sha.len())];
        table.add_row(vec![
            Cell::new(&entry.name).fg(Color::Cyan),
            Cell::new(short_sha).fg(Color::Yellow),
        ]);
    }

    println!(
        "{}",
        style(format!(
            "Available {} in this backup:",
            pluralize_branch(branches.len())
        ))
        .dim()
    );
    println!("{table}");

    if branches.len() > 10 {
        println!(
            "  {} ... and {} more",
            style("↪").dim(),
            branches.len() - 10
        );
    }
    println!();
}

/// Display warning about skipped/corrupted lines in backup file
fn display_skipped_lines(skipped: &[SkippedLine]) {
    let count = skipped.len();
    let line_word = pluralize(count, "line", "lines");

    println!(
        "{} {} {} in backup file:",
        style("⚠️").yellow().bold(),
        style(format!("{} corrupted", count)).yellow(),
        line_word
    );

    // Show up to 3 skipped lines as examples
    for line in skipped.iter().take(3) {
        // Truncate long lines for display
        let display_content = if line.content.len() > 60 {
            format!("{}...", &line.content[..57])
        } else {
            line.content.clone()
        };
        println!(
            "  {} Line {}: {}",
            style("→").dim(),
            style(line.line_number).yellow(),
            style(display_content).dim()
        );
    }

    if count > 3 {
        println!("  {} ... and {} more", style("→").dim(), count - 3);
    }
    println!();
}

/// Display backups that will be deleted in a table format
pub fn display_backups_to_clean(
    repo_name: &str,
    backups: &[BackupToDelete],
    keep: usize,
    _dry_run: bool,
) {
    println!(
        "Cleaning backups for '{}' (keeping {} most recent)...\n",
        style(repo_name).cyan(),
        keep
    );

    if backups.is_empty() {
        println!("  {} No old backups to clean\n", style("ℹ️").blue());
        return;
    }

    println!("{}", style("Backups to Delete:").bold());

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("Backup").add_attribute(Attribute::Bold),
        Cell::new("Age").add_attribute(Attribute::Bold),
        Cell::new("Branches").add_attribute(Attribute::Bold),
        Cell::new("Size").add_attribute(Attribute::Bold),
    ]);

    for backup in backups {
        table.add_row(vec![
            Cell::new(backup.info.filename()),
            Cell::new(backup.info.format_age()).fg(Color::DarkGrey),
            Cell::new(backup.info.branch_count.to_string()),
            Cell::new(backup.format_size()).fg(Color::DarkGrey),
        ]);
    }

    println!("{table}\n");
}

/// Ask for confirmation to delete backups
pub fn confirm_backup_clean(count: usize, total_size: u64) -> bool {
    let file_word = pluralize(count, "backup", "backups");
    let prompt = format!(
        "Delete {} {} ({})?",
        count,
        file_word,
        format_bytes(total_size)
    );
    confirm(&prompt, false)
}

/// Display cleanup success message
pub fn display_backup_clean_success(result: &CleanResult) {
    let file_word = pluralize(result.deleted_count, "backup", "backups");
    println!(
        "{} Deleted {} {} (freed {})",
        style("✅").green().bold(),
        style(result.deleted_count).cyan(),
        file_word,
        style(format_bytes(result.bytes_freed)).cyan()
    );
}

/// Display cleanup dry-run header and footer (styled like branch clean)
pub fn display_backup_clean_dry_run(count: usize, total_size: u64) {
    let file_word = pluralize(count, "backup", "backups");
    println!(
        "{}",
        style("[DRY RUN] No backups will be deleted.")
            .yellow()
            .bold()
    );
    println!();
    println!(
        "{} Would delete {} {} ({})",
        style("ℹ️").blue(),
        style(count).cyan(),
        file_word,
        style(format_bytes(total_size)).cyan()
    );
}

/// Display message when no backups found for cleanup
pub fn display_no_backups_for_repo(repo_name: &str) {
    println!(
        "{} No backups found for repository '{}'",
        style("ℹ️").blue(),
        repo_name
    );
}

/// Display backup storage statistics in a table
pub fn display_backup_stats(stats: &BackupStats) {
    if stats.repos.is_empty() {
        info("No backups found.");
        println!();
        println!(
            "  {} Backups are created automatically when running 'deadbranch clean'.",
            style("↪").dim()
        );
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("#").add_attribute(Attribute::Bold),
        Cell::new("Repository").add_attribute(Attribute::Bold),
        Cell::new("Backups").add_attribute(Attribute::Bold),
        Cell::new("Size").add_attribute(Attribute::Bold),
    ]);

    for (i, repo) in stats.repos.iter().enumerate() {
        table.add_row(vec![
            Cell::new((i + 1).to_string()).fg(Color::DarkGrey),
            Cell::new(&repo.repo_name).fg(Color::Yellow),
            Cell::new(repo.backup_count.to_string()).fg(Color::Cyan),
            Cell::new(format_bytes(repo.total_bytes)).fg(Color::DarkGrey),
        ]);
    }

    println!("\n{}", style("Backup storage statistics:").bold());
    println!(
        "{} {}",
        style("Location:").dim(),
        style(stats.backups_dir.display()).dim()
    );
    println!("{table}");

    println!(
        "{} {} {}, {}",
        style("Total:").dim(),
        style(stats.total_backups()).cyan(),
        pluralize(stats.total_backups(), "backup", "backups"),
        style(format_bytes(stats.total_bytes())).cyan()
    );
    println!();
}

/// Display repository branch statistics and age distribution in tables
pub fn display_repo_stats(stats: &RepoStats) {
    if stats.total == 0 {
        info("No branches found.");
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        Cell::new("Category").add_attribute(Attribute::Bold),
        Cell::new("Total").add_attribute(Attribute::Bold),
        Cell::new("Local").add_attribute(Attribute::Bold),
        Cell::new("Remote").add_attribute(Attribute::Bold),
    ]);

    table.add_row(vec![
        Cell::new("All branches"),
        Cell::new(stats.total.to_string()),
        Cell::new(stats.local.to_string()),
        Cell::new(stats.remote.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Merged"),
        Cell::new(stats.merged.to_string()).fg(Color::Green),
        Cell::new(stats.merged_local.to_string()).fg(Color::Green),
        Cell::new(stats.merged_remote.to_string()).fg(Color::Green),
    ]);
    table.add_row(vec![
        Cell::new("Unmerged"),
        Cell::new(stats.unmerged.to_string()).fg(Color::Yellow),
        Cell::new(stats.unmerged_local.to_string()).fg(Color::Yellow),
        Cell::new(stats.unmerged_remote.to_string()).fg(Color::Yellow),
    ]);
    table.add_row(vec![
        Cell::new(format!("Stale (>{}d)", stats.threshold_days)),
        Cell::new(stats.stale.to_string()).fg(Color::DarkGrey),
        Cell::new(stats.stale_local.to_string()).fg(Color::DarkGrey),
        Cell::new(stats.stale_remote.to_string()).fg(Color::DarkGrey),
    ]);
    table.add_row(vec![
        Cell::new("Safe to delete"),
        Cell::new(stats.safe_to_delete.to_string()).fg(Color::Cyan),
        Cell::new(stats.safe_local.to_string()).fg(Color::Cyan),
        Cell::new(stats.safe_remote.to_string()).fg(Color::Cyan),
    ]);

    println!("\n{}", style("Repository Statistics:").bold());
    println!("{table}");

    let mut age_table = Table::new();
    age_table.load_preset(UTF8_FULL);
    age_table.set_header(vec![
        Cell::new("Age Range").add_attribute(Attribute::Bold),
        Cell::new("Count").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
    ]);

    // A bucket is labeled "stale" if its minimum age >= threshold_days,
    // meaning every branch in it would be considered stale.
    let buckets: &[(&str, usize, u32)] = &[
        ("< 7 days", stats.age_lt7, 0),
        ("7\u{2013}30 days", stats.age_7_30, 7),
        ("30\u{2013}90 days", stats.age_30_90, 30),
        ("> 90 days", stats.age_gt90, 90),
    ];

    for (label, count, bucket_min) in buckets {
        let is_stale = *bucket_min >= stats.threshold_days;
        let status_cell = if is_stale {
            Cell::new("stale").fg(Color::Yellow)
        } else {
            Cell::new("fresh").fg(Color::Green)
        };
        age_table.add_row(vec![
            Cell::new(*label),
            Cell::new(count.to_string()),
            status_cell,
        ]);
    }

    println!("\n{}", style("Age Distribution:").bold());
    println!("{age_table}");

    if stats.safe_to_delete > 0 {
        println!();
        println!(
            "{} Run '{}' to remove {} safe-to-delete {}",
            style("💡"),
            style("deadbranch clean").cyan(),
            style(stats.safe_to_delete).cyan(),
            pluralize_branch(stats.safe_to_delete)
        );
    }

    println!();
}
