//! Interactive TUI mode for branch selection and deletion

#[allow(dead_code)]
mod app;
mod event;
mod render;

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
    let mut app = app::App::new(all_branches, initial_filter, default_branch, force);
    event::run(&mut app)
}
