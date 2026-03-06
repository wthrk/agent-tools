//! Common utilities for integration tests

use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test helper that creates an isolated test environment
pub struct TestEnv {
    /// Temporary directory acting as home (kept alive for the test duration)
    #[allow(dead_code)]
    pub home: TempDir,
    /// Path to agent-tools home (~/.agent-tools)
    pub agent_tools_home: PathBuf,
    /// Path to claude home (~/.claude)
    pub claude_home: PathBuf,
    /// Path to codex home (~/.codex)
    pub codex_home: PathBuf,
    /// Path to a test project
    pub project: PathBuf,
}

impl TestEnv {
    pub fn new() -> Self {
        let home = TempDir::new().unwrap();
        let agent_tools_home = home.path().join(".agent-tools");
        let claude_home = home.path().join(".claude");
        let codex_home = home.path().join(".codex");
        let project = home.path().join("test-project");

        // Create directories
        fs::create_dir_all(agent_tools_home.join("skills")).unwrap();
        fs::create_dir_all(&claude_home).unwrap();
        fs::create_dir_all(&codex_home).unwrap();
        fs::create_dir_all(project.join(".claude")).unwrap();
        fs::create_dir_all(project.join(".git")).unwrap();

        Self {
            home,
            agent_tools_home,
            claude_home,
            codex_home,
            project,
        }
    }

    /// Create a skill in ~/.agent-tools/skills/
    pub fn create_skill(&self, name: &str) {
        let skill_dir = self.agent_tools_home.join("skills").join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), format!("# {}\n", name)).unwrap();
    }

    /// Create config.yaml
    pub fn create_config(&self, content: &str) {
        fs::write(self.agent_tools_home.join("config.yaml"), content).unwrap();
    }

    /// Create settings.json
    pub fn create_settings(&self, content: &str) {
        fs::write(self.agent_tools_home.join("settings.json"), content).unwrap();
    }

    /// Create a Claude template profile under templates/claude/<name>/
    pub fn create_claude_profile(&self, name: &str, config_yaml: &str, settings_json: &str) {
        let profile_dir = self.agent_tools_home.join("templates/claude").join(name);
        fs::create_dir_all(&profile_dir).unwrap();
        fs::write(profile_dir.join("config.yaml"), config_yaml).unwrap();
        fs::write(profile_dir.join("settings.json"), settings_json).unwrap();
    }

    /// Create a Codex template profile under templates/codex/<name>/
    pub fn create_codex_profile(&self, name: &str, codex_config_toml: &str) {
        let profile_dir = self.agent_tools_home.join("templates/codex").join(name);
        fs::create_dir_all(profile_dir.join("agents")).unwrap();
        fs::write(profile_dir.join("config.toml"), codex_config_toml).unwrap();
        fs::write(
            profile_dir.join("agents/worker.toml"),
            "model_reasoning_effort = \"medium\"\n",
        )
        .unwrap();
    }

    /// Get a command configured for this test environment
    #[allow(deprecated)]
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("agent-tools").unwrap();
        cmd.env("AGENT_TOOLS_HOME", &self.agent_tools_home);
        cmd.env("CLAUDE_HOME", &self.claude_home);
        cmd.env("CODEX_HOME", &self.codex_home);
        cmd.current_dir(&self.project);
        cmd
    }
}
