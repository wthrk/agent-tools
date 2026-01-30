use clap::{Parser, Subcommand};

mod commands;
mod config;
mod fs_utils;
mod paths;
mod project;
mod skill_meta;

#[derive(Parser)]
#[command(name = "skill-tools")]
#[command(
    version,
    about = "CLI tool for managing Claude Code skills and settings"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize skill-tools (create directories, setup PATH)
    Init,

    /// Update skill-tools (git pull && cargo build)
    Update,

    /// Show current status (links, config validation)
    Status,

    /// Sync ~/.claude based on config.yaml
    Sync {
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,

        /// Remove links for skills not in config
        #[arg(long)]
        prune: bool,
    },

    /// Link a skill to ~/.claude/skills/
    Link {
        /// Skill name to link
        name: String,
    },

    /// Unlink a skill from ~/.claude/skills/
    Unlink {
        /// Skill name to unlink
        name: String,
    },

    /// Manage project skills
    Skill {
        #[command(subcommand)]
        command: SkillCommands,
    },

    /// Clean up old backups
    Cleanup,
}

#[derive(Subcommand)]
enum SkillCommands {
    /// List available skills (global)
    List,

    /// Install a skill to current project
    Install {
        /// Skill name to install
        name: String,

        /// Project path (default: auto-detect)
        #[arg(long)]
        project: Option<String>,
    },

    /// Update a skill in current project
    Update {
        /// Skill name to update (or --all)
        name: Option<String>,

        /// Update all installed skills
        #[arg(long)]
        all: bool,

        /// Force update, overwriting local changes
        #[arg(long)]
        force: bool,

        /// Project path (default: auto-detect)
        #[arg(long)]
        project: Option<String>,
    },

    /// Remove a skill from current project
    Remove {
        /// Skill name to remove
        name: String,

        /// Project path (default: auto-detect)
        #[arg(long)]
        project: Option<String>,
    },

    /// List installed skills in current project
    Installed {
        /// Project path (default: auto-detect)
        #[arg(long)]
        project: Option<String>,
    },

    /// Show diff between installed and source skill
    Diff {
        /// Skill name to diff
        name: String,

        /// Project path (default: auto-detect)
        #[arg(long)]
        project: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    // Check OS (only macOS and Linux supported)
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        anyhow::bail!("skill-tools only supports macOS and Linux");
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Update => commands::update::run(),
        Commands::Status => commands::status::run(),
        Commands::Sync { dry_run, prune } => commands::sync::run(dry_run, prune),
        Commands::Link { name } => commands::link::run(&name),
        Commands::Unlink { name } => commands::unlink::run(&name),
        Commands::Skill { command } => match command {
            SkillCommands::List => commands::skill::list::run(),
            SkillCommands::Install { name, project } => {
                commands::skill::install::run(&name, project.as_deref())
            }
            SkillCommands::Update {
                name,
                all,
                force,
                project,
            } => commands::skill::update::run(name.as_deref(), all, force, project.as_deref()),
            SkillCommands::Remove { name, project } => {
                commands::skill::remove::run(&name, project.as_deref())
            }
            SkillCommands::Installed { project } => {
                commands::skill::installed::run(project.as_deref())
            }
            SkillCommands::Diff { name, project } => {
                commands::skill::diff::run(&name, project.as_deref())
            }
        },
        Commands::Cleanup => commands::cleanup::run(),
    }
}
