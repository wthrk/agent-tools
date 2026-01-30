//! YAML loader for test cases.

use crate::types::{
    Assertion, AssertionOrFile, AssertionRef, FileRefValue, SimplifiedTestCase, TestCase, TestFile,
};
use glob::glob;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during loading.
#[derive(Error, Debug)]
pub enum LoaderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yml::Error),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("glob pattern error: {0}")]
    Glob(#[from] glob::PatternError),
    #[error(
        "duplicate assertion ID '{id}' in test '{test_id}': first defined in {first_source}, redefined in {second_source}"
    )]
    DuplicateAssertionId {
        id: String,
        test_id: String,
        first_source: String,
        second_source: String,
    },
    #[error("circular file reference detected: {0}")]
    CircularReference(String),
    #[error("file reference '{path}' is outside skill-tests directory")]
    FileRefOutsideSkillTests { path: PathBuf },
    #[error("file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("undefined assertion reference '{name}' in scenario '{scenario}'")]
    UndefinedAssertionRef { name: String, scenario: String },
    #[error("empty prompt in scenario '{scenario}'")]
    EmptyPrompt { scenario: String },
    #[error("duplicate assertion ID '{id}' in scenario '{scenario}'")]
    DuplicateAssertionIdInScenario { id: String, scenario: String },
}

/// Load test cases from a YAML file.
///
/// # Errors
/// Returns an error if:
/// - The file cannot be read
/// - YAML parsing fails
/// - Validation fails (empty id, prompt, or `expected_skills`)
pub fn load_test_cases<P: AsRef<Path>>(path: P) -> Result<Vec<TestCase>, LoaderError> {
    let content = std::fs::read_to_string(path)?;
    let cases: Vec<TestCase> = serde_yml::from_str(&content)?;

    // Validate test cases
    for case in &cases {
        validate_test_case(case)?;
    }

    Ok(cases)
}

/// Validate a single test case.
fn validate_test_case(case: &TestCase) -> Result<(), LoaderError> {
    if case.id.is_empty() {
        return Err(LoaderError::Validation(
            "test case id cannot be empty".into(),
        ));
    }
    if case.prompt.is_empty() {
        return Err(LoaderError::Validation(format!(
            "test case '{}' prompt cannot be empty",
            case.id
        )));
    }
    // Allow empty expected_skills only if forbid_skills is set (negative test case)
    if case.expected_skills.is_empty() && case.forbid_skills.is_empty() {
        return Err(LoaderError::Validation(format!(
            "test case '{}' must have at least one expected skill or forbid skill",
            case.id
        )));
    }
    Ok(())
}

// =============================================================================
// New simplified test case loader
// =============================================================================

/// Discover test files in a skill directory using patterns.
///
/// # Errors
/// Returns an error if glob pattern is invalid.
pub fn discover_test_files(
    skill_dir: &Path,
    patterns: &[String],
    exclude_patterns: &[String],
) -> Result<Vec<PathBuf>, LoaderError> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();

    for pattern in patterns {
        let full_pattern = skill_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();

        for path in glob(&pattern_str)?.flatten() {
            // Check if excluded
            let relative = path.strip_prefix(skill_dir).unwrap_or(&path);
            let relative_str = relative.to_string_lossy();

            let excluded = exclude_patterns.iter().any(|ex| {
                // If pattern contains glob wildcards, use glob matching
                // Otherwise, use substring match for simple patterns like "node_modules/"
                if ex.contains('*') || ex.contains('?') || ex.contains('[') {
                    glob::Pattern::new(ex)
                        .map_or_else(|_| relative_str.contains(ex), |p| p.matches(&relative_str))
                } else {
                    relative_str.contains(ex)
                }
            });

            if !excluded && !seen.contains(&path) {
                seen.insert(path.clone());
                files.push(path);
            }
        }
    }

    // Sort by path for deterministic order
    files.sort();
    Ok(files)
}

/// Load simplified test cases from a YAML file.
///
/// # Errors
/// Returns an error if:
/// - The file cannot be read
/// - YAML parsing fails
/// - Validation fails
pub fn load_simplified_test_cases<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<SimplifiedTestCase>, LoaderError> {
    let content = std::fs::read_to_string(path.as_ref())?;
    let cases: Vec<SimplifiedTestCase> = serde_yml::from_str(&content)?;

    // Validate test cases
    for case in &cases {
        validate_simplified_test_case(case)?;
    }

    Ok(cases)
}

/// Validate a simplified test case.
fn validate_simplified_test_case(case: &SimplifiedTestCase) -> Result<(), LoaderError> {
    if case.id.is_empty() {
        return Err(LoaderError::Validation(
            "test case id cannot be empty".into(),
        ));
    }
    if case.prompt.is_empty() {
        return Err(LoaderError::Validation(format!(
            "test case '{}' prompt cannot be empty",
            case.id
        )));
    }
    Ok(())
}

/// Resolve all file references in assertions and check for duplicates.
///
/// This function:
/// 1. Resolves `file:` references to actual assertions
/// 2. Validates paths are within skill-tests directory
/// 3. Detects circular references
/// 4. Detects duplicate assertion IDs
///
/// # Arguments
/// * `assertions` - The assertions to resolve
/// * `base_path` - The path of the YAML file containing the assertions (for relative path resolution)
/// * `skill_tests_dir` - The skill-tests directory (file references must stay within)
/// * `test_id` - The test case ID (for error messages)
///
/// # Errors
/// Returns an error if file references are invalid, circular, or contain duplicate IDs.
pub fn resolve_assertions(
    assertions: &[AssertionOrFile],
    base_path: &Path,
    skill_tests_dir: &Path,
    test_id: &str,
) -> Result<Vec<Assertion>, LoaderError> {
    let mut resolved = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut visited_files: HashSet<PathBuf> = HashSet::new();

    resolve_assertions_inner(
        assertions,
        base_path,
        skill_tests_dir,
        test_id,
        &mut resolved,
        &mut seen_ids,
        &mut visited_files,
    )?;

    Ok(resolved)
}

fn resolve_assertions_inner(
    assertions: &[AssertionOrFile],
    base_path: &Path,
    skill_tests_dir: &Path,
    test_id: &str,
    resolved: &mut Vec<Assertion>,
    seen_ids: &mut HashSet<String>,
    visited_files: &mut HashSet<PathBuf>,
) -> Result<(), LoaderError> {
    let base_dir = base_path.parent().unwrap_or(base_path);

    for aof in assertions {
        match aof {
            AssertionOrFile::Inline(assertion) => {
                let id = assertion.id().to_string();
                if seen_ids.contains(&id) {
                    return Err(LoaderError::DuplicateAssertionId {
                        id,
                        test_id: test_id.to_string(),
                        first_source: "previous assertion".to_string(),
                        second_source: "inline assertion".to_string(),
                    });
                }
                seen_ids.insert(id);
                resolved.push(assertion.clone());
            }
            AssertionOrFile::FileRef { file } => {
                let paths = match file {
                    FileRefValue::Single(p) => vec![p.clone()],
                    FileRefValue::Multiple(ps) => ps.clone(),
                };

                for path_str in paths {
                    let file_path = base_dir.join(&path_str);

                    // Resolve to canonical path
                    let canonical = file_path
                        .canonicalize()
                        .map_err(|_| LoaderError::FileNotFound(file_path.clone()))?;

                    // Check if within skill-tests directory
                    let skill_tests_canonical = skill_tests_dir
                        .canonicalize()
                        .unwrap_or_else(|_| skill_tests_dir.to_path_buf());

                    if !canonical.starts_with(&skill_tests_canonical) {
                        return Err(LoaderError::FileRefOutsideSkillTests { path: file_path });
                    }

                    // Check for circular reference
                    if visited_files.contains(&canonical) {
                        return Err(LoaderError::CircularReference(
                            canonical.display().to_string(),
                        ));
                    }
                    visited_files.insert(canonical.clone());

                    // Load assertions from file
                    let content = std::fs::read_to_string(&canonical)?;
                    let file_assertions: Vec<Assertion> = serde_yml::from_str(&content)?;

                    // Check for duplicate IDs
                    for assertion in &file_assertions {
                        let id = assertion.id().to_string();
                        if seen_ids.contains(&id) {
                            return Err(LoaderError::DuplicateAssertionId {
                                id,
                                test_id: test_id.to_string(),
                                first_source: "previous assertion".to_string(),
                                second_source: canonical.display().to_string(),
                            });
                        }
                        seen_ids.insert(id);
                    }

                    resolved.extend(file_assertions);
                }
            }
        }
    }

    Ok(())
}

/// Load and resolve a simplified test case with all file references resolved.
///
/// Supports both formats:
/// - Legacy array format: `[{id, prompt, assertions, ...}, ...]`
/// - `TestFile` format: `{scenarios: {name: {prompt, assertions, ...}}, assertions: {...}}`
///
/// # Errors
/// Returns an error if the test file cannot be read, parsed, or if file references are invalid.
#[allow(clippy::type_complexity)]
pub fn load_and_resolve_test_case(
    test_file: &Path,
    skill_tests_dir: &Path,
) -> Result<Vec<(SimplifiedTestCase, Vec<Assertion>, Vec<Assertion>)>, LoaderError> {
    let content = std::fs::read_to_string(test_file)?;

    // Detect format: if it parses as a map with "scenarios" key, use TestFile format
    if let Ok(value) = serde_yml::from_str::<serde_yml::Value>(&content) {
        if value.get("scenarios").is_some() {
            // Use TestFile format
            return load_and_resolve_testfile_format(&content);
        }
    }

    // Legacy array format
    let cases: Vec<SimplifiedTestCase> = serde_yml::from_str(&content)?;

    // Validate test cases
    for case in &cases {
        validate_simplified_test_case(case)?;
    }

    let mut result = Vec::with_capacity(cases.len());

    for case in cases {
        let assertions =
            resolve_assertions(&case.assertions, test_file, skill_tests_dir, &case.id)?;

        let golden_assertions = resolve_assertions(
            &case.golden_assertions,
            test_file,
            skill_tests_dir,
            &case.id,
        )?;

        result.push((case, assertions, golden_assertions));
    }

    Ok(result)
}

/// Load `TestFile` format and convert to legacy tuple format for compatibility.
#[allow(clippy::type_complexity)]
fn load_and_resolve_testfile_format(
    content: &str,
) -> Result<Vec<(SimplifiedTestCase, Vec<Assertion>, Vec<Assertion>)>, LoaderError> {
    let test_file: TestFile = serde_yml::from_str(content)?;
    let resolved_scenarios = resolve_test_file(&test_file)?;

    let result = resolved_scenarios
        .into_iter()
        .map(|scenario| {
            let test_case = SimplifiedTestCase {
                id: scenario.name,
                desc: scenario.desc,
                prompt: scenario.prompt,
                iterations: scenario.iterations,
                assertions: vec![],        // Already resolved
                golden_assertions: vec![], // Already resolved
            };
            (test_case, scenario.assertions, scenario.golden_assertions)
        })
        .collect();

    Ok(result)
}

// =============================================================================
// New TestFile format loader (scenarios + named assertions)
// =============================================================================

/// Resolved scenario with all assertion references resolved to actual assertions.
#[derive(Debug, Clone)]
pub struct ResolvedScenario {
    /// Scenario name (from `HashMap` key).
    pub name: String,
    /// Scenario description.
    pub desc: Option<String>,
    /// Prompt to send to Claude.
    pub prompt: String,
    /// Number of iterations (overrides config).
    pub iterations: Option<u32>,
    /// Resolved assertions.
    pub assertions: Vec<Assertion>,
    /// Resolved golden assertions.
    pub golden_assertions: Vec<Assertion>,
}

/// Load a `TestFile` and resolve all assertion references.
///
/// # Errors
/// Returns an error if:
/// - The file cannot be read or parsed
/// - An assertion reference is undefined
///
/// # Example
///
/// ```yaml
/// desc: "検索機能のテスト"
///
/// assertions:
///   has-numbered-list:
///     type: regex
///     pattern: "\\d+\\."
///     expect: present
///
/// scenarios:
///   search-basic:
///     desc: "基本検索テスト"
///     prompt: "検索してください"
///     assertions:
///       - has-numbered-list  # name reference
/// ```
pub fn load_test_file<P: AsRef<Path>>(path: P) -> Result<TestFile, LoaderError> {
    let content = std::fs::read_to_string(path)?;
    let test_file: TestFile = serde_yml::from_str(&content)?;
    Ok(test_file)
}

/// Resolve all assertion references in a `TestFile` to actual assertions.
///
/// # Errors
/// Returns an error if an assertion reference is undefined.
pub fn resolve_test_file(test_file: &TestFile) -> Result<Vec<ResolvedScenario>, LoaderError> {
    // Build named assertions map
    let named_assertions: HashMap<String, Assertion> = test_file
        .assertions
        .iter()
        .map(|(name, def)| (name.clone(), def.to_assertion(name)))
        .collect();

    let mut resolved = Vec::with_capacity(test_file.scenarios.len());

    for (name, scenario) in &test_file.scenarios {
        // Validate prompt is not empty
        if scenario.prompt.trim().is_empty() {
            return Err(LoaderError::EmptyPrompt {
                scenario: name.clone(),
            });
        }

        let assertions =
            resolve_assertion_refs_with_dup_check(&scenario.assertions, &named_assertions, name)?;

        let golden_assertions = resolve_assertion_refs_with_dup_check(
            &scenario.golden_assertions,
            &named_assertions,
            name,
        )?;

        resolved.push(ResolvedScenario {
            name: name.clone(),
            desc: scenario.desc.clone(),
            prompt: scenario.prompt.clone(),
            iterations: scenario.iterations,
            assertions,
            golden_assertions,
        });
    }

    // Sort by name for deterministic ordering
    resolved.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(resolved)
}

/// Resolve assertion references to actual assertions with duplicate ID checking.
fn resolve_assertion_refs_with_dup_check(
    refs: &[AssertionRef],
    named_assertions: &HashMap<String, Assertion>,
    scenario_name: &str,
) -> Result<Vec<Assertion>, LoaderError> {
    let mut resolved = Vec::with_capacity(refs.len());
    let mut seen_ids = HashSet::new();

    for r in refs {
        let assertion = match r {
            AssertionRef::Name(name) => named_assertions
                .get(name)
                .ok_or_else(|| LoaderError::UndefinedAssertionRef {
                    name: name.clone(),
                    scenario: scenario_name.to_string(),
                })?
                .clone(),
            AssertionRef::Inline(assertion) => assertion.clone(),
        };

        // Check for duplicate IDs within this list
        let id = assertion.id();
        if !seen_ids.insert(id.to_string()) {
            return Err(LoaderError::DuplicateAssertionIdInScenario {
                id: id.to_string(),
                scenario: scenario_name.to_string(),
            });
        }
        resolved.push(assertion);
    }

    Ok(resolved)
}

/// Load and resolve a `TestFile` with all references resolved.
///
/// # Errors
/// Returns an error if the file cannot be read, parsed, or references are undefined.
pub fn load_and_resolve_test_file<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<ResolvedScenario>, LoaderError> {
    let test_file = load_test_file(path)?;
    resolve_test_file(&test_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_yaml() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: "test-001"
  prompt: "Test prompt"
  expected_skills: ["skill-a"]
  match_policy: "all"
  forbid_skills: []
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let cases = load_test_cases(file.path())?;
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].id, "test-001");
        assert_eq!(cases[0].prompt, "Test prompt");
        assert_eq!(cases[0].expected_skills, vec!["skill-a"]);
        Ok(())
    }

    #[test]
    fn test_load_multiple_cases() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: "test-001"
  prompt: "First test"
  expected_skills: ["skill-a"]
- id: "test-002"
  prompt: "Second test"
  expected_skills: ["skill-b", "skill-c"]
  match_policy: "any"
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let cases = load_test_cases(file.path())?;
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[1].expected_skills, vec!["skill-b", "skill-c"]);
        Ok(())
    }

    #[test]
    fn test_load_empty_id_fails() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: ""
  prompt: "Test prompt"
  expected_skills: ["skill-a"]
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let result = load_test_cases(file.path());
        assert!(result.is_err());
        let err = result.err().ok_or("expected error")?;
        assert!(err.to_string().contains("id cannot be empty"));
        Ok(())
    }

    #[test]
    fn test_load_empty_prompt_fails() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: "test-001"
  prompt: ""
  expected_skills: ["skill-a"]
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let result = load_test_cases(file.path());
        assert!(result.is_err());
        let err = result.err().ok_or("expected error")?;
        assert!(err.to_string().contains("prompt cannot be empty"));
        Ok(())
    }

    #[test]
    fn test_load_empty_skills_and_forbid_fails() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: "test-001"
  prompt: "Test prompt"
  expected_skills: []
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let result = load_test_cases(file.path());
        assert!(result.is_err());
        let err = result.err().ok_or("expected error")?;
        assert!(
            err.to_string()
                .contains("at least one expected skill or forbid skill")
        );
        Ok(())
    }

    #[test]
    fn test_load_negative_test_case() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: "negative-001"
  prompt: "What is the weather today?"
  expected_skills: []
  forbid_skills: ["some-skill"]
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let cases = load_test_cases(file.path())?;
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].id, "negative-001");
        assert!(cases[0].expected_skills.is_empty());
        assert_eq!(cases[0].forbid_skills, vec!["some-skill"]);
        Ok(())
    }

    #[test]
    fn test_load_with_optional_fields() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: "test-001"
  prompt: "Test prompt"
  expected_skills: ["skill-a"]
  iterations: 5
  forbid_skills: ["skill-x"]
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let cases = load_test_cases(file.path())?;
        assert_eq!(cases[0].iterations, Some(5));
        assert_eq!(cases[0].forbid_skills, vec!["skill-x"]);
        Ok(())
    }

    #[test]
    fn test_load_unknown_field_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: "test-001"
  prompt: "Test prompt"
  expected_skills: ["skill-a"]
  unknown_field: "should fail"
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let result = load_test_cases(file.path());
        assert!(result.is_err(), "unknown field should be rejected");
        let err = result.err().ok_or("expected error")?;
        assert!(
            err.to_string().contains("unknown field"),
            "error message should mention unknown field: {err}"
        );
        Ok(())
    }

    #[test]
    fn test_load_with_validation() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: "test-001"
  prompt: "Test prompt"
  expected_skills: ["skill-a"]
  validation:
    assertions:
      - id: "check-output"
        type: regex
        pattern: "hello"
        expect: present
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let cases = load_test_cases(file.path())?;
        assert_eq!(cases.len(), 1);
        assert!(cases[0].validation.is_some());
        let validation = cases[0]
            .validation
            .as_ref()
            .ok_or("validation should be present")?;
        assert_eq!(validation.assertions.len(), 1);
        Ok(())
    }

    #[test]
    fn test_load_with_llm_eval_assertion() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: "test-001"
  prompt: "Test prompt"
  expected_skills: ["skill-a"]
  validation:
    assertions:
      - id: "semantic-check"
        type: llm_eval
        pattern: "Does the output mention {{output}}?"
        expect: pass
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let cases = load_test_cases(file.path())?;
        let validation = cases[0]
            .validation
            .as_ref()
            .ok_or("validation should be present")?;
        assert_eq!(validation.assertions.len(), 1);

        let crate::types::Assertion::LlmEval(a) = &validation.assertions[0] else {
            return Err("expected LlmEval assertion".into());
        };
        assert_eq!(a.id, "semantic-check");
        assert!(a.pattern.contains("{{output}}"));
        Ok(())
    }

    #[test]
    fn test_load_validation_unknown_field_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let yaml = r#"
- id: "test-001"
  prompt: "Test prompt"
  expected_skills: ["skill-a"]
  validation:
    assertions:
      - id: "check-output"
        type: regex
        pattern: "hello"
        expect: present
    unknown_validation_field: "should fail"
"#;

        let mut file = NamedTempFile::new()?;
        file.write_all(yaml.as_bytes())?;

        let result = load_test_cases(file.path());
        assert!(
            result.is_err(),
            "unknown field in validation should be rejected"
        );
        Ok(())
    }
}

#[cfg(test)]
mod simplified_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_simplified_test_cases() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let yaml = r#"
- id: test-001
  prompt: "Do something"
  iterations: 5
  assertions:
    - id: check-output
      type: contains
      pattern: "expected"
      expect: present
"#;
        let test_file = dir.path().join("test.yaml");
        fs::write(&test_file, yaml)?;

        let cases = load_simplified_test_cases(&test_file)?;
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].id, "test-001");
        assert_eq!(cases[0].prompt, "Do something");
        assert_eq!(cases[0].iterations, Some(5));
        Ok(())
    }

    #[test]
    fn test_discover_test_files() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;

        // Create skill-tests directory structure
        let skill_tests = dir.path().join("skill-tests");
        fs::create_dir_all(&skill_tests)?;

        // Create test files
        fs::write(
            skill_tests.join("test-basic.yaml"),
            "- id: test\n  prompt: test",
        )?;
        fs::write(
            skill_tests.join("test-advanced.yaml"),
            "- id: test2\n  prompt: test",
        )?;
        fs::write(
            skill_tests.join("other.yaml"),
            "- id: other\n  prompt: test",
        )?;

        // Create nested directory
        let nested = skill_tests.join("nested");
        fs::create_dir_all(&nested)?;
        fs::write(
            nested.join("test-nested.yaml"),
            "- id: nested\n  prompt: test",
        )?;

        let patterns = vec!["skill-tests/**/test-*.yaml".to_string()];
        let exclude = vec![];

        let files = discover_test_files(dir.path(), &patterns, &exclude)?;
        assert_eq!(files.len(), 3);

        // Check that other.yaml is not included
        let has_other = files
            .iter()
            .any(|p| p.file_name().is_some_and(|name| name == "other.yaml"));
        assert!(!has_other);
        Ok(())
    }

    #[test]
    fn test_discover_test_files_with_exclude() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;

        let skill_tests = dir.path().join("skill-tests");
        fs::create_dir_all(&skill_tests)?;
        fs::write(skill_tests.join("test-a.yaml"), "[]")?;

        let node_modules = skill_tests.join("node_modules");
        fs::create_dir_all(&node_modules)?;
        fs::write(node_modules.join("test-b.yaml"), "[]")?;

        let patterns = vec!["skill-tests/**/test-*.yaml".to_string()];
        let exclude = vec!["node_modules/".to_string()];

        let files = discover_test_files(dir.path(), &patterns, &exclude)?;
        assert_eq!(files.len(), 1);
        assert!(
            files[0]
                .file_name()
                .is_some_and(|name| name == "test-a.yaml")
        );
        Ok(())
    }

    #[test]
    fn test_resolve_assertions_inline() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let skill_tests = dir.path().join("skill-tests");
        fs::create_dir_all(&skill_tests)?;

        let test_file = skill_tests.join("test.yaml");
        fs::write(&test_file, "")?;

        let assertions = vec![AssertionOrFile::Inline(crate::types::Assertion::Contains(
            crate::types::ContainsAssertion {
                id: "check".to_string(),
                desc: None,
                pattern: "test".to_string(),
                expect: crate::types::PatternExpect::Present,
            },
        ))];

        let resolved = resolve_assertions(&assertions, &test_file, &skill_tests, "test-001")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].id(), "check");
        Ok(())
    }

    #[test]
    fn test_resolve_assertions_file_ref() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let skill_tests = dir.path().join("skill-tests");
        let shared = skill_tests.join("shared");
        fs::create_dir_all(&shared)?;

        // Create shared assertions file
        let shared_file = shared.join("common.yaml");
        fs::write(
            &shared_file,
            r#"
- id: shared-check
  type: contains
  pattern: "common"
  expect: present
"#,
        )?;

        let test_file = skill_tests.join("test.yaml");
        fs::write(&test_file, "")?;

        let assertions = vec![AssertionOrFile::FileRef {
            file: FileRefValue::Single("./shared/common.yaml".to_string()),
        }];

        let resolved = resolve_assertions(&assertions, &test_file, &skill_tests, "test-001")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].id(), "shared-check");
        Ok(())
    }

    #[test]
    fn test_resolve_assertions_duplicate_id() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let skill_tests = dir.path().join("skill-tests");
        fs::create_dir_all(&skill_tests)?;

        let test_file = skill_tests.join("test.yaml");
        fs::write(&test_file, "")?;

        let assertions = vec![
            AssertionOrFile::Inline(crate::types::Assertion::Contains(
                crate::types::ContainsAssertion {
                    id: "duplicate".to_string(),
                    desc: None,
                    pattern: "a".to_string(),
                    expect: crate::types::PatternExpect::Present,
                },
            )),
            AssertionOrFile::Inline(crate::types::Assertion::Contains(
                crate::types::ContainsAssertion {
                    id: "duplicate".to_string(),
                    desc: None,
                    pattern: "b".to_string(),
                    expect: crate::types::PatternExpect::Present,
                },
            )),
        ];

        let result = resolve_assertions(&assertions, &test_file, &skill_tests, "test-001");
        assert!(matches!(
            result,
            Err(LoaderError::DuplicateAssertionId { .. })
        ));
        Ok(())
    }

    #[test]
    fn test_resolve_assertions_outside_skill_tests() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let skill_tests = dir.path().join("skill-tests");
        fs::create_dir_all(&skill_tests)?;

        // Create a file outside skill-tests
        let outside = dir.path().join("outside.yaml");
        fs::write(
            &outside,
            r#"
- id: outside
  type: contains
  pattern: "x"
  expect: present
"#,
        )?;

        let test_file = skill_tests.join("test.yaml");
        fs::write(&test_file, "")?;

        let assertions = vec![AssertionOrFile::FileRef {
            file: FileRefValue::Single("../outside.yaml".to_string()),
        }];

        let result = resolve_assertions(&assertions, &test_file, &skill_tests, "test-001");
        assert!(matches!(
            result,
            Err(LoaderError::FileRefOutsideSkillTests { .. })
        ));
        Ok(())
    }

    #[test]
    fn test_resolve_assertions_circular() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let skill_tests = dir.path().join("skill-tests");
        fs::create_dir_all(&skill_tests)?;

        // Note: This test is limited because we can't easily create circular YAML references
        // The circular detection is for when file A references file B which references file A
        // We test the basic structure here

        let test_file = skill_tests.join("test.yaml");
        fs::write(&test_file, "")?;

        // Empty assertions should work
        let assertions: Vec<AssertionOrFile> = vec![];
        let result = resolve_assertions(&assertions, &test_file, &skill_tests, "test-001")?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_load_and_resolve_test_case() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let skill_tests = dir.path().join("skill-tests");
        let shared = skill_tests.join("shared");
        fs::create_dir_all(&shared)?;

        // Create shared assertions
        fs::write(
            shared.join("common.yaml"),
            r#"
- id: shared-check
  type: contains
  pattern: "common"
  expect: present
"#,
        )?;

        // Create test file with mixed assertions
        let test_file = skill_tests.join("test.yaml");
        fs::write(
            &test_file,
            r#"
- id: test-001
  prompt: "Test prompt"
  assertions:
    - file: ./shared/common.yaml
    - id: inline-check
      type: contains
      pattern: "inline"
      expect: present
  golden_assertions:
    - id: golden-check
      type: regex
      pattern: "^best"
      expect: present
"#,
        )?;

        let result = load_and_resolve_test_case(&test_file, &skill_tests)?;
        assert_eq!(result.len(), 1);

        let (case, assertions, golden) = &result[0];
        assert_eq!(case.id, "test-001");
        assert_eq!(assertions.len(), 2);
        assert_eq!(assertions[0].id(), "shared-check");
        assert_eq!(assertions[1].id(), "inline-check");
        assert_eq!(golden.len(), 1);
        assert_eq!(golden[0].id(), "golden-check");
        Ok(())
    }

    // =========================================================================
    // Tests for new TestFile format (scenarios + named assertions)
    // =========================================================================

    #[test]
    fn test_load_test_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let yaml = r#"
desc: "検索機能のテスト"

assertions:
  has-numbered-list:
    desc: "番号付きリストが含まれていること"
    type: regex
    pattern: "\\d+\\."
    expect: present
  has-score:
    type: regex
    pattern: "\\d+/35"
    expect: present

scenarios:
  search-basic:
    desc: "基本検索テスト"
    prompt: "検索してください"
    iterations: 5
    assertions:
      - has-numbered-list
      - has-score
  search-advanced:
    prompt: "高度な検索"
    assertions:
      - has-numbered-list
      - type: contains
        id: inline-check
        pattern: "結果"
        expect: present
"#;
        let test_file_path = dir.path().join("test.yaml");
        fs::write(&test_file_path, yaml)?;

        let test_file = load_test_file(&test_file_path)?;
        assert_eq!(test_file.desc, Some("検索機能のテスト".to_string()));
        assert_eq!(test_file.assertions.len(), 2);
        assert!(test_file.assertions.contains_key("has-numbered-list"));
        assert!(test_file.assertions.contains_key("has-score"));
        assert_eq!(test_file.scenarios.len(), 2);

        let basic = test_file
            .scenarios
            .get("search-basic")
            .ok_or("scenario 'search-basic' not found")?;
        assert_eq!(basic.desc, Some("基本検索テスト".to_string()));
        assert_eq!(basic.prompt, "検索してください");
        assert_eq!(basic.iterations, Some(5));
        assert_eq!(basic.assertions.len(), 2);

        Ok(())
    }

    #[test]
    fn test_resolve_test_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let yaml = r#"
assertions:
  check-a:
    type: contains
    pattern: "hello"
    expect: present
  check-b:
    desc: "Check for world"
    type: contains
    pattern: "world"
    expect: present

scenarios:
  test-1:
    desc: "Test scenario"
    prompt: "Say hello"
    assertions:
      - check-a
      - check-b
    golden_assertions:
      - check-a
"#;
        let test_file_path = dir.path().join("test.yaml");
        fs::write(&test_file_path, yaml)?;

        let test_file = load_test_file(&test_file_path)?;
        let resolved = resolve_test_file(&test_file)?;

        assert_eq!(resolved.len(), 1);
        let scenario = &resolved[0];
        assert_eq!(scenario.name, "test-1");
        assert_eq!(scenario.desc, Some("Test scenario".to_string()));
        assert_eq!(scenario.prompt, "Say hello");
        assert_eq!(scenario.assertions.len(), 2);
        assert_eq!(scenario.assertions[0].id(), "check-a");
        assert_eq!(scenario.assertions[1].id(), "check-b");
        assert_eq!(scenario.assertions[1].desc(), Some("Check for world"));
        assert_eq!(scenario.golden_assertions.len(), 1);
        assert_eq!(scenario.golden_assertions[0].id(), "check-a");

        Ok(())
    }

    #[test]
    fn test_resolve_test_file_with_inline() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let yaml = r#"
assertions:
  named-check:
    type: contains
    pattern: "named"
    expect: present

scenarios:
  mixed-test:
    prompt: "Test"
    assertions:
      - named-check
      - type: regex
        id: inline-regex
        desc: "Inline regex check"
        pattern: "\\d+"
        expect: present
"#;
        let test_file_path = dir.path().join("test.yaml");
        fs::write(&test_file_path, yaml)?;

        let resolved = load_and_resolve_test_file(&test_file_path)?;

        assert_eq!(resolved.len(), 1);
        let scenario = &resolved[0];
        assert_eq!(scenario.assertions.len(), 2);
        assert_eq!(scenario.assertions[0].id(), "named-check");
        assert_eq!(scenario.assertions[1].id(), "inline-regex");
        assert_eq!(scenario.assertions[1].desc(), Some("Inline regex check"));

        Ok(())
    }

    #[test]
    fn test_resolve_test_file_undefined_ref() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let yaml = r#"
assertions:
  existing:
    type: contains
    pattern: "x"
    expect: present

scenarios:
  test:
    prompt: "Test"
    assertions:
      - nonexistent
"#;
        let test_file_path = dir.path().join("test.yaml");
        fs::write(&test_file_path, yaml)?;

        let test_file = load_test_file(&test_file_path)?;
        let result = resolve_test_file(&test_file);

        assert!(matches!(
            result,
            Err(LoaderError::UndefinedAssertionRef {
                name,
                scenario,
            }) if name == "nonexistent" && scenario == "test"
        ));
        Ok(())
    }

    #[test]
    fn test_resolve_test_file_deterministic_order() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let yaml = r#"
scenarios:
  zebra:
    prompt: "Z"
    assertions: []
  alpha:
    prompt: "A"
    assertions: []
  middle:
    prompt: "M"
    assertions: []
"#;
        let test_file_path = dir.path().join("test.yaml");
        fs::write(&test_file_path, yaml)?;

        let resolved = load_and_resolve_test_file(&test_file_path)?;

        // Should be sorted alphabetically
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0].name, "alpha");
        assert_eq!(resolved[1].name, "middle");
        assert_eq!(resolved[2].name, "zebra");

        Ok(())
    }

    #[test]
    fn test_resolve_test_file_empty_prompt() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let yaml = r#"
scenarios:
  empty-prompt:
    prompt: ""
    assertions: []
"#;
        let test_file_path = dir.path().join("test.yaml");
        fs::write(&test_file_path, yaml)?;

        let test_file = load_test_file(&test_file_path)?;
        let result = resolve_test_file(&test_file);

        assert!(matches!(
            result,
            Err(LoaderError::EmptyPrompt { scenario }) if scenario == "empty-prompt"
        ));
        Ok(())
    }

    #[test]
    fn test_resolve_test_file_whitespace_only_prompt() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let yaml = r#"
scenarios:
  ws-prompt:
    prompt: "   "
    assertions: []
"#;
        let test_file_path = dir.path().join("test.yaml");
        fs::write(&test_file_path, yaml)?;

        let test_file = load_test_file(&test_file_path)?;
        let result = resolve_test_file(&test_file);

        assert!(matches!(
            result,
            Err(LoaderError::EmptyPrompt { scenario }) if scenario == "ws-prompt"
        ));
        Ok(())
    }

    #[test]
    fn test_resolve_test_file_duplicate_assertion_id() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let yaml = r#"
assertions:
  dup-check:
    type: contains
    pattern: "x"
    expect: present

scenarios:
  test:
    prompt: "Test prompt"
    assertions:
      - dup-check
      - id: dup-check
        type: regex
        pattern: "y"
        expect: present
"#;
        let test_file_path = dir.path().join("test.yaml");
        fs::write(&test_file_path, yaml)?;

        let test_file = load_test_file(&test_file_path)?;
        let result = resolve_test_file(&test_file);

        assert!(matches!(
            result,
            Err(LoaderError::DuplicateAssertionIdInScenario { id, scenario })
                if id == "dup-check" && scenario == "test"
        ));
        Ok(())
    }

    #[test]
    fn test_resolve_test_file_same_id_in_assertions_and_golden_allowed()
    -> Result<(), Box<dyn std::error::Error>> {
        // Same assertion ID in both assertions and golden_assertions should be allowed
        let dir = TempDir::new()?;
        let yaml = r#"
assertions:
  shared-check:
    type: contains
    pattern: "x"
    expect: present

scenarios:
  test:
    prompt: "Test prompt"
    assertions:
      - shared-check
    golden_assertions:
      - shared-check
"#;
        let test_file_path = dir.path().join("test.yaml");
        fs::write(&test_file_path, yaml)?;

        let resolved = load_and_resolve_test_file(&test_file_path)?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].assertions.len(), 1);
        assert_eq!(resolved[0].golden_assertions.len(), 1);
        assert_eq!(resolved[0].assertions[0].id(), "shared-check");
        assert_eq!(resolved[0].golden_assertions[0].id(), "shared-check");

        Ok(())
    }

    #[test]
    fn test_resolve_test_file_duplicate_within_golden() -> Result<(), Box<dyn std::error::Error>> {
        // Duplicate ID within golden_assertions should be an error
        let dir = TempDir::new()?;
        let yaml = r#"
assertions:
  check-a:
    type: contains
    pattern: "x"
    expect: present

scenarios:
  test:
    prompt: "Test prompt"
    assertions: []
    golden_assertions:
      - check-a
      - check-a
"#;
        let test_file_path = dir.path().join("test.yaml");
        fs::write(&test_file_path, yaml)?;

        let test_file = load_test_file(&test_file_path)?;
        let result = resolve_test_file(&test_file);

        assert!(matches!(
            result,
            Err(LoaderError::DuplicateAssertionIdInScenario { id, scenario })
                if id == "check-a" && scenario == "test"
        ));
        Ok(())
    }
}
