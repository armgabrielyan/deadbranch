//! CLI argument definitions using clap

use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "deadbranch")]
#[command(author, version, about = "Clean up stale git branches safely", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List stale branches
    List {
        /// Only show branches older than N days (default: from config or 30)
        #[arg(short, long)]
        days: Option<u32>,

        /// Only show local branches
        #[arg(long)]
        local: bool,

        /// Only show remote branches
        #[arg(long, conflicts_with = "local")]
        remote: bool,

        /// Only show merged branches
        #[arg(long)]
        merged: bool,
    },

    /// Delete stale branches (merged only by default, use --force for unmerged)
    #[command(alias = "snap")]
    Clean {
        /// Only delete branches older than N days (default: from config or 30)
        #[arg(short, long)]
        days: Option<u32>,

        /// Only delete merged branches (this is the default behavior)
        #[arg(long)]
        merged: bool,

        /// Force delete unmerged branches (dangerous!)
        #[arg(long)]
        force: bool,

        /// Show what would be deleted without doing it
        #[arg(long, conflicts_with = "interactive")]
        dry_run: bool,

        /// Only delete local branches
        #[arg(long)]
        local: bool,

        /// Only delete remote branches
        #[arg(long, conflicts_with = "local")]
        remote: bool,

        /// Skip confirmation prompts (useful for scripts)
        #[arg(short, long, conflicts_with = "interactive")]
        yes: bool,

        /// Open interactive TUI for branch selection
        #[arg(short, long)]
        interactive: bool,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Manage backups
    Backup {
        #[command(subcommand)]
        action: BackupAction,
    },

    /// Show repository branch statistics
    Stats {
        /// Treat branches older than N days as stale (default: from config or 30)
        #[arg(short, long)]
        days: Option<u32>,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set a configuration value
    Set {
        /// Configuration key (e.g., default-days, protected-branches, default-branch, exclude-patterns)
        key: String,

        /// Configuration value(s) - use multiple arguments for lists
        /// Example: config set exclude-patterns "wip/*" "draft/*" "temp/*"
        #[arg(required = true, num_args = 1..)]
        values: Vec<String>,
    },

    /// Show current configuration
    Show,

    /// Open config file in $EDITOR
    Edit,

    /// Reset configuration to defaults
    Reset,
}

#[derive(Subcommand)]
pub enum BackupAction {
    /// List available backups
    List {
        /// Only show backups for current repository
        #[arg(long, conflicts_with = "repo")]
        current: bool,

        /// Show backups for a specific repository by name
        #[arg(long)]
        repo: Option<String>,
    },

    /// Restore a branch from backup
    Restore {
        /// Name of the branch to restore
        branch: String,

        /// Restore from a specific backup file (defaults to most recent)
        #[arg(long)]
        from: Option<String>,

        /// Restore with a different branch name
        #[arg(long, value_name = "NAME")]
        r#as: Option<String>,

        /// Overwrite existing branch if it exists
        #[arg(long)]
        force: bool,
    },

    /// Show backup storage statistics
    Stats,

    /// Remove old backups, keeping the most recent ones
    Clean {
        /// Clean backups for current repository
        #[arg(long, conflicts_with = "repo", required_unless_present = "repo")]
        current: bool,

        /// Clean backups for a specific repository by name
        #[arg(long, required_unless_present = "current")]
        repo: Option<String>,

        /// Number of most recent backups to keep (default: 10)
        #[arg(long, default_value = "10")]
        keep: usize,

        /// Show what would be deleted without doing it
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}
