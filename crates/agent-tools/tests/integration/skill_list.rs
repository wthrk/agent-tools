//! Skill list command tests

use super::common::TestEnv;
use predicates::prelude::*;

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
