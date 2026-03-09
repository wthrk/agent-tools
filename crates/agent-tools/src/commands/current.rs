use anyhow::Result;
use colored::Colorize;

use crate::commands::profile::{ProfileState, load_state};
use crate::paths;

pub fn run() -> Result<()> {
    let state_dir = paths::profile_state_dir()?;
    let current = load_state(&state_dir.join("current.json"))?;
    let previous = load_state(&state_dir.join("previous.json"))?;

    println!("{}", "Profile state".bold());
    print_state("Current", &current);
    print_state("Previous", &previous);
    Ok(())
}

fn print_state(label: &str, state: &ProfileState) {
    let claude = state.claude.as_deref().unwrap_or("-");
    let codex = state.codex.as_deref().unwrap_or("-");
    let switched_at = state.switched_at.as_deref().unwrap_or("-");
    println!(
        "{} claude={} codex={} switched_at={}",
        format!("{label}:").cyan(),
        claude,
        codex,
        switched_at
    );
}
