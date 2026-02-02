use anyhow::Result;
use colored::Colorize;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

/// Regex for validating skill name format
static NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]([a-z0-9-]*[a-z0-9])?$").unwrap());

/// Regex for finding markdown links (including anchors like foo.md#section)
static MD_LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[.*?\]\(([^)]+\.md)(#[^)]*)?\)").unwrap());

/// Allowed frontmatter keys
const ALLOWED_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "allowed-tools",
    "metadata",
    "user-invocable",
    "disable-model-invocation",
    "argument-hint",
];

/// Forbidden files that should not exist in a skill directory
const FORBIDDEN_FILES: &[&str] = &[
    "CHANGELOG.md",
    "INSTALLATION_GUIDE.md",
    "QUICK_REFERENCE.md",
];

/// Maximum line count before warning
const MAX_LINES: usize = 500;

/// Maximum word count before warning
const MAX_WORDS: usize = 5000;

/// Maximum name length
const MAX_NAME_LENGTH: usize = 64;

/// Maximum description length
const MAX_DESCRIPTION_LENGTH: usize = 1024;

/// Validation result
#[derive(Default)]
struct ValidationResult {
    errors: Vec<String>,
    warnings: Vec<String>,
    successes: Vec<String>,
}

impl ValidationResult {
    fn add_error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }

    fn add_warning(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    fn add_success(&mut self, msg: impl Into<String>) {
        self.successes.push(msg.into());
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Parse frontmatter from SKILL.md content
fn parse_frontmatter(content: &str) -> Result<(serde_yaml::Value, &str), String> {
    // Must start with ---
    if !content.starts_with("---") {
        return Err("Frontmatter must start with '---'".to_string());
    }

    // Find closing ---
    let rest = &content[3..];
    let closing_pos = rest.find("\n---").ok_or("Missing closing '---' in frontmatter")?;

    let frontmatter_str = &rest[..closing_pos];
    let body = &rest[closing_pos + 4..];

    // Parse YAML
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(frontmatter_str).map_err(|e| format!("YAML parse error: {}", e))?;

    Ok((yaml, body))
}

/// Validate skill name format
fn validate_name_format(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }

    if name.len() > MAX_NAME_LENGTH {
        return Err(format!(
            "Name exceeds {} characters ({} chars)",
            MAX_NAME_LENGTH,
            name.len()
        ));
    }

    // Regex: ^[a-z0-9][a-z0-9-]*[a-z0-9]$ or ^[a-z0-9]$
    if !NAME_REGEX.is_match(name) {
        return Err(format!(
            "Name must match pattern ^[a-z0-9][a-z0-9-]*[a-z0-9]$ or ^[a-z0-9]$: '{}'",
            name
        ));
    }

    // Check for consecutive hyphens
    if name.contains("--") {
        return Err(format!("Name cannot contain consecutive hyphens: '{}'", name));
    }

    Ok(())
}

/// Validate description
fn validate_description(description: &str) -> Result<(), String> {
    if description.contains('<') || description.contains('>') {
        return Err("Description cannot contain '<' or '>'".to_string());
    }

    if description.len() > MAX_DESCRIPTION_LENGTH {
        return Err(format!(
            "Description exceeds {} characters ({} chars)",
            MAX_DESCRIPTION_LENGTH,
            description.len()
        ));
    }

    Ok(())
}

/// Count words in text
fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Check if a markdown file contains a table of contents
fn has_table_of_contents(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("## table of contents") || lower.contains("## contents")
}

/// Check reference depth (markdown files linking to other markdown files)
fn check_reference_depth(path: &Path, result: &mut ValidationResult) {
    let references_dir = path.join("references");
    if !references_dir.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(&references_dir) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.extension().is_some_and(|e| e == "md") {
                if let Ok(content) = fs::read_to_string(&entry_path) {
                    // Check for links to other .md files
                    for cap in MD_LINK_REGEX.captures_iter(&content) {
                        if let Some(link) = cap.get(1) {
                            let link_str = link.as_str();
                            // Check if this is a relative link to another md file
                            if !link_str.starts_with("http") {
                                result.add_warning(format!(
                                    "Reference depth > 1: {} links to {}",
                                    entry_path.display(),
                                    link_str
                                ));
                            }
                        }
                    }

                    // Check for table of contents in large files
                    let line_count = content.lines().count();
                    if line_count > 100 && !has_table_of_contents(&content) {
                        result.add_warning(format!(
                            "File {} has {} lines but no table of contents",
                            entry_path.display(),
                            line_count
                        ));
                    }
                }
            }
        }
    }
}

/// Validate a skill directory
fn validate_skill(path: &Path) -> ValidationResult {
    let mut result = ValidationResult::default();

    let skill_md_path = path.join("SKILL.md");

    // Check SKILL.md exists
    if !skill_md_path.exists() {
        result.add_error("SKILL.md not found");
        return result;
    }
    result.add_success("SKILL.md exists");

    // Read content
    let content = match fs::read_to_string(&skill_md_path) {
        Ok(c) => c,
        Err(e) => {
            result.add_error(format!("Failed to read SKILL.md: {}", e));
            return result;
        }
    };

    // Parse frontmatter
    let (frontmatter, body) = match parse_frontmatter(&content) {
        Ok((f, b)) => {
            result.add_success("Frontmatter format valid");
            (f, b)
        }
        Err(e) => {
            result.add_error(e);
            return result;
        }
    };

    // Check frontmatter is a mapping
    let mapping = match frontmatter.as_mapping() {
        Some(m) => m,
        None => {
            result.add_error("Frontmatter must be a YAML mapping");
            return result;
        }
    };

    // Check for disallowed keys
    let allowed_set: HashSet<&str> = ALLOWED_KEYS.iter().copied().collect();
    for key in mapping.keys() {
        if let Some(key_str) = key.as_str() {
            if !allowed_set.contains(key_str) {
                result.add_error(format!("Disallowed frontmatter key: '{}'", key_str));
            }
        }
    }

    // Check required fields
    let name = match mapping.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            result.add_error("Missing required field: 'name'");
            ""
        }
    };

    let description = match mapping.get("description").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => {
            result.add_error("Missing required field: 'description'");
            ""
        }
    };

    // Validate name format
    if !name.is_empty() {
        match validate_name_format(name) {
            Ok(()) => result.add_success(format!("Name '{}' is valid", name)),
            Err(e) => result.add_error(e),
        }
    }

    // Validate description
    if !description.is_empty() {
        match validate_description(description) {
            Ok(()) => result.add_success("Description is valid"),
            Err(e) => result.add_error(e),
        }
    }

    // Warnings: line count
    let line_count = content.lines().count();
    if line_count > MAX_LINES {
        result.add_warning(format!(
            "SKILL.md has {} lines (recommended: < {})",
            line_count, MAX_LINES
        ));
    } else {
        result.add_success(format!("Line count: {} (< {})", line_count, MAX_LINES));
    }

    // Warnings: word count
    let word_count = count_words(&content);
    if word_count > MAX_WORDS {
        result.add_warning(format!(
            "SKILL.md has {} words (recommended: < {})",
            word_count, MAX_WORDS
        ));
    } else {
        result.add_success(format!("Word count: {} (< {})", word_count, MAX_WORDS));
    }

    // Warnings: forbidden files
    for forbidden in FORBIDDEN_FILES {
        let forbidden_path = path.join(forbidden);
        if forbidden_path.exists() {
            result.add_warning(format!(
                "Non-recommended file exists: {} (consider removing)",
                forbidden
            ));
        }
    }

    // Warnings: reference depth and TOC
    check_reference_depth(path, &mut result);

    // Check table of contents in SKILL.md body
    if line_count > 100 && !has_table_of_contents(body) {
        result.add_warning(format!(
            "SKILL.md has {} lines but no table of contents",
            line_count
        ));
    }

    result
}

/// Print validation results
fn print_results(result: &ValidationResult) {
    for success in &result.successes {
        println!("{} {}", "✓".green(), success);
    }

    for warning in &result.warnings {
        println!("{} {}", "⚠".yellow(), warning);
    }

    for error in &result.errors {
        println!("{} {}", "✗".red(), error);
    }

    println!();
    println!(
        "Errors: {}, Warnings: {}",
        result.errors.len(),
        result.warnings.len()
    );
}

/// Run validation command
pub fn run(path: Option<&str>, strict: bool) -> Result<i32> {
    let path = path.unwrap_or(".");
    let skill_path = Path::new(path);

    if !skill_path.exists() {
        anyhow::bail!("Path does not exist: {}", path);
    }

    println!(
        "Validating skill at: {}\n",
        skill_path.canonicalize()?.display()
    );

    let result = validate_skill(skill_path);
    print_results(&result);

    // Determine exit code
    if result.has_errors() {
        Ok(1)
    } else if result.has_warnings() {
        if strict {
            Ok(1)
        } else {
            Ok(2)
        }
    } else {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_skill_md(dir: &Path, content: &str) {
        fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn test_validate_name_format_valid() {
        assert!(validate_name_format("a").is_ok());
        assert!(validate_name_format("abc").is_ok());
        assert!(validate_name_format("my-skill").is_ok());
        assert!(validate_name_format("skill-123").is_ok());
        assert!(validate_name_format("a1b2c3").is_ok());
    }

    #[test]
    fn test_validate_name_format_invalid() {
        assert!(validate_name_format("").is_err());
        assert!(validate_name_format("-skill").is_err());
        assert!(validate_name_format("skill-").is_err());
        assert!(validate_name_format("skill--name").is_err());
        assert!(validate_name_format("My-Skill").is_err());
        assert!(validate_name_format("skill_name").is_err());
    }

    #[test]
    fn test_validate_name_length() {
        let long_name = "a".repeat(65);
        assert!(validate_name_format(&long_name).is_err());

        let max_name = "a".repeat(64);
        assert!(validate_name_format(&max_name).is_ok());
    }

    #[test]
    fn test_validate_description_valid() {
        assert!(validate_description("A simple description").is_ok());
        assert!(validate_description("Description with numbers 123").is_ok());
    }

    #[test]
    fn test_validate_description_invalid() {
        assert!(validate_description("Has <tag> in it").is_err());
        assert!(validate_description("Has > symbol").is_err());
    }

    #[test]
    fn test_validate_description_length() {
        let long_desc = "a".repeat(1025);
        assert!(validate_description(&long_desc).is_err());

        let max_desc = "a".repeat(1024);
        assert!(validate_description(&max_desc).is_ok());
    }

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = r#"---
name: test
description: A test skill
---

# Content
"#;
        let result = parse_frontmatter(content);
        assert!(result.is_ok());
        let (yaml, _body) = result.unwrap();
        assert_eq!(yaml["name"].as_str(), Some("test"));
    }

    #[test]
    fn test_parse_frontmatter_missing_start() {
        let content = "name: test\n---\n";
        assert!(parse_frontmatter(content).is_err());
    }

    #[test]
    fn test_parse_frontmatter_missing_end() {
        let content = "---\nname: test\n";
        assert!(parse_frontmatter(content).is_err());
    }

    #[test]
    fn test_validate_skill_missing_skill_md() {
        let dir = TempDir::new().unwrap();
        let result = validate_skill(dir.path());
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e.contains("SKILL.md not found")));
    }

    #[test]
    fn test_validate_skill_valid() {
        let dir = TempDir::new().unwrap();
        create_skill_md(
            dir.path(),
            r#"---
name: test-skill
description: A test skill for validation
---

# Test Skill
"#,
        );

        let result = validate_skill(dir.path());
        assert!(!result.has_errors());
    }

    #[test]
    fn test_validate_skill_missing_name() {
        let dir = TempDir::new().unwrap();
        create_skill_md(
            dir.path(),
            r#"---
description: A test skill
---

# Test
"#,
        );

        let result = validate_skill(dir.path());
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e.contains("name")));
    }

    #[test]
    fn test_validate_skill_disallowed_key() {
        let dir = TempDir::new().unwrap();
        create_skill_md(
            dir.path(),
            r#"---
name: test
description: A test skill
author: Someone
---

# Test
"#,
        );

        let result = validate_skill(dir.path());
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e.contains("author")));
    }

    #[test]
    fn test_validate_skill_forbidden_files_warning() {
        let dir = TempDir::new().unwrap();
        create_skill_md(
            dir.path(),
            r#"---
name: test
description: A test skill
---

# Test
"#,
        );
        fs::write(dir.path().join("CHANGELOG.md"), "# Changelog").unwrap();

        let result = validate_skill(dir.path());
        assert!(!result.has_errors());
        assert!(result.has_warnings());
        assert!(result.warnings.iter().any(|w| w.contains("CHANGELOG.md")));
    }

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("one two three"), 3);
        assert_eq!(count_words("word"), 1);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("  spaced   out  "), 2);
    }

    #[test]
    fn test_has_table_of_contents() {
        assert!(has_table_of_contents("## Table of Contents\n- Item"));
        assert!(has_table_of_contents("## Contents\n- Item"));
        assert!(has_table_of_contents("## table of contents\n"));
        assert!(!has_table_of_contents("# No TOC here"));
    }
}
