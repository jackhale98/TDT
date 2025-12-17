//! FMEA (Failure Mode and Effects Analysis) report

use miette::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tabled::{builder::Builder, settings::Style};

use crate::cli::helpers::truncate_str;
use crate::cli::GlobalOpts;
use crate::core::project::Project;
use crate::core::shortid::ShortIdIndex;

use super::{load_all_risks, write_output};

#[derive(clap::Args, Debug)]
pub struct FmeaArgs {
    /// Output to file instead of stdout
    #[arg(long, short = 'o')]
    pub output: Option<PathBuf>,

    /// Minimum RPN to include (default: 0)
    #[arg(long, default_value = "0")]
    pub min_rpn: u16,

    /// Only show design risks
    #[arg(long)]
    pub design_only: bool,

    /// Only show process risks
    #[arg(long)]
    pub process_only: bool,
}

pub fn run(args: FmeaArgs, _global: &GlobalOpts) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;
    let short_ids = ShortIdIndex::load(&project);

    // Load all risks
    let mut risks = load_all_risks(&project);

    // Filter by type
    if args.design_only {
        risks.retain(|r| r.risk_type == crate::entities::risk::RiskType::Design);
    }
    if args.process_only {
        risks.retain(|r| r.risk_type == crate::entities::risk::RiskType::Process);
    }

    // Filter by min RPN
    risks.retain(|r| r.rpn.unwrap_or(0) >= args.min_rpn);

    // Sort by RPN descending
    risks.sort_by(|a, b| b.rpn.unwrap_or(0).cmp(&a.rpn.unwrap_or(0)));

    // First pass: collect all row data
    struct FmeaRow {
        id: String,
        failure_mode: String,
        cause: String,
        effect: String,
        s: String,
        o: String,
        d: String,
        rpn: String,
        level: String,
        mitigations: String,
    }
    let mut rows: Vec<FmeaRow> = Vec::new();
    let mut total_rpn: u32 = 0;
    let mut by_level: HashMap<String, usize> = HashMap::new();

    for risk in &risks {
        let risk_short = short_ids
            .get_short_id(&risk.id.to_string())
            .unwrap_or_else(|| risk.id.to_string());
        let failure_mode =
            truncate_str(risk.failure_mode.as_deref().unwrap_or("-"), 20).to_string();
        let cause = truncate_str(risk.cause.as_deref().unwrap_or("-"), 15).to_string();
        let effect = truncate_str(risk.effect.as_deref().unwrap_or("-"), 15).to_string();
        let s = risk.severity.map_or("-".to_string(), |v| v.to_string());
        let o = risk.occurrence.map_or("-".to_string(), |v| v.to_string());
        let d = risk.detection.map_or("-".to_string(), |v| v.to_string());
        let rpn = risk.rpn.map_or("-".to_string(), |v| v.to_string());
        let level = risk.risk_level.map_or("-".to_string(), |l| l.to_string());
        let mitigations = if risk.mitigations.is_empty() {
            "None".to_string()
        } else {
            format!("{} action(s)", risk.mitigations.len())
        };

        if let Some(rpn_val) = risk.rpn {
            total_rpn += rpn_val as u32;
        }

        if let Some(ref lvl) = risk.risk_level {
            *by_level.entry(lvl.to_string()).or_insert(0) += 1;
        }

        rows.push(FmeaRow {
            id: risk_short,
            failure_mode,
            cause,
            effect,
            s,
            o,
            d,
            rpn,
            level,
            mitigations,
        });
    }

    // Generate report
    let mut output = String::new();
    output.push_str("# FMEA Report\n\n");

    // Build table with tabled
    let mut builder = Builder::default();
    builder.push_record(["ID", "Failure Mode", "Cause", "Effect", "S", "O", "D", "RPN", "Level", "Mitigations"]);

    for row in &rows {
        builder.push_record([
            &row.id,
            &row.failure_mode,
            &row.cause,
            &row.effect,
            &row.s,
            &row.o,
            &row.d,
            &row.rpn,
            &row.level,
            &row.mitigations,
        ]);
    }
    output.push_str(&builder.build().with(Style::markdown()).to_string());

    // Summary
    output.push_str("\n## Summary\n\n");
    output.push_str(&format!("- **Total Risks:** {}\n", risks.len()));
    if !risks.is_empty() {
        output.push_str(&format!(
            "- **Average RPN:** {:.1}\n",
            total_rpn as f64 / risks.len() as f64
        ));
    }
    output.push_str(&format!(
        "- **Critical:** {}\n",
        by_level.get("critical").unwrap_or(&0)
    ));
    output.push_str(&format!(
        "- **High:** {}\n",
        by_level.get("high").unwrap_or(&0)
    ));
    output.push_str(&format!(
        "- **Medium:** {}\n",
        by_level.get("medium").unwrap_or(&0)
    ));
    output.push_str(&format!(
        "- **Low:** {}\n",
        by_level.get("low").unwrap_or(&0)
    ));

    let unmitigated = risks.iter().filter(|r| r.mitigations.is_empty()).count();
    output.push_str(&format!("- **Unmitigated:** {}\n", unmitigated));

    // Output
    write_output(&output, args.output)?;
    Ok(())
}
