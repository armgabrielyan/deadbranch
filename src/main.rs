//! deadbranch - Clean up stale git branches safely

mod backup;
mod branch;
mod cli;
mod config;
mod error;
mod git;
mod stats;
mod tui;
mod ui;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use std::fs;
use std::io::Write;

use branch::BranchFilter;
use cli::{BackupAction, Cli, Commands, ConfigAction};
use config::Config;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check if we're in a git repository (except for config, backup, and completions commands)
    if !matches!(
        cli.command,
        Commands::Config { .. } | Commands::Backup { .. } | Commands::Completions { .. }
    ) && !git::is_git_repository()
    {
        ui::error("Not a git repository (or any parent up to mount point)");
        std::process::exit(1);
    }

    match cli.command {
        Commands::List {
            days,
            local,
            remote,
            merged,
        } => cmd_list(days, local, remote, merged),

        Commands::Clean {
            days,
            merged,
            force,
            dry_run,
            local,
            remote,
            yes,
            interactive,
        } => cmd_clean(
            days,
            merged,
            force,
            dry_run,
            local,
            remote,
            yes,
            interactive,
        ),

        Commands::Config { action } => cmd_config(action),

        Commands::Backup { action } => cmd_backup(action),

        Commands::Stats { days } => cmd_stats(days),

        Commands::Completions { shell } => {
            generate(
                shell,
                &mut Cli::command(),
                "deadbranch",
                &mut std::io::stdout(),
            );
            Ok(())
        }
    }
}

/// List stale branches
fn cmd_list(
    days: Option<u32>,
    local_only: bool,
    remote_only: bool,
    merged_only: bool,
) -> Result<()> {
    let config = Config::load()?;

    // Use CLI value if provided, otherwise use config default
    let min_age = days.unwrap_or(config.general.default_days);

    // Get default branch for merge detection
    let default_branch = config
        .branches
        .default_branch
        .clone()
        .unwrap_or_else(|| git::get_default_branch().unwrap_or_else(|_| "main".to_string()));

    ui::info(&format!(
        "Using '{}' as the default branch for merge detection",
        default_branch
    ));

    // List all branches
    let spinner = ui::spinner("Loading branches...");
    let all_branches = git::list_branches(&default_branch)?;
    spinner.finish_and_clear();

    // Filter branches
    let filter = BranchFilter {
        min_age_days: min_age,
        local_only,
        remote_only,
        merged_only,
        protected_branches: config.branches.protected,
        exclude_patterns: config.branches.exclude_patterns,
    };

    let mut branches: Vec<_> = all_branches
        .into_iter()
        .filter(|b| filter.matches(b))
        .collect();

    // Sort: unmerged first, then by age (oldest first)
    branch::sort_branches(&mut branches);

    // Separate local and remote for grouped display
    let mut local: Vec<_> = branches.iter().filter(|b| !b.is_remote).cloned().collect();
    let mut remote: Vec<_> = branches.iter().filter(|b| b.is_remote).cloned().collect();

    // Sort each group separately
    branch::sort_branches(&mut local);
    branch::sort_branches(&mut remote);

    // Display in table format
    if !local.is_empty() {
        ui::display_branches(&local, "Local Branches:");
    }
    if !remote.is_empty() {
        ui::display_branches(&remote, "Remote Branches:");
    }
    if local.is_empty() && remote.is_empty() {
        ui::info("No stale branches found.");
    }

    Ok(())
}

/// Clean (delete) stale branches
#[allow(clippy::too_many_arguments)]
fn cmd_clean(
    days: Option<u32>,
    merged: bool,
    force: bool,
    dry_run: bool,
    local_only: bool,
    remote_only: bool,
    skip_confirm: bool,
    interactive: bool,
) -> Result<()> {
    let config = Config::load()?;

    // Use CLI value if provided, otherwise use config default
    let min_age = days.unwrap_or(config.general.default_days);

    // Get default branch for merge detection
    let default_branch = config
        .branches
        .default_branch
        .clone()
        .unwrap_or_else(|| git::get_default_branch().unwrap_or_else(|_| "main".to_string()));

    // List all branches
    let spinner = ui::spinner("Loading branches...");
    let all_branches = git::list_branches(&default_branch)?;
    spinner.finish_and_clear();

    if interactive {
        // For TUI, apply only age + protection + exclusion filters.
        // merged/local/remote become initial toggle state in the TUI.
        let tui_filter = BranchFilter {
            min_age_days: min_age,
            local_only: false,
            remote_only: false,
            merged_only: false,
            protected_branches: config.branches.protected.clone(),
            exclude_patterns: config.branches.exclude_patterns.clone(),
        };

        let tui_branches: Vec<_> = all_branches
            .into_iter()
            .filter(|b| tui_filter.matches(b))
            .collect();

        if tui_branches.is_empty() {
            ui::info("No branches to show in interactive mode.");
            return Ok(());
        }

        // Build initial filter state from CLI flags for toggle seeding
        let initial_filter = BranchFilter {
            min_age_days: 0,
            local_only,
            remote_only,
            merged_only: merged,
            protected_branches: Vec::new(),
            exclude_patterns: Vec::new(),
        };

        return tui::run_interactive(tui_branches, &initial_filter, &default_branch, force);
    }

    // By default, only delete merged branches unless --force is used
    let merged_only = merged || !force;

    // Create filter - by default, show both local and remote branches
    // Use --local or --remote to filter to only one type
    let filter = BranchFilter {
        min_age_days: min_age,
        local_only,
        remote_only,
        merged_only,
        protected_branches: config.branches.protected.clone(),
        exclude_patterns: config.branches.exclude_patterns,
    };

    // Filter branches
    let mut branches: Vec<_> = all_branches
        .into_iter()
        .filter(|b| filter.matches(b))
        .collect();

    // Sort: unmerged first, then by age (oldest first)
    branch::sort_branches(&mut branches);

    if branches.is_empty() {
        ui::info("No branches to delete.");
        return Ok(());
    }

    // Separate local and remote
    let mut local_branches: Vec<_> = branches.iter().filter(|b| !b.is_remote).cloned().collect();
    let mut remote_branches: Vec<_> = branches.iter().filter(|b| b.is_remote).cloned().collect();

    // Sort each group separately
    branch::sort_branches(&mut local_branches);
    branch::sort_branches(&mut remote_branches);

    if dry_run {
        // For dry-run, show all tables upfront
        if !local_branches.is_empty() {
            let title = format!(
                "Local {} to Delete:",
                ui::pluralize_branch_cap(local_branches.len())
            );
            ui::display_branches(&local_branches, &title);
        }
        if !remote_branches.is_empty() {
            let title = format!(
                "Remote {} to Delete:",
                ui::pluralize_branch_cap(remote_branches.len())
            );
            ui::display_branches(&remote_branches, &title);
        }

        // Count by operation type
        let local_safe: usize = local_branches
            .iter()
            .filter(|b| force || b.is_merged)
            .count();
        let local_force: usize = local_branches.len() - local_safe;
        let remote_count: usize = remote_branches.len();
        let total = local_branches.len() + remote_count;

        ui::print_dry_run_summary(total, local_safe, local_force, remote_count);
        return Ok(());
    }

    // Handle local branches - show table right before confirmation
    if !local_branches.is_empty() {
        let title = format!(
            "Local {} to Delete:",
            ui::pluralize_branch_cap(local_branches.len())
        );
        ui::display_branches(&local_branches, &title);

        if skip_confirm || ui::confirm_local_deletion(&local_branches) {
            delete_branches_with_backup(&local_branches, force)?;
        } else {
            println!();
            ui::info("Skipped local branch deletion.");
        }
    }

    // Handle remote branches - show table as part of the warning
    if !remote_branches.is_empty() {
        // Add visual separation if we just handled local branches
        if !local_branches.is_empty() {
            println!();
            println!("{}", console::style("─".repeat(50)).dim());
            println!();
        }

        // First, fetch and prune to ensure we have accurate data
        let spinner = ui::spinner("Fetching remote to ensure data is up to date...");
        match git::fetch_and_prune() {
            Ok(()) => ui::spinner_success(&spinner, "Remote data is up to date"),
            Err(e) => {
                ui::spinner_warn(&spinner, "Could not fetch remote");
                ui::warning(&format!("  {}", e));
                ui::warning("  Remote branch data may be stale.");
            }
        }

        // Show table and get confirmation
        let title = format!(
            "Remote {} to Delete:",
            ui::pluralize_branch_cap(remote_branches.len())
        );
        ui::display_branches(&remote_branches, &title);

        if skip_confirm || ui::confirm_remote_deletion(&remote_branches) {
            delete_remote_branches_with_backup(&remote_branches)?;
        } else {
            println!();
            ui::info("Skipped remote branch deletion.");
        }
    }

    Ok(())
}

/// Delete local branches and create backup file
pub(crate) fn delete_branches_with_backup(branches: &[branch::Branch], force: bool) -> Result<()> {
    let backup = create_backup_file(branches)?;
    let branch_word = ui::pluralize_branch(branches.len());

    // Visual separation after confirmation
    println!();
    println!("Deleting local {}...", branch_word);

    let mut deleted = 0;
    let mut failed = 0;

    for branch in branches {
        match git::delete_local_branch(&branch.name, force) {
            Ok(()) => {
                println!("  {} {}", console::style("✅").green(), branch.name);
                deleted += 1;
            }
            Err(e) => {
                println!("  {} {} ({})", console::style("❌").red(), branch.name, e);
                failed += 1;
            }
        }
    }

    // Summary footer
    println!();
    let branch_word = ui::pluralize_branch(deleted);
    if failed == 0 {
        ui::success(&format!("Deleted {} local {}", deleted, branch_word));
    } else {
        ui::warning(&format!(
            "Deleted {} local {}, {} failed",
            deleted, branch_word, failed
        ));
    }
    println!(
        "  {} Backup: {}",
        console::style("↪").dim(),
        console::style(&backup).dim()
    );

    Ok(())
}

/// Delete remote branches and create backup file.
/// Uses batch `git push origin --delete` for a single network round-trip.
pub(crate) fn delete_remote_branches_with_backup(branches: &[branch::Branch]) -> Result<()> {
    let backup = create_backup_file(branches)?;
    let branch_word = ui::pluralize_branch(branches.len());

    // Visual separation after confirmation
    println!();
    println!("Deleting remote {}...", branch_word);

    let names: Vec<String> = branches.iter().map(|b| b.name.clone()).collect();
    let results = git::delete_remote_branches_batch(&names)?;

    let mut deleted = 0;
    let mut failed = 0;

    for (name, success, error) in &results {
        if *success {
            println!("  {} {}", console::style("✅").green(), name);
            deleted += 1;
        } else {
            let err_msg = error.as_deref().unwrap_or("unknown error");
            println!("  {} {} ({})", console::style("❌").red(), name, err_msg);
            failed += 1;
        }
    }

    // Summary footer
    println!();
    let branch_word = ui::pluralize_branch(deleted);
    if failed == 0 {
        ui::success(&format!("Deleted {} remote {}", deleted, branch_word));
    } else {
        ui::warning(&format!(
            "Deleted {} remote {}, {} failed",
            deleted, branch_word, failed
        ));
    }
    println!(
        "  {} Backup: {}",
        console::style("↪").dim(),
        console::style(&backup).dim()
    );

    Ok(())
}

/// Create a backup file with branch SHAs for potential restoration
/// Saves to ~/.deadbranch/backups/<repo-name>/backup-<timestamp>.txt
pub(crate) fn create_backup_file(branches: &[branch::Branch]) -> Result<String> {
    let repo_name = Config::get_repo_name();
    let backup_dir = Config::repo_backup_dir(&repo_name)?;

    // Create backup directory if it doesn't exist
    fs::create_dir_all(&backup_dir)?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("backup-{}.txt", timestamp);
    let backup_path = backup_dir.join(&filename);

    let mut file = fs::File::create(&backup_path)?;

    writeln!(file, "# deadbranch backup")?;
    writeln!(file, "# Created: {}", Utc::now().to_rfc3339())?;
    writeln!(file, "# Repository: {}", repo_name)?;
    writeln!(
        file,
        "# Working directory: {}",
        std::env::current_dir()?.display()
    )?;
    writeln!(file, "#")?;
    writeln!(file, "# To restore a branch, run the git command shown")?;
    writeln!(file, "#")?;
    writeln!(file)?;

    for branch in branches {
        let sha =
            git::get_branch_sha(&branch.name).unwrap_or_else(|_| branch.last_commit_sha.clone());
        let restore_name = if branch.is_remote {
            branch.name.strip_prefix("origin/").unwrap_or(&branch.name)
        } else {
            &branch.name
        };
        writeln!(file, "# {}", branch.name)?;
        writeln!(file, "git branch {} {}", restore_name, sha)?;
        writeln!(file)?;
    }

    Ok(backup_path.display().to_string())
}

/// Handle config subcommands
fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let config = Config::load()?;
            let config_path = Config::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "(unknown)".to_string());

            ui::display_config(
                config.general.default_days,
                &config.branches.protected,
                &config.branches.exclude_patterns,
                config.branches.default_branch.as_deref(),
                &config_path,
            );
        }

        ConfigAction::Set { key, values } => {
            let mut config = Config::load()?;
            config.set(&key, &values)?;
            config.save()?;

            // Format display based on single value or list
            let display_value = if values.len() == 1 {
                values[0].clone()
            } else {
                values.join(", ")
            };
            ui::success(&format!("Set {} = {}", key, display_value));
        }

        ConfigAction::Edit => {
            // Ensure config file exists
            let _ = Config::load()?;
            let config_path = Config::config_path()?;

            // Get editor from $EDITOR or $VISUAL, fallback to common editors
            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| {
                    // Try common editors
                    if which::which("nano").is_ok() {
                        "nano".to_string()
                    } else if which::which("vim").is_ok() {
                        "vim".to_string()
                    } else if which::which("vi").is_ok() {
                        "vi".to_string()
                    } else {
                        "nano".to_string() // Default fallback
                    }
                });

            ui::info(&format!(
                "Opening {} in {}...",
                config_path.display(),
                editor
            ));

            let status = std::process::Command::new(&editor)
                .arg(&config_path)
                .status()
                .with_context(|| format!("Failed to open editor: {}", editor))?;

            if status.success() {
                ui::success("Config file saved");
            } else {
                ui::warning("Editor exited with non-zero status");
            }
        }

        ConfigAction::Reset => {
            if ui::confirm("Reset configuration to defaults?", false) {
                let config = Config::default();
                config.save()?;
                ui::success("Configuration reset to defaults");
            } else {
                ui::info("Cancelled");
            }
        }
    }

    Ok(())
}

/// Show repository branch statistics
fn cmd_stats(days: Option<u32>) -> Result<()> {
    let config = Config::load()?;
    let min_age = days.unwrap_or(config.general.default_days);

    let default_branch = config
        .branches
        .default_branch
        .clone()
        .unwrap_or_else(|| git::get_default_branch().unwrap_or_else(|_| "main".to_string()));

    ui::info(&format!(
        "Using '{}' as the default branch for merge detection",
        default_branch
    ));

    let spinner = ui::spinner("Loading branches...");
    let all_branches = git::list_branches(&default_branch)?;
    spinner.finish_and_clear();

    // Apply the same visibility rules as list/clean: respect protected and
    // exclude_patterns, but no age filter — stats covers all visible branches.
    let filter = BranchFilter {
        min_age_days: 0,
        local_only: false,
        remote_only: false,
        merged_only: false,
        protected_branches: config.branches.protected,
        exclude_patterns: config.branches.exclude_patterns,
    };

    let branches: Vec<_> = all_branches
        .into_iter()
        .filter(|b| filter.matches(b))
        .collect();

    let repo_stats = stats::compute_stats(&branches, min_age);
    ui::display_repo_stats(&repo_stats);

    Ok(())
}

/// Handle backup subcommands
fn cmd_backup(action: BackupAction) -> Result<()> {
    match action {
        BackupAction::List { current, repo } => {
            // Determine which repo to show (if any specific one)
            let target_repo = if current {
                // Check if we're in a git repo for --current
                if !git::is_git_repository() {
                    ui::error("Not a git repository (or any parent up to mount point)");
                    ui::info("Use 'deadbranch backup list' without --current to see all backups.");
                    std::process::exit(1);
                }
                Some(Config::get_repo_name())
            } else {
                repo
            };

            if let Some(repo_name) = target_repo {
                // Show detailed view for specific repo
                let backups = backup::list_repo_backups(&repo_name)?;

                if backups.is_empty() {
                    ui::info(&format!("No backups found for repository '{}'", repo_name));
                    println!();
                    println!(
                        "  {} Backups are created automatically when running 'deadbranch clean'.",
                        console::style("↪").dim()
                    );
                } else {
                    ui::display_repo_backups(&repo_name, &backups);
                }
            } else {
                // Show summary of all repos
                let all_backups = backup::list_all_backups()?;

                if all_backups.is_empty() {
                    ui::info("No backups found.");
                    println!();
                    println!(
                        "  {} Backups are created automatically when running 'deadbranch clean'.",
                        console::style("↪").dim()
                    );
                } else {
                    ui::display_all_backups(&all_backups);
                }
            }
        }

        BackupAction::Stats => {
            let stats = backup::get_backup_stats()?;
            ui::display_backup_stats(&stats);
        }

        BackupAction::Restore {
            branch,
            from,
            r#as,
            force,
        } => {
            // Restore requires being in a git repository
            if !git::is_git_repository() {
                ui::error("Not a git repository (or any parent up to mount point)");
                std::process::exit(1);
            }

            match backup::restore_branch(&branch, from.as_deref(), r#as.as_deref(), force) {
                Ok(result) => {
                    ui::display_restore_success(&result);
                }
                Err(e) => {
                    ui::display_restore_error(&e, &branch);
                    std::process::exit(1);
                }
            }
        }

        BackupAction::Clean {
            current,
            repo,
            keep,
            dry_run,
            yes,
        } => {
            // Determine target repo
            let repo_name = if current {
                if !git::is_git_repository() {
                    ui::error("Not a git repository (or any parent up to mount point)");
                    ui::info("Use --repo <name> to specify a repository by name.");
                    std::process::exit(1);
                }
                Config::get_repo_name()
            } else if let Some(name) = repo {
                name
            } else {
                ui::error("Either --current or --repo <name> is required");
                std::process::exit(1);
            };

            // Get backups to clean
            let backups_to_clean = backup::get_backups_to_clean(&repo_name, keep)?;

            // Check if there are any backups at all for this repo
            let all_backups = backup::list_repo_backups(&repo_name)?;
            if all_backups.is_empty() {
                ui::display_no_backups_for_repo(&repo_name);
                return Ok(());
            }

            // Display what will be deleted
            ui::display_backups_to_clean(&repo_name, &backups_to_clean, keep, dry_run);

            if backups_to_clean.is_empty() {
                return Ok(());
            }

            if dry_run {
                let total_size: u64 = backups_to_clean.iter().map(|b| b.size_bytes).sum();
                ui::display_backup_clean_dry_run(backups_to_clean.len(), total_size);
                return Ok(());
            }

            // Confirm deletion unless --yes was provided
            let total_size: u64 = backups_to_clean.iter().map(|b| b.size_bytes).sum();
            if !yes && !ui::confirm_backup_clean(backups_to_clean.len(), total_size) {
                ui::info("Cancelled");
                return Ok(());
            }

            // Perform deletion
            let result = backup::delete_backups(&backups_to_clean)?;
            ui::display_backup_clean_success(&result);
        }
    }

    Ok(())
}
