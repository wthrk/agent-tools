//! Report generation for test results.

use crate::types::{TestResult, Verdict};

/// Format for report output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportFormat {
    #[default]
    Table,
    Json,
}

impl std::str::FromStr for ReportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            _ => Err(format!("unknown format: {s}. Valid formats: table, json")),
        }
    }
}

/// Generate JSON report for test results.
///
/// # Panics
/// This function should not panic as `TestResult` implements `Serialize`.
#[must_use]
pub fn format_results_json(results: &[TestResult]) -> String {
    serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".to_string())
}

/// Format iteration output for verbose mode.
#[must_use]
pub fn format_iteration_output(
    index: usize,
    total: usize,
    test_id: &str,
    iteration: u32,
    verdict: Verdict,
    latency_ms: u64,
) -> String {
    let verdict_str = match verdict {
        Verdict::Pass => "Pass",
        Verdict::Fail => "Fail",
        Verdict::Warn => "Warn",
    };

    let latency_secs = f64::from(u32::try_from(latency_ms).unwrap_or(u32::MAX)) / 1000.0;
    format!("[{index}/{total}] {test_id} iter={iteration} ... {verdict_str} ({latency_secs:.1}s)")
}

/// Format test case summary.
#[must_use]
pub fn format_test_summary(test_id: &str, passed: u32, total: u32) -> String {
    let pass_rate = if total > 0 {
        (f64::from(passed) / f64::from(total)) * 100.0
    } else {
        0.0
    };

    format!("{test_id}: {passed}/{total} ({pass_rate:.0}%) Pass")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_iteration_output() {
        let output = format_iteration_output(1, 10, "test-001", 1, Verdict::Pass, 1500);

        assert!(output.contains("[1/10]"));
        assert!(output.contains("test-001"));
        assert!(output.contains("iter=1"));
        assert!(output.contains("Pass"));
        assert!(output.contains("1.5s"));
    }

    #[test]
    fn test_format_test_summary() {
        let summary = format_test_summary("test-001", 8, 10);

        assert!(summary.contains("test-001"));
        assert!(summary.contains("8/10"));
        assert!(summary.contains("80%"));
    }

    #[test]
    fn test_report_format_from_str() -> Result<(), String> {
        assert_eq!("table".parse::<ReportFormat>()?, ReportFormat::Table);
        assert_eq!("json".parse::<ReportFormat>()?, ReportFormat::Json);
        assert!("invalid".parse::<ReportFormat>().is_err());
        Ok(())
    }

    #[test]
    fn test_report_format_csv_removed() {
        // CSV format has been removed - should return error
        let result = "csv".parse::<ReportFormat>();
        assert!(result.is_err(), "csv format should return error");
        if let Err(e) = result {
            assert!(
                e.contains("unknown format"),
                "error should contain 'unknown format': {e}"
            );
        }
    }
}
