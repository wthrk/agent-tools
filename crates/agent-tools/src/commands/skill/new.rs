use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::io::{self, Write};

use crate::commands::link;
use crate::config::{add_auto_deploy_skill, validate_skill_name};
use crate::paths;

/// Convert kebab-case to Title Case
/// Example: "my-skill" -> "My Skill"
fn to_title_case(name: &str) -> String {
    name.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// SKILL.md template (English)
fn skill_template(name: &str) -> String {
    let title = to_title_case(name);
    format!(
        r#"---
name: {name}
description: "[TODO: What it does]. Use when [TODO: trigger conditions]."
allowed-tools: []
user-invocable: true
argument-hint:
---

# {title}

## Overview

TODO: Describe what this skill does in 1-2 sentences.

## When to Use

Use this skill when:

- TODO: First trigger condition
- TODO: Second trigger condition

## The Process

1. TODO: First step
2. TODO: Second step
3. TODO: Third step

## Tips

- TODO: Add helpful tips for using this skill effectively
"#
    )
}

/// README.md template (Japanese)
fn readme_template(name: &str) -> String {
    let title = to_title_case(name);
    format!(
        r#"# {title}

## 概要

TODO: このスキルが何をするか1-2文で説明してください。

## 使用条件

以下の場合にこのスキルを使用してください:

- TODO: 最初のトリガー条件
- TODO: 2番目のトリガー条件

## ワークフロー

1. TODO: 最初のステップ
2. TODO: 2番目のステップ
3. TODO: 3番目のステップ

## ヒント

- TODO: このスキルを効果的に使用するためのヒントを追加してください
"#
    )
}

/// AGENTS.md template
fn agents_template() -> &'static str {
    r#"# Agent Instructions

README.md is the Japanese explanation of this skill.
When updating SKILL.md or any related files, also update README.md to keep them in sync.
"#
}

/// Ask user a yes/no question with default.
///
/// Returns true if user answers yes (or presses Enter for default yes).
fn ask_yes_no(prompt: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{} {} ", prompt, suffix);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return Ok(default_yes);
    }

    Ok(input == "y" || input == "yes")
}

pub fn run(name: &str, add_to_config: Option<bool>) -> Result<()> {
    // Validate skill name
    validate_skill_name(name)?;

    let skills_dir = paths::skills_dir()?;
    let skill_dir = skills_dir.join(name);

    // Check if skill already exists
    if skill_dir.exists() {
        bail!("Skill '{}' already exists at {}", name, skill_dir.display());
    }

    // Create skills directory if it doesn't exist
    if !skills_dir.exists() {
        fs::create_dir_all(&skills_dir).with_context(|| {
            format!(
                "Failed to create skills directory: {}",
                skills_dir.display()
            )
        })?;
    }

    // Create skill directory
    fs::create_dir_all(&skill_dir)
        .with_context(|| format!("Failed to create skill directory: {}", skill_dir.display()))?;

    // Create SKILL.md
    let skill_md_path = skill_dir.join("SKILL.md");
    fs::write(&skill_md_path, skill_template(name))
        .with_context(|| format!("Failed to create SKILL.md: {}", skill_md_path.display()))?;

    // Create README.md
    let readme_path = skill_dir.join("README.md");
    fs::write(&readme_path, readme_template(name))
        .with_context(|| format!("Failed to create README.md: {}", readme_path.display()))?;

    // Create AGENTS.md
    let agents_path = skill_dir.join("AGENTS.md");
    fs::write(&agents_path, agents_template())
        .with_context(|| format!("Failed to create AGENTS.md: {}", agents_path.display()))?;

    println!(
        "{} Created skill '{}' at {}",
        "✓".green(),
        name.cyan(),
        skill_dir.display()
    );

    // Ask about adding to auto_deploy_skills
    let should_add = match add_to_config {
        Some(value) => value,
        None => ask_yes_no("Add to auto_deploy_skills?", true)?,
    };

    if should_add {
        // Create symlink first (so if it fails, config is not modified)
        link::run(name)?;

        // Add to config.yaml
        let config_path = paths::config_path()?;
        add_auto_deploy_skill(&config_path, name)?;
        println!(
            "  {} Added '{}' to auto_deploy_skills in config.yaml",
            "✓".green(),
            name.cyan()
        );
    }

    println!();
    println!(
        "{} Edit {} to define your skill",
        "→".blue(),
        skill_md_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_title_case() {
        assert_eq!(to_title_case("my-skill"), "My Skill");
        assert_eq!(to_title_case("skill"), "Skill");
        assert_eq!(to_title_case("my-awesome-skill"), "My Awesome Skill");
        assert_eq!(to_title_case("a"), "A");
        assert_eq!(to_title_case("a-b-c"), "A B C");
    }

    #[test]
    fn test_skill_template_contains_required_sections() {
        let template = skill_template("test-skill");
        assert!(template.contains("name: test-skill"));
        assert!(template.contains("# Test Skill"));
        assert!(template.contains("## Overview"));
        assert!(template.contains("## When to Use"));
        assert!(template.contains("## The Process"));
        assert!(template.contains("## Tips"));
    }

    #[test]
    fn test_readme_template_contains_required_sections() {
        let template = readme_template("test-skill");
        assert!(template.contains("# Test Skill"));
        assert!(template.contains("## 概要"));
        assert!(template.contains("## 使用条件"));
        assert!(template.contains("## ワークフロー"));
        assert!(template.contains("## ヒント"));
    }

    #[test]
    fn test_agents_template() {
        let template = agents_template();
        assert!(template.contains("README.md"));
        assert!(template.contains("SKILL.md"));
        assert!(template.contains("sync"));
    }
}
