//! Interactive TUI mode for branch selection and deletion

mod app;

use anyhow::Result;

use crate::branch::{Branch, BranchFilter};
use crate::config::Config;

/// Run the interactive TUI for branch selection and deletion.
pub fn run_interactive(
    all_branches: Vec<Branch>,
    initial_filter: &BranchFilter,
    _config: &Config,
    default_branch: &str,
    force: bool,
) -> Result<()> {
    let _app = app::App::new(all_branches, initial_filter, default_branch, force);
    Ok(())
}
