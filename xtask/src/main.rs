use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "Build tasks for agent-tools")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run cargo fmt --check
    Fmt,
    /// Run cargo check
    Check,
    /// Run cargo clippy
    Clippy,
    /// Run cargo deny
    Deny,
    /// Run cargo test
    Test,
    /// Run all CI checks (fmt, check, clippy, deny, test, skill-test)
    Ci,
    /// Run all tests (ci + docker + integration)
    TestAll,
    /// Run skill tests
    SkillTest {
        /// Arguments to pass to skill-test
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run integration tests
    IntegrationTest,
    /// Run docker tests
    DockerTest,
    /// Build docker test image
    DockerTestBuild,
    /// Build release and install to ~/.agent-tools/bin/
    Install,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(e) = run(cli.command) {
        eprintln!("Error: {e:?}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Fmt => cmd_fmt(),
        Commands::Check => cmd_check(),
        Commands::Clippy => cmd_clippy(),
        Commands::Deny => cmd_deny(),
        Commands::Test => cmd_test(),
        Commands::Ci => cmd_ci(),
        Commands::TestAll => cmd_test_all(),
        Commands::SkillTest { args } => cmd_skill_test(&args),
        Commands::IntegrationTest => cmd_integration_test(),
        Commands::DockerTest => cmd_docker_test(),
        Commands::DockerTestBuild => cmd_docker_test_build(),
        Commands::Install => cmd_install(),
    }
}

fn cmd_fmt() -> Result<()> {
    cargo(&["fmt", "--all", "--check"])
}

fn cmd_check() -> Result<()> {
    cargo(&["check", "--workspace", "--all-targets"])
}

fn cmd_clippy() -> Result<()> {
    cargo(&["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])
}

fn cmd_deny() -> Result<()> {
    cargo(&["deny", "check"])
}

fn cmd_test() -> Result<()> {
    cargo(&["test", "--workspace"])
}

fn cmd_ci() -> Result<()> {
    cmd_fmt()?;
    cmd_check()?;
    cmd_clippy()?;
    cmd_deny()?;
    cmd_test()?;
    cmd_skill_test(&[])?;
    Ok(())
}

fn cmd_test_all() -> Result<()> {
    cmd_ci()?;
    cmd_docker_test()?;
    cmd_integration_test()?;
    Ok(())
}

fn cmd_skill_test(args: &[String]) -> Result<()> {
    if args.is_empty() {
        cargo(&[
            "run", "-p", "skill-test", "--release", "--",
            "skills/*",
            "--iterations", "1",
            "--hook", "forced",
            "--threshold", "80",
            "--timeout", "180000",
        ])
    } else {
        let mut cmd_args = vec!["run", "-p", "skill-test", "--release", "--"];
        cmd_args.extend(args.iter().map(String::as_str));
        cargo(&cmd_args)
    }
}

fn cmd_integration_test() -> Result<()> {
    cargo(&[
        "test", "-p", "skill-test",
        "--features", "integration-test",
        "--", "--test-threads=2",
    ])
}

fn cmd_docker_test() -> Result<()> {
    exec("docker", &["compose", "-f", "docker-compose.test.yaml", "run", "--rm", "test"])
}

fn cmd_docker_test_build() -> Result<()> {
    exec("docker", &["compose", "-f", "docker-compose.test.yaml", "build", "--no-cache", "test"])
}

fn cmd_install() -> Result<()> {
    cargo(&["build", "--release", "-p", "agent-tools", "-p", "skill-test"])?;

    let home = env::var_os("HOME").context("HOME environment variable not set")?;
    let bin_dir = PathBuf::from(home).join(".agent-tools/bin");

    std::fs::create_dir_all(&bin_dir)
        .with_context(|| format!("Failed to create directory: {}", bin_dir.display()))?;

    let copy_binary = |name: &str| -> Result<()> {
        let src = PathBuf::from("target/release").join(name);
        let dst = bin_dir.join(name);
        std::fs::copy(&src, &dst)
            .with_context(|| format!("Failed to copy {} to {}", src.display(), dst.display()))?;
        println!("Installed: {}", dst.display());
        Ok(())
    };

    copy_binary("agent-tools")?;
    copy_binary("skill-test")?;

    println!();
    println!("Add to shell profile:");
    println!("  export PATH={}:$PATH", bin_dir.display());

    Ok(())
}

fn cargo(args: &[&str]) -> Result<()> {
    exec("cargo", args)
}

fn exec(program: &str, args: &[&str]) -> Result<()> {
    let cmd_line = format!("{program} {}", args.join(" "));
    eprintln!("$ {cmd_line}");

    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute: {cmd_line}"))?;

    if !status.success() {
        let code_info = match status.code() {
            Some(code) => format!("exit code {code}"),
            None => "terminated by signal".to_string(),
        };
        bail!("{cmd_line}: {code_info}");
    }
    Ok(())
}
