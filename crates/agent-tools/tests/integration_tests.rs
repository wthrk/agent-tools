//! Integration tests for agent-tools CLI
#![allow(deprecated)] // Command::cargo_bin is deprecated but works for our use case

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Test helper that creates an isolated test environment
struct TestEnv {
    /// Temporary directory acting as home (kept alive for the test duration)
    #[allow(dead_code)]
    home: TempDir,
    /// Path to agent-tools home (~/.agent-tools)
    agent_tools_home: std::path::PathBuf,
    /// Path to claude home (~/.claude)
    claude_home: std::path::PathBuf,
    /// Path to a test project
    project: std::path::PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let home = TempDir::new().unwrap();
        let agent_tools_home = home.path().join(".agent-tools");
        let claude_home = home.path().join(".claude");
        let project = home.path().join("test-project");

        // Create directories
        fs::create_dir_all(agent_tools_home.join("skills")).unwrap();
        fs::create_dir_all(&claude_home).unwrap();
        fs::create_dir_all(project.join(".claude")).unwrap();
        fs::create_dir_all(project.join(".git")).unwrap();

        Self {
            home,
            agent_tools_home,
            claude_home,
            project,
        }
    }

    /// Create a skill in ~/.agent-tools/skills/
    fn create_skill(&self, name: &str) {
        let skill_dir = self.agent_tools_home.join("skills").join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), format!("# {}\n", name)).unwrap();
    }

    /// Create config.yaml
    fn create_config(&self, content: &str) {
        fs::write(self.agent_tools_home.join("config.yaml"), content).unwrap();
    }

    /// Create settings.json
    fn create_settings(&self, content: &str) {
        fs::write(self.agent_tools_home.join("settings.json"), content).unwrap();
    }

    /// Get a command configured for this test environment
    fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("agent-tools").unwrap();
        cmd.env("AGENT_TOOLS_HOME", &self.agent_tools_home);
        cmd.env("CLAUDE_HOME", &self.claude_home);
        cmd.current_dir(&self.project);
        cmd
    }
}

// =============================================================================
// Basic CLI tests
// =============================================================================

#[test]
fn test_version() {
    Command::cargo_bin("agent-tools")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"agent-tools \d+\.\d+\.\d+").unwrap());
}

#[test]
fn test_help_shows_subcommands() {
    Command::cargo_bin("agent-tools")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("build"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("sync"))
        .stdout(predicate::str::contains("link"))
        .stdout(predicate::str::contains("unlink"))
        .stdout(predicate::str::contains("skill"))
        .stdout(predicate::str::contains("cleanup"));
}

// =============================================================================
// build tests
// =============================================================================

#[test]
fn test_build_help() {
    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["build", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Build agent-tools"));
}

#[test]
fn test_build_no_agent_tools_home() {
    let home = TempDir::new().unwrap();

    // Point to non-existent agent-tools home
    let mut cmd = Command::cargo_bin("agent-tools").unwrap();
    cmd.env("AGENT_TOOLS_HOME", home.path().join("nonexistent"));
    cmd.args(["build"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)not found|error").unwrap());
}

// =============================================================================
// update (agent-tools update) tests
// =============================================================================

#[test]
fn test_update_no_git_repo() {
    let env = TestEnv::new();

    // Without git repo, update should fail
    env.cmd()
        .args(["update"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)git|not found|error").unwrap());
}

#[test]
fn test_update_with_uncommitted_changes() {
    let env = TestEnv::new();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&env.agent_tools_home)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&env.agent_tools_home)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&env.agent_tools_home)
        .output()
        .unwrap();

    // Create uncommitted file
    fs::write(env.agent_tools_home.join("uncommitted.txt"), "test").unwrap();

    // With uncommitted changes, update should fail
    env.cmd()
        .args(["update"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)uncommitted|changes").unwrap());
}

// =============================================================================
// skill list tests
// =============================================================================

#[test]
fn test_skill_list_empty() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?i)no skills").unwrap());
}

#[test]
fn test_skill_list_with_skills() {
    let env = TestEnv::new();
    env.create_skill("sample-skill-a");
    env.create_skill("sample-skill-b");

    env.cmd()
        .args(["skill", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sample-skill-a"))
        .stdout(predicate::str::contains("sample-skill-b"));
}

// =============================================================================
// skill install tests
// =============================================================================

#[test]
fn test_skill_install() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Verify skill was installed
    let skill_dir = env.project.join(".claude/skills/test-skill");
    assert!(skill_dir.exists(), "Skill directory not created");
    assert!(skill_dir.join("SKILL.md").exists(), "SKILL.md not copied");
    assert!(
        skill_dir.join(".skill-meta.yaml").exists(),
        ".skill-meta.yaml not created"
    );
}

#[test]
fn test_skill_install_from_subdir() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Create subdirectory
    let subdir = env.project.join("src/components");
    fs::create_dir_all(&subdir).unwrap();

    // Run from subdirectory
    let mut cmd = Command::cargo_bin("agent-tools").unwrap();
    cmd.env("AGENT_TOOLS_HOME", &env.agent_tools_home);
    cmd.env("CLAUDE_HOME", &env.claude_home);
    cmd.current_dir(&subdir);
    cmd.args(["skill", "install", "test-skill"]);
    cmd.assert().success();

    // Verify skill was installed to project root
    let skill_dir = env.project.join(".claude/skills/test-skill");
    assert!(skill_dir.exists(), "Skill not installed to project root");
}

#[test]
fn test_skill_install_not_found() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "install", "nonexistent-skill"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)not found|does not exist|error").unwrap());
}

// =============================================================================
// skill installed tests
// =============================================================================

#[test]
fn test_skill_installed() {
    let env = TestEnv::new();
    env.create_skill("skill-a");
    env.create_skill("skill-b");

    // Install skills
    env.cmd()
        .args(["skill", "install", "skill-a"])
        .assert()
        .success();
    env.cmd()
        .args(["skill", "install", "skill-b"])
        .assert()
        .success();

    // Check installed list
    env.cmd()
        .args(["skill", "installed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("skill-a"))
        .stdout(predicate::str::contains("skill-b"));
}

// =============================================================================
// sync tests
// =============================================================================

#[test]
fn test_sync_settings() {
    let env = TestEnv::new();
    env.create_settings(r#"{"test": true}"#);
    env.create_config(
        r#"config_version: 1
auto_deploy_skills: []
manage_settings: true
manage_plugins: false
"#,
    );

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd().args(["sync"]).assert().success();

    // Check settings.json is linked
    let settings_path = env.claude_home.join("settings.json");
    assert!(
        settings_path.is_symlink(),
        "settings.json should be a symlink"
    );

    let link_target = fs::read_link(&settings_path).unwrap();
    assert_eq!(
        link_target,
        env.agent_tools_home.join("settings.json"),
        "settings.json should link to agent-tools home"
    );
}

#[test]
fn test_sync_skills() {
    let env = TestEnv::new();
    env.create_skill("sample-skill-a");
    env.create_skill("sample-skill-b");
    env.create_config(
        r#"config_version: 1
auto_deploy_skills:
  - sample-skill-a
manage_settings: false
manage_plugins: false
"#,
    );

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd().args(["sync"]).assert().success();

    // Check sample-skill-a is linked
    let skill_a_path = env.claude_home.join("skills/sample-skill-a");
    assert!(
        skill_a_path.is_symlink(),
        "sample-skill-a should be a symlink"
    );

    let link_target = fs::read_link(&skill_a_path).unwrap();
    assert_eq!(
        link_target,
        env.agent_tools_home.join("skills/sample-skill-a"),
        "sample-skill-a should link to agent-tools home"
    );

    // Check sample-skill-b is NOT linked (not in config)
    let skill_b_path = env.claude_home.join("skills/sample-skill-b");
    assert!(
        !skill_b_path.exists(),
        "sample-skill-b should not be linked"
    );
}

#[test]
fn test_sync_dry_run() {
    let env = TestEnv::new();
    env.create_skill("sample-skill-a");
    env.create_settings(r#"{"test": true}"#);
    env.create_config(
        r#"config_version: 1
auto_deploy_skills:
  - sample-skill-a
manage_settings: true
manage_plugins: false
"#,
    );

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd()
        .args(["sync", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));

    // Check that no symlinks were created
    let settings_path = env.claude_home.join("settings.json");
    assert!(
        !settings_path.is_symlink(),
        "settings.json should not be linked in dry-run mode"
    );

    let skill_a_path = env.claude_home.join("skills/sample-skill-a");
    assert!(
        !skill_a_path.is_symlink(),
        "sample-skill-a should not be linked in dry-run mode"
    );
}

// =============================================================================
// link/unlink tests
// =============================================================================

#[test]
fn test_link_unlink() {
    let env = TestEnv::new();
    env.create_skill("sample-skill-a");

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Test link command
    env.cmd()
        .args(["link", "sample-skill-a"])
        .assert()
        .success();

    let skill_path = env.claude_home.join("skills/sample-skill-a");
    assert!(skill_path.is_symlink(), "sample-skill-a should be linked");

    // Test unlink command
    env.cmd()
        .args(["unlink", "sample-skill-a"])
        .assert()
        .success();

    assert!(!skill_path.exists(), "sample-skill-a should be unlinked");
}

#[test]
fn test_link_skill_not_found() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd()
        .args(["link", "nonexistent-skill"])
        .assert()
        .failure();
}

#[test]
fn test_unlink_not_linked() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    env.cmd()
        .args(["unlink", "nonexistent-skill"])
        .assert()
        .failure();
}

// =============================================================================
// skill update tests
// =============================================================================

#[test]
fn test_skill_update_up_to_date() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Update should report up to date
    env.cmd()
        .args(["skill", "update", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("up to date"));
}

#[test]
fn test_skill_update_source_changed() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Modify source skill
    let source_skill = env.agent_tools_home.join("skills/test-skill/SKILL.md");
    fs::write(&source_skill, "# Updated content\n").unwrap();

    // Update should detect and apply changes
    env.cmd()
        .args(["skill", "update", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));

    // Verify update was applied
    let installed_skill = env.project.join(".claude/skills/test-skill/SKILL.md");
    let content = fs::read_to_string(installed_skill).unwrap();
    assert!(content.contains("Updated content"));
}

#[test]
fn test_skill_update_local_changes_conflict() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Modify both source and installed skill
    let source_skill = env.agent_tools_home.join("skills/test-skill/SKILL.md");
    fs::write(&source_skill, "# Source updated\n").unwrap();

    let installed_skill = env.project.join(".claude/skills/test-skill/SKILL.md");
    fs::write(&installed_skill, "# Local changes\n").unwrap();

    // Update should report conflict
    env.cmd()
        .args(["skill", "update", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("local changes"));
}

#[test]
fn test_skill_update_force() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Modify both source and installed skill
    let source_skill = env.agent_tools_home.join("skills/test-skill/SKILL.md");
    fs::write(&source_skill, "# Source updated\n").unwrap();

    let installed_skill = env.project.join(".claude/skills/test-skill/SKILL.md");
    fs::write(&installed_skill, "# Local changes\n").unwrap();

    // Force update should overwrite local changes
    env.cmd()
        .args(["skill", "update", "test-skill", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));

    // Verify source content was applied
    let content = fs::read_to_string(installed_skill).unwrap();
    assert!(content.contains("Source updated"));
}

#[test]
fn test_skill_update_all() {
    let env = TestEnv::new();
    env.create_skill("skill-a");
    env.create_skill("skill-b");

    // Install skills
    env.cmd()
        .args(["skill", "install", "skill-a"])
        .assert()
        .success();
    env.cmd()
        .args(["skill", "install", "skill-b"])
        .assert()
        .success();

    // Update all should process both
    env.cmd()
        .args(["skill", "update", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("skill-a"))
        .stdout(predicate::str::contains("skill-b"));
}

#[test]
fn test_skill_update_not_installed() {
    let env = TestEnv::new();
    env.create_skill("test-skill");
    env.create_skill("other-skill");

    // Install a different skill so .claude/skills exists
    env.cmd()
        .args(["skill", "install", "other-skill"])
        .assert()
        .success();

    // Update uninstalled skill should report not installed
    env.cmd()
        .args(["skill", "update", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not installed"));
}

// =============================================================================
// skill remove tests
// =============================================================================

#[test]
fn test_skill_remove() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    let skill_dir = env.project.join(".claude/skills/test-skill");
    assert!(skill_dir.exists());

    // Remove skill
    env.cmd()
        .args(["skill", "remove", "test-skill"])
        .assert()
        .success();

    assert!(!skill_dir.exists(), "Skill should be removed");
}

#[test]
fn test_skill_remove_not_installed() {
    let env = TestEnv::new();

    // Ensure project skills dir exists
    fs::create_dir_all(env.project.join(".claude/skills")).unwrap();

    env.cmd()
        .args(["skill", "remove", "nonexistent-skill"])
        .assert()
        .failure();
}

// =============================================================================
// skill diff tests
// =============================================================================

#[test]
fn test_skill_diff_no_changes() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Diff should show no changes
    env.cmd()
        .args(["skill", "diff", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences"));
}

#[test]
fn test_skill_diff_with_changes() {
    let env = TestEnv::new();
    env.create_skill("test-skill");

    // Install skill
    env.cmd()
        .args(["skill", "install", "test-skill"])
        .assert()
        .success();

    // Modify installed skill
    let installed_skill = env.project.join(".claude/skills/test-skill/SKILL.md");
    fs::write(&installed_skill, "# Modified locally\n").unwrap();

    // Diff should show changes
    env.cmd()
        .args(["skill", "diff", "test-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SKILL.md"));
}

// =============================================================================
// status tests
// =============================================================================

#[test]
fn test_status() {
    let env = TestEnv::new();
    env.create_skill("sample-skill-a");
    env.create_settings(r#"{"test": true}"#);
    env.create_config(
        r#"config_version: 1
auto_deploy_skills:
  - sample-skill-a
manage_settings: true
manage_plugins: false
"#,
    );

    // Ensure directories exist
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Run sync first
    env.cmd().args(["sync"]).assert().success();

    // Status should show linked skills
    env.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sample-skill-a"))
        .stdout(predicate::str::contains("symlink"));
}

#[test]
fn test_status_no_config() {
    let env = TestEnv::new();

    // Status without config should work
    env.cmd().args(["status"]).assert().success();
}

// =============================================================================
// cleanup tests
// =============================================================================

#[test]
fn test_cleanup() {
    let env = TestEnv::new();

    // Create old backup directories
    let backups_dir = env.agent_tools_home.join("backups");
    fs::create_dir_all(&backups_dir).unwrap();
    fs::create_dir_all(backups_dir.join("old-backup-dir")).unwrap();
    fs::write(backups_dir.join("old-backup-file.txt"), "backup").unwrap();

    // Cleanup should succeed
    env.cmd().args(["cleanup"]).assert().success();

    // Backups should be cleaned
    let entries: Vec<_> = fs::read_dir(&backups_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(entries.is_empty(), "Backups should be cleaned up");
}

#[test]
fn test_cleanup_no_backups() {
    let env = TestEnv::new();

    // Cleanup with no backup directory should succeed
    env.cmd().args(["cleanup"]).assert().success();
}

// =============================================================================
// skill new tests
// =============================================================================

#[test]
fn test_skill_new_creates_skill() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Create skill without adding to config
    env.cmd()
        .args(["skill", "new", "test-skill", "--no-auto-deploy"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created skill"));

    // Verify skill directory and SKILL.md were created
    let skill_dir = env.agent_tools_home.join("skills/test-skill");
    assert!(skill_dir.exists(), "Skill directory should exist");
    assert!(skill_dir.join("SKILL.md").exists(), "SKILL.md should exist");

    // Verify SKILL.md contains correct content
    let content = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(content.contains("name: test-skill"));
}

#[test]
fn test_skill_new_adds_to_config_and_links() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Create skill with auto-deploy (using --yes to skip prompt)
    env.cmd()
        .args(["skill", "new", "my-skill", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto_deploy_skills"));

    // Verify skill directory was created
    let skill_dir = env.agent_tools_home.join("skills/my-skill");
    assert!(skill_dir.exists());

    // Verify config was updated
    let config_path = env.agent_tools_home.join("config.yaml");
    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("my-skill"),
        "Config should contain the skill name"
    );

    // Verify symlink was created
    let link_path = env.claude_home.join("skills/my-skill");
    assert!(link_path.is_symlink(), "Symlink should be created");
}

#[test]
fn test_skill_new_already_exists() {
    let env = TestEnv::new();
    env.create_skill("existing-skill");

    // Try to create skill that already exists
    env.cmd()
        .args(["skill", "new", "existing-skill", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_skill_new_invalid_name_path_traversal() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "new", "../escape", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)letters|invalid|error").unwrap());
}

#[test]
fn test_skill_new_invalid_name_slash() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "new", "foo/bar", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)letters|invalid|error").unwrap());
}

#[test]
fn test_skill_new_invalid_name_dot() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "new", "skill.name", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)letters|invalid|error").unwrap());
}

#[test]
fn test_skill_new_invalid_name_special_chars() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "new", "skill@name", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)letters|invalid|error").unwrap());
}

#[test]
fn test_skill_new_invalid_name_starts_with_hyphen() {
    let env = TestEnv::new();

    env.cmd()
        .args(["skill", "new", "-invalid", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)cannot start|invalid|error").unwrap());
}

#[test]
fn test_skill_new_invalid_name_with_underscore() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Underscores are not allowed in skill names (only lowercase, numbers, hyphens)
    env.cmd()
        .args(["skill", "new", "my_skill_name", "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("lowercase"));
}

#[test]
fn test_skill_new_link_already_exists() {
    let env = TestEnv::new();

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Create a conflicting file at the link target
    fs::write(env.claude_home.join("skills/conflict-skill"), "existing").unwrap();

    // Try to create skill with auto-deploy (should fail on link creation)
    env.cmd()
        .args(["skill", "new", "conflict-skill", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)exists|error").unwrap());
}

#[test]
fn test_skill_new_creates_config_if_missing() {
    let env = TestEnv::new();

    // Remove config if it exists
    let config_path = env.agent_tools_home.join("config.yaml");
    if config_path.exists() {
        fs::remove_file(&config_path).unwrap();
    }

    // Ensure claude skills dir exists
    fs::create_dir_all(env.claude_home.join("skills")).unwrap();

    // Create skill with auto-deploy
    env.cmd()
        .args(["skill", "new", "auto-config-skill", "--yes"])
        .assert()
        .success();

    // Verify config was created
    assert!(config_path.exists(), "Config should be created");
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("auto-config-skill"));
    assert!(content.contains("config_version: 1"));
}

#[test]
fn test_skill_new_invalid_name_too_long() {
    let env = TestEnv::new();

    // Create a name that's 65 characters (exceeds 64 limit)
    let long_name = "a".repeat(65);

    env.cmd()
        .args(["skill", "new", &long_name, "--no-auto-deploy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"(?i)cannot exceed|too long|error").unwrap());
}

#[test]
fn test_skill_new_help() {
    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["skill", "new", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Create a new skill"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--no-auto-deploy"));
}

// =============================================================================
// skill validate tests
// =============================================================================

#[test]
fn test_skill_validate_help() {
    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["skill", "validate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Validate"))
        .stdout(predicate::str::contains("--strict"));
}

#[test]
fn test_skill_validate_valid_skill_exit_code_0() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("SKILL.md"),
        r#"---
name: test-skill
description: A valid test skill
---

# Test Skill
"#,
    )
    .unwrap();

    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["skill", "validate", dir.path().to_str().unwrap()])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Errors: 0"))
        .stdout(predicate::str::contains("Warnings: 0"));
}

#[test]
fn test_skill_validate_with_errors_exit_code_1() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("SKILL.md"),
        r#"---
name: Invalid_Name
description: A test skill with invalid name
---

# Test Skill
"#,
    )
    .unwrap();

    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["skill", "validate", dir.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Errors: 1"));
}

#[test]
fn test_skill_validate_warnings_only_exit_code_2() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("SKILL.md"),
        r#"---
name: test-skill
description: A valid test skill
---

# Test Skill
"#,
    )
    .unwrap();
    // Create a forbidden file to trigger a warning
    fs::write(dir.path().join("CHANGELOG.md"), "# Changelog").unwrap();

    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["skill", "validate", dir.path().to_str().unwrap()])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("Errors: 0"))
        .stdout(predicate::str::contains("Warnings: 1"));
}

#[test]
fn test_skill_validate_strict_warnings_exit_code_1() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("SKILL.md"),
        r#"---
name: test-skill
description: A valid test skill
---

# Test Skill
"#,
    )
    .unwrap();
    // Create a forbidden file to trigger a warning
    fs::write(dir.path().join("CHANGELOG.md"), "# Changelog").unwrap();

    Command::cargo_bin("agent-tools")
        .unwrap()
        .args([
            "skill",
            "validate",
            "--strict",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Errors: 0"))
        .stdout(predicate::str::contains("Warnings: 1"));
}

#[test]
fn test_skill_validate_missing_skill_md_exit_code_1() {
    let dir = TempDir::new().unwrap();
    // No SKILL.md created

    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["skill", "validate", dir.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("SKILL.md not found"));
}

#[test]
fn test_skill_validate_disallowed_frontmatter_key() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("SKILL.md"),
        r#"---
name: test-skill
description: A test skill
author: Someone
---

# Test Skill
"#,
    )
    .unwrap();

    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["skill", "validate", dir.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("author"));
}

#[test]
fn test_skill_validate_hooks_key_allowed() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("SKILL.md"),
        r#"---
name: test-skill
description: A test skill with hooks
hooks:
  post-install: ./setup.sh
---

# Test Skill
"#,
    )
    .unwrap();

    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["skill", "validate", dir.path().to_str().unwrap()])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Errors: 0"));
}

#[test]
fn test_skill_validate_reference_depth_warning() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("SKILL.md"),
        r#"---
name: test-skill
description: A test skill
---

# Test Skill
"#,
    )
    .unwrap();

    // Create references directory with a markdown file that links to another md file
    let references_dir = dir.path().join("references");
    fs::create_dir(&references_dir).unwrap();
    fs::write(
        references_dir.join("guide.md"),
        r#"# Guide

See [other doc](other.md) for more info.
"#,
    )
    .unwrap();

    Command::cargo_bin("agent-tools")
        .unwrap()
        .args(["skill", "validate", dir.path().to_str().unwrap()])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("Reference depth > 1"));
}
