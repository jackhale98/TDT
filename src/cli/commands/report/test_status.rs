//! Test Status report

use miette::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tabled::{builder::Builder, settings::Style};

use crate::cli::helpers::{format_date_local, truncate_str};
use crate::cli::GlobalOpts;
use crate::core::project::Project;
use crate::core::shortid::ShortIdIndex;
use crate::entities::result::{Result as TestResult, Verdict};
use crate::entities::test::Test;

use super::{load_all_results, load_all_tests, write_output};

#[derive(clap::Args, Debug)]
pub struct TestStatusArgs {
    /// Output to file instead of stdout
    #[arg(long, short = 'o')]
    pub output: Option<PathBuf>,
}

pub fn run(args: TestStatusArgs, _global: &GlobalOpts) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;
    let short_ids = ShortIdIndex::load(&project);

    let tests = load_all_tests(&project);
    let results = load_all_results(&project);

    // Build latest result for each test
    let mut latest_results: HashMap<String, &TestResult> = HashMap::new();
    for result in &results {
        let test_id = result.test_id.to_string();
        if let Some(existing) = latest_results.get(&test_id) {
            if result.executed_date > existing.executed_date {
                latest_results.insert(test_id, result);
            }
        } else {
            latest_results.insert(test_id, result);
        }
    }

    // Categorize tests
    let mut executed = 0;
    let mut pending = 0;
    let mut passed = 0;
    let mut failed = 0;
    let mut conditional = 0;
    let mut recent_failures: Vec<(&Test, &TestResult)> = Vec::new();

    for test in &tests {
        let test_id = test.id.to_string();
        if let Some(result) = latest_results.get(&test_id) {
            executed += 1;
            match result.verdict {
                Verdict::Pass => passed += 1,
                Verdict::Fail => {
                    failed += 1;
                    recent_failures.push((test, result));
                }
                Verdict::Conditional => conditional += 1,
                Verdict::Incomplete | Verdict::NotApplicable => {}
            }
        } else {
            pending += 1;
        }
    }

    // Sort failures by date (most recent first)
    recent_failures.sort_by(|a, b| b.1.executed_date.cmp(&a.1.executed_date));
    recent_failures.truncate(10);

    // Generate report
    let mut output = String::new();
    output.push_str("# Test Execution Status Report\n\n");

    output.push_str("## Summary\n\n");
    let mut summary = Builder::default();
    summary.push_record(["Metric", "Count"]);
    summary.push_record(["Total Protocols", &tests.len().to_string()]);
    summary.push_record(["Executed", &executed.to_string()]);
    summary.push_record(["Pending", &pending.to_string()]);
    summary.push_record(["Passed", &passed.to_string()]);
    summary.push_record(["Failed", &failed.to_string()]);
    summary.push_record(["Conditional", &conditional.to_string()]);

    if executed > 0 {
        let pass_rate = (passed as f64 / executed as f64) * 100.0;
        summary.push_record(["Pass Rate", &format!("{:.1}%", pass_rate)]);
    }
    output.push_str(&summary.build().with(Style::markdown()).to_string());

    if !recent_failures.is_empty() {
        output.push_str("\n## Recent Failures\n\n");
        let mut failures = Builder::default();
        failures.push_record(["Test ID", "Title", "Execution Date"]);
        for (test, result) in &recent_failures {
            let test_short = short_ids
                .get_short_id(&test.id.to_string())
                .unwrap_or_else(|| test.id.to_string());
            failures.push_record([
                test_short,
                truncate_str(&test.title, 40),
                format_date_local(&result.executed_date),
            ]);
        }
        output.push_str(&failures.build().with(Style::markdown()).to_string());
    }

    write_output(&output, args.output)?;
    Ok(())
}
