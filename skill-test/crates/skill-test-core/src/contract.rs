//! Contract loader and merger.

use crate::types::{Assertion, Contract, MergedContract, ValidationError};
use std::collections::HashSet;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during contract loading.
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yml::Error),
    #[error("common.yaml not found at {0}")]
    CommonNotFound(String),
    #[error("contract not found for skill '{0}' (strict mode)")]
    SkillContractNotFound(String),
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),
}

/// Load contracts from a directory.
///
/// # Arguments
/// * `dir` - Path to the contracts directory
/// * `skills` - List of skills to load contracts for
/// * `strict` - If true, error on missing skill contracts; if false, warn and continue
///
/// # Returns
/// A merged contract containing all assertions.
///
/// # Errors
/// Returns an error if:
/// - `common.yaml` is not found
/// - YAML parsing fails
/// - Duplicate assertion IDs are found
/// - Strict mode is enabled and a skill contract is missing
pub fn load_contracts<P: AsRef<Path>>(
    dir: P,
    skills: &[String],
    strict: bool,
) -> Result<MergedContract, ContractError> {
    let dir = dir.as_ref();
    let common_path = dir.join("common.yaml");

    // Load common contract (required)
    if !common_path.exists() {
        return Err(ContractError::CommonNotFound(
            common_path.display().to_string(),
        ));
    }
    let common_content = std::fs::read_to_string(&common_path)?;
    let common: Contract = serde_yml::from_str(&common_content)?;

    let mut merged = MergedContract {
        assertions: common.assertions,
        golden_assertions: common.golden_assertions,
        warnings: Vec::new(),
    };

    // Track seen assertion IDs for duplicate detection
    let mut seen_ids: HashSet<String> = merged
        .assertions
        .iter()
        .map(|a| a.id().to_string())
        .collect();

    for id in merged.golden_assertions.iter().map(Assertion::id) {
        if !seen_ids.insert(id.to_string()) {
            return Err(ContractError::Validation(
                ValidationError::DuplicateAssertionId(id.to_string()),
            ));
        }
    }

    // Load skill-specific contracts
    let skills_dir = dir.join("skills");

    for skill in skills {
        let skill_path = skills_dir.join(format!("{skill}.yaml"));

        if !skill_path.exists() {
            if strict {
                return Err(ContractError::SkillContractNotFound(skill.clone()));
            }
            merged.warnings.push(skill.clone());
            continue;
        }

        let skill_content = std::fs::read_to_string(&skill_path)?;
        let skill_contract: Contract = serde_yml::from_str(&skill_content)?;

        // Check for duplicate assertion IDs
        for assertion in &skill_contract.assertions {
            let id = assertion.id();
            if !seen_ids.insert(id.to_string()) {
                return Err(ContractError::Validation(
                    ValidationError::DuplicateAssertionId(id.to_string()),
                ));
            }
        }

        for assertion in &skill_contract.golden_assertions {
            let id = assertion.id();
            if !seen_ids.insert(id.to_string()) {
                return Err(ContractError::Validation(
                    ValidationError::DuplicateAssertionId(id.to_string()),
                ));
            }
        }

        merged.assertions.extend(skill_contract.assertions);
        merged
            .golden_assertions
            .extend(skill_contract.golden_assertions);
    }

    Ok(merged)
}

/// Load contracts for skills that were actually called (for `match_policy`: any).
///
/// This only loads contracts for skills in `called_skills`, not all `expected_skills`.
///
/// # Errors
/// See `load_contracts` for error conditions.
pub fn load_contracts_for_called<P: AsRef<Path>>(
    dir: P,
    called_skills: &[String],
    strict: bool,
) -> Result<MergedContract, ContractError> {
    load_contracts(dir, called_skills, strict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_contracts(dir: &TempDir) -> std::io::Result<()> {
        let common = r#"
assertions:
  - id: "no-todo"
    type: "regex"
    pattern: "TODO|FIXME"
    expect: "absent"
"#;
        fs::write(dir.path().join("common.yaml"), common)?;

        fs::create_dir_all(dir.path().join("skills"))?;

        let skill_a = r#"
skill: "skill-a"
assertions:
  - id: "uses-a"
    type: "contains"
    pattern: "skill-a"
    expect: "present"
"#;
        fs::write(dir.path().join("skills/skill-a.yaml"), skill_a)?;
        Ok(())
    }

    #[test]
    fn test_load_common_only() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        create_test_contracts(&dir)?;

        let merged = load_contracts(dir.path(), &[], false)?;
        assert_eq!(merged.assertions.len(), 1);
        assert_eq!(merged.assertions[0].id(), "no-todo");
        Ok(())
    }

    #[test]
    fn test_load_with_skill() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        create_test_contracts(&dir)?;

        let merged = load_contracts(dir.path(), &["skill-a".to_string()], false)?;
        assert_eq!(merged.assertions.len(), 2);
        Ok(())
    }

    #[test]
    fn test_missing_common_fails() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;

        let result = load_contracts(dir.path(), &[], false);
        assert!(result.is_err());
        let err = result.err().ok_or("expected error")?;
        assert!(err.to_string().contains("common.yaml not found"));
        Ok(())
    }

    #[test]
    fn test_missing_skill_strict_fails() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        create_test_contracts(&dir)?;

        let result = load_contracts(dir.path(), &["nonexistent".to_string()], true);
        assert!(result.is_err());
        let err = result.err().ok_or("expected error")?;
        assert!(err.to_string().contains("contract not found"));
        Ok(())
    }

    #[test]
    fn test_missing_skill_non_strict_warns() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        create_test_contracts(&dir)?;

        let merged = load_contracts(dir.path(), &["nonexistent".to_string()], false)?;
        assert_eq!(merged.warnings, vec!["nonexistent"]);
        Ok(())
    }

    #[test]
    fn test_duplicate_id_fails() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;

        let common = r#"
assertions:
  - id: "duplicate-id"
    type: "regex"
    pattern: "test"
    expect: "present"
"#;
        fs::write(dir.path().join("common.yaml"), common)?;
        fs::create_dir_all(dir.path().join("skills"))?;

        let skill_a = r#"
skill: "skill-a"
assertions:
  - id: "duplicate-id"
    type: "contains"
    pattern: "test"
    expect: "present"
"#;
        fs::write(dir.path().join("skills/skill-a.yaml"), skill_a)?;

        let result = load_contracts(dir.path(), &["skill-a".to_string()], false);
        assert!(result.is_err());
        let err = result.err().ok_or("expected error")?;
        assert!(err.to_string().contains("duplicate assertion id"));
        Ok(())
    }

    #[test]
    fn test_golden_assertions() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;

        let common = r#"
assertions:
  - id: "required"
    type: "regex"
    pattern: "test"
    expect: "present"
golden_assertions:
  - id: "optional"
    type: "contains"
    pattern: "best-practice"
    expect: "present"
"#;
        fs::write(dir.path().join("common.yaml"), common)?;

        let merged = load_contracts(dir.path(), &[], false)?;
        assert_eq!(merged.assertions.len(), 1);
        assert_eq!(merged.golden_assertions.len(), 1);
        assert_eq!(merged.golden_assertions[0].id(), "optional");
        Ok(())
    }
}
