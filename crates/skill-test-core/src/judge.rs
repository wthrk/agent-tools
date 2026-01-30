//! Verdict judgment based on truth table.

use crate::types::{ContractResult, JudgmentResult, MatchPolicy, Verdict};
use std::collections::HashSet;

/// Judge the test result based on skill matching and contract evaluation.
///
/// # Truth Table
/// | forbid_called | expected_matched | contract_passed | extra_called | → Verdict |
/// |---------------|------------------|-----------------|--------------|-----------|
/// | true          | *                | *               | *            | Fail      |
/// | false         | false            | *               | *            | Fail      |
/// | false         | true             | false           | *            | Fail      |
/// | false         | true             | true/null       | true         | Warn      |
/// | false         | true             | true/null       | false        | Pass      |
#[must_use]
pub fn judge(
    expected: &[String],
    forbid: &[String],
    called: &[String],
    match_policy: MatchPolicy,
    contract_result: Option<&ContractResult>,
) -> JudgmentResult {
    let called_set: HashSet<&str> = called.iter().map(String::as_str).collect();
    let expected_set: HashSet<&str> = expected.iter().map(String::as_str).collect();
    let forbid_set: HashSet<&str> = forbid.iter().map(String::as_str).collect();

    // 1. Check forbidden skills
    let forbidden_called: Vec<&str> = forbid_set.intersection(&called_set).copied().collect();
    if !forbidden_called.is_empty() {
        return JudgmentResult {
            verdict: Verdict::Fail,
            reason: format!("forbidden skill(s) called: {}", forbidden_called.join(", ")),
        };
    }

    // 2. Check expected skills based on match policy
    let expected_matched = match match_policy {
        MatchPolicy::All => expected_set.is_subset(&called_set),
        MatchPolicy::Any => !expected_set.is_disjoint(&called_set),
    };

    if !expected_matched {
        let missing: Vec<&str> = expected_set.difference(&called_set).copied().collect();
        return JudgmentResult {
            verdict: Verdict::Fail,
            reason: format!(
                "expected skill(s) not called: {} (policy: {:?})",
                missing.join(", "),
                match_policy
            ),
        };
    }

    // 3. Check contract assertions
    if let Some(result) = contract_result {
        if !result.contract_passed {
            return JudgmentResult {
                verdict: Verdict::Fail,
                reason: format!(
                    "contract assertion(s) failed: {}",
                    result.failures.join(", ")
                ),
            };
        }
    }

    // 4. Check for extra skills (warning only)
    let extra_skills: Vec<&str> = called_set.difference(&expected_set).copied().collect();
    if !extra_skills.is_empty() {
        return JudgmentResult {
            verdict: Verdict::Warn,
            reason: format!("unexpected skill(s) called: {}", extra_skills.join(", ")),
        };
    }

    // 5. All checks passed
    JudgmentResult {
        verdict: Verdict::Pass,
        reason: "all checks passed".to_string(),
    }
}

/// Check if skills match based on policy.
#[must_use]
pub fn skills_match(expected: &[String], called: &[String], policy: MatchPolicy) -> bool {
    let expected_set: HashSet<&str> = expected.iter().map(String::as_str).collect();
    let called_set: HashSet<&str> = called.iter().map(String::as_str).collect();

    match policy {
        MatchPolicy::All => expected_set.is_subset(&called_set),
        MatchPolicy::Any => !expected_set.is_disjoint(&called_set),
    }
}

/// Get skills that were called from expected skills.
#[must_use]
pub fn get_matched_skills(expected: &[String], called: &[String]) -> Vec<String> {
    let called_set: HashSet<&str> = called.iter().map(String::as_str).collect();

    expected
        .iter()
        .filter(|s| called_set.contains(s.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|&s| s.to_string()).collect()
    }

    #[test]
    fn test_pass_all_expected_called() {
        let result = judge(
            &s(&["skill-a"]),
            &[],
            &s(&["skill-a"]),
            MatchPolicy::All,
            None,
        );

        assert_eq!(result.verdict, Verdict::Pass);
    }

    #[test]
    fn test_fail_forbidden_called() {
        let result = judge(
            &s(&["skill-a"]),
            &s(&["skill-x"]),
            &s(&["skill-a", "skill-x"]),
            MatchPolicy::All,
            None,
        );

        assert_eq!(result.verdict, Verdict::Fail);
        assert!(result.reason.contains("forbidden"));
    }

    #[test]
    fn test_fail_expected_not_called() {
        let result = judge(
            &s(&["skill-a", "skill-b"]),
            &[],
            &s(&["skill-a"]),
            MatchPolicy::All,
            None,
        );

        assert_eq!(result.verdict, Verdict::Fail);
        assert!(result.reason.contains("not called"));
    }

    #[test]
    fn test_pass_any_policy() {
        let result = judge(
            &s(&["skill-a", "skill-b"]),
            &[],
            &s(&["skill-a"]),
            MatchPolicy::Any,
            None,
        );

        assert_eq!(result.verdict, Verdict::Pass);
    }

    #[test]
    fn test_fail_any_policy_none_called() {
        let result = judge(
            &s(&["skill-a", "skill-b"]),
            &[],
            &s(&["skill-c"]),
            MatchPolicy::Any,
            None,
        );

        assert_eq!(result.verdict, Verdict::Fail);
    }

    #[test]
    fn test_warn_extra_skills() {
        let result = judge(
            &s(&["skill-a"]),
            &[],
            &s(&["skill-a", "skill-b"]),
            MatchPolicy::All,
            None,
        );

        assert_eq!(result.verdict, Verdict::Warn);
        assert!(result.reason.contains("unexpected"));
    }

    #[test]
    fn test_fail_contract_failed() {
        let contract_result = ContractResult {
            contract_passed: false,
            golden_passed: None,
            details: vec![],
            failures: vec!["no-todo".to_string()],
            golden_failures: vec![],
        };

        let result = judge(
            &s(&["skill-a"]),
            &[],
            &s(&["skill-a"]),
            MatchPolicy::All,
            Some(&contract_result),
        );

        assert_eq!(result.verdict, Verdict::Fail);
        assert!(result.reason.contains("contract"));
    }

    #[test]
    fn test_pass_with_contract() {
        let contract_result = ContractResult {
            contract_passed: true,
            golden_passed: Some(true),
            details: vec![],
            failures: vec![],
            golden_failures: vec![],
        };

        let result = judge(
            &s(&["skill-a"]),
            &[],
            &s(&["skill-a"]),
            MatchPolicy::All,
            Some(&contract_result),
        );

        assert_eq!(result.verdict, Verdict::Pass);
    }

    #[test]
    fn test_skills_match_all() {
        assert!(skills_match(
            &s(&["a", "b"]),
            &s(&["a", "b", "c"]),
            MatchPolicy::All
        ));
        assert!(!skills_match(&s(&["a", "b"]), &s(&["a"]), MatchPolicy::All));
    }

    #[test]
    fn test_skills_match_any() {
        assert!(skills_match(&s(&["a", "b"]), &s(&["a"]), MatchPolicy::Any));
        assert!(!skills_match(&s(&["a", "b"]), &s(&["c"]), MatchPolicy::Any));
    }

    #[test]
    fn test_get_matched_skills() {
        let matched = get_matched_skills(&s(&["a", "b", "c"]), &s(&["a", "c", "d"]));
        assert_eq!(matched, vec!["a", "c"]);
    }
}
