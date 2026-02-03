use clap::{Parser, Subcommand};

mod commands;
mod config;
mod fs_utils;
mod paths;
mod project;
mod skill_meta;

#[derive(Parser)]
#[command(name = "agent-tools")]
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
    /// Build agent-tools and install to bin/
    Build,

    /// Initialize agent-tools (create directories, setup PATH)
    Init,

    /// Update agent-tools (git pull && cargo build)
    Update,

    /// Rebase current work onto latest main
    Rebase,

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
    /// Create a new skill
    New {
        /// Name of the skill to create
        name: String,

        /// Auto-confirm adding to auto_deploy_skills (skip prompt)
        #[arg(long, short = 'y')]
        yes: bool,

        /// Skip adding to auto_deploy_skills and linking
        #[arg(long)]
        no_auto_deploy: bool,
    },

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

    /// Validate a skill
    Validate {
        /// Path to skill directory (default: current directory)
        path: Option<String>,

        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,
    },
}

fn main() -> anyhow::Result<()> {
    // Check OS (only macOS and Linux supported)
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        anyhow::bail!("agent-tools only supports macOS and Linux");
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Build => commands::build::run(),
        Commands::Init => commands::init::run(),
        Commands::Update => commands::update::run(),
        Commands::Rebase => commands::rebase::run(),
        Commands::Status => commands::status::run(),
        Commands::Sync { dry_run, prune } => commands::sync::run(dry_run, prune),
        Commands::Link { name } => commands::link::run(&name),
        Commands::Unlink { name } => commands::unlink::run(&name),
        Commands::Skill { command } => match command {
            SkillCommands::New {
                name,
                yes,
                no_auto_deploy,
            } => {
                let add_to_config = if no_auto_deploy {
                    Some(false)
                } else if yes {
                    Some(true)
                } else {
                    None
                };
                commands::skill::new::run(&name, add_to_config)
            }
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
            SkillCommands::Validate { path, strict } => {
                let exit_code = commands::skill::validate::run(path.as_deref(), strict)?;
                std::process::exit(exit_code);
            }
        },
        Commands::Cleanup => commands::cleanup::run(),
    }
}
