//! Open Issues report

use miette::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tabled::{builder::Builder, settings::Style};

use crate::cli::helpers::truncate_str;
use crate::cli::GlobalOpts;
use crate::core::project::Project;
use crate::core::shortid::ShortIdIndex;
use crate::entities::result::{Result as TestResult, Verdict};

use super::{load_all_capas, load_all_ncrs, load_all_results, load_all_tests, write_output};

#[derive(clap::Args, Debug)]
pub struct OpenIssuesArgs {
    /// Output to file instead of stdout
    #[arg(long, short = 'o')]
    pub output: Option<PathBuf>,
}

pub fn run(args: OpenIssuesArgs, _global: &GlobalOpts) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;
    let short_ids = ShortIdIndex::load(&project);

    // Load NCRs
    let ncrs = load_all_ncrs(&project);
    let open_ncrs: Vec<_> = ncrs
        .iter()
        .filter(|n| n.ncr_status != crate::entities::ncr::NcrStatus::Closed)
        .collect();

    // Load CAPAs
    let capas = load_all_capas(&project);
    let open_capas: Vec<_> = capas
        .iter()
        .filter(|c| c.capa_status != crate::entities::capa::CapaStatus::Closed)
        .collect();

    // Load test failures
    let tests = load_all_tests(&project);
    let results = load_all_results(&project);
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

    let failed_tests: Vec<_> = tests
        .iter()
        .filter(|t| {
            latest_results
                .get(&t.id.to_string())
                .is_some_and(|r| r.verdict == Verdict::Fail)
        })
        .collect();

    // Generate report
    let mut output = String::new();
    output.push_str("# Open Issues Report\n\n");

    // Summary
    output.push_str("## Summary\n\n");
    let mut summary = Builder::default();
    summary.push_record(["Category", "Count"]);
    summary.push_record(["Open NCRs", &open_ncrs.len().to_string()]);
    summary.push_record(["Open CAPAs", &open_capas.len().to_string()]);
    summary.push_record(["Failed Tests", &failed_tests.len().to_string()]);
    output.push_str(&summary.build().with(Style::markdown()).to_string());

    // Open NCRs
    if !open_ncrs.is_empty() {
        output.push_str("\n## Open NCRs\n\n");
        let mut ncr_table = Builder::default();
        ncr_table.push_record(["ID", "Title", "Severity", "Status"]);
        for ncr in &open_ncrs {
            let ncr_short = short_ids
                .get_short_id(&ncr.id.to_string())
                .unwrap_or_else(|| ncr.id.to_string());
            ncr_table.push_record([
                ncr_short,
                truncate_str(&ncr.title, 30),
                ncr.severity.to_string(),
                ncr.ncr_status.to_string(),
            ]);
        }
        output.push_str(&ncr_table.build().with(Style::markdown()).to_string());
    }

    // Open CAPAs
    if !open_capas.is_empty() {
        output.push_str("\n## Open CAPAs\n\n");
        let mut capa_table = Builder::default();
        capa_table.push_record(["ID", "Title", "Type", "Status"]);
        for capa in &open_capas {
            let capa_short = short_ids
                .get_short_id(&capa.id.to_string())
                .unwrap_or_else(|| capa.id.to_string());
            capa_table.push_record([
                capa_short,
                truncate_str(&capa.title, 30),
                capa.capa_type.to_string(),
                capa.capa_status.to_string(),
            ]);
        }
        output.push_str(&capa_table.build().with(Style::markdown()).to_string());
    }

    // Failed Tests
    if !failed_tests.is_empty() {
        output.push_str("\n## Failed Tests\n\n");
        let mut test_table = Builder::default();
        test_table.push_record(["ID", "Title", "Type"]);
        for test in &failed_tests {
            let test_short = short_ids
                .get_short_id(&test.id.to_string())
                .unwrap_or_else(|| test.id.to_string());
            test_table.push_record([
                test_short,
                truncate_str(&test.title, 40),
                test.test_type.to_string(),
            ]);
        }
        output.push_str(&test_table.build().with(Style::markdown()).to_string());
    }

    write_output(&output, args.output)?;
    Ok(())
}
