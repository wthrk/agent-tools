//! Skill validate command tests

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
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
#[allow(deprecated)]
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
#[allow(deprecated)]
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
#[allow(deprecated)]
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
#[allow(deprecated)]
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
#[allow(deprecated)]
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
#[allow(deprecated)]
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
#[allow(deprecated)]
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
#[allow(deprecated)]
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
