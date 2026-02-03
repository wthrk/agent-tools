//! Skill installed command tests

use super::common::TestEnv;
use predicates::prelude::*;

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
