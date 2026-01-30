//! Integration tests for skill-tools CLI
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
    /// Path to skill-tools home (~/.skill-tools)
    skill_tools_home: std::path::PathBuf,
    /// Path to claude home (~/.claude)
    claude_home: std::path::PathBuf,
    /// Path to a test project
    project: std::path::PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let home = TempDir::new().unwrap();
        let skill_tools_home = home.path().join(".skill-tools");
        let claude_home = home.path().join(".claude");
        let project = home.path().join("test-project");

        // Create directories
        fs::create_dir_all(skill_tools_home.join("skills")).unwrap();
        fs::create_dir_all(&claude_home).unwrap();
        fs::create_dir_all(project.join(".claude")).unwrap();
        fs::create_dir_all(project.join(".git")).unwrap();

        Self {
            home,
            skill_tools_home,
            claude_home,
            project,
        }
    }

    /// Create a skill in ~/.skill-tools/skills/
    fn create_skill(&self, name: &str) {
        let skill_dir = self.skill_tools_home.join("skills").join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), format!("# {}\n", name)).unwrap();
    }

    /// Create config.yaml
    fn create_config(&self, content: &str) {
        fs::write(self.skill_tools_home.join("config.yaml"), content).unwrap();
    }

    /// Create settings.json
    fn create_settings(&self, content: &str) {
        fs::write(self.skill_tools_home.join("settings.json"), content).unwrap();
    }

    /// Get a command configured for this test environment
    fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("skill-tools").unwrap();
        cmd.env("SKILL_TOOLS_HOME", &self.skill_tools_home);
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
    Command::cargo_bin("skill-tools")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"skill-tools \d+\.\d+\.\d+").unwrap());
}

#[test]
fn test_help_shows_subcommands() {
    Command::cargo_bin("skill-tools")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
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
// update (skill-tools update) tests
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
        .current_dir(&env.skill_tools_home)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&env.skill_tools_home)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&env.skill_tools_home)
        .output()
        .unwrap();

    // Create uncommitted file
    fs::write(env.skill_tools_home.join("uncommitted.txt"), "test").unwrap();

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
    let mut cmd = Command::cargo_bin("skill-tools").unwrap();
    cmd.env("SKILL_TOOLS_HOME", &env.skill_tools_home);
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
        env.skill_tools_home.join("settings.json"),
        "settings.json should link to skill-tools home"
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
        env.skill_tools_home.join("skills/sample-skill-a"),
        "sample-skill-a should link to skill-tools home"
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
    let source_skill = env.skill_tools_home.join("skills/test-skill/SKILL.md");
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
    let source_skill = env.skill_tools_home.join("skills/test-skill/SKILL.md");
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
    let source_skill = env.skill_tools_home.join("skills/test-skill/SKILL.md");
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
    let backups_dir = env.skill_tools_home.join("backups");
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
