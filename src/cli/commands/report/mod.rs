//! `tdt report` command - Generate engineering reports

mod bom;
mod fmea;
mod open_issues;
mod rvm;
mod test_status;
mod tolerance;

use clap::Subcommand;
use miette::{IntoDiagnostic, Result};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::cli::GlobalOpts;
use crate::core::project::Project;
use crate::entities::assembly::Assembly;
use crate::entities::component::Component;
use crate::entities::feature::Feature;
use crate::entities::mate::Mate;
use crate::entities::quote::Quote;
use crate::entities::requirement::Requirement;
use crate::entities::result::Result as TestResult;
use crate::entities::risk::Risk;
use crate::entities::stackup::Stackup;
use crate::entities::test::Test;

pub use bom::BomArgs;
pub use fmea::FmeaArgs;
pub use open_issues::OpenIssuesArgs;
pub use rvm::RvmArgs;
pub use test_status::TestStatusArgs;
pub use tolerance::ToleranceArgs;

#[derive(Subcommand, Debug)]
pub enum ReportCommands {
    /// Requirements Verification Matrix (RVM)
    Rvm(RvmArgs),

    /// FMEA report sorted by RPN
    Fmea(FmeaArgs),

    /// BOM (Bill of Materials) with costs
    Bom(BomArgs),

    /// Test execution status summary
    TestStatus(TestStatusArgs),

    /// All open issues (NCRs, CAPAs, failed tests)
    OpenIssues(OpenIssuesArgs),

    /// Tolerance analysis report (features, mates, stackups by component)
    #[clap(alias = "tol")]
    Tolerance(ToleranceArgs),
}

pub fn run(cmd: ReportCommands, global: &GlobalOpts) -> Result<()> {
    match cmd {
        ReportCommands::Rvm(args) => rvm::run(args, global),
        ReportCommands::Fmea(args) => fmea::run(args, global),
        ReportCommands::Bom(args) => bom::run(args, global),
        ReportCommands::TestStatus(args) => test_status::run(args, global),
        ReportCommands::OpenIssues(args) => open_issues::run(args, global),
        ReportCommands::Tolerance(args) => tolerance::run(args, global),
    }
}

// Shared helper functions

pub(crate) fn write_output(content: &str, output_path: Option<PathBuf>) -> Result<()> {
    match output_path {
        Some(path) => {
            let file = File::create(&path).into_diagnostic()?;
            let mut writer = BufWriter::new(file);
            writer.write_all(content.as_bytes()).into_diagnostic()?;
            println!("Report written to: {}", path.display());
        }
        None => {
            print!("{}", content);
        }
    }
    Ok(())
}

pub(crate) fn load_all_requirements(project: &Project) -> Vec<Requirement> {
    let mut requirements = Vec::new();
    let dir = project.root().join("requirements");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(req) = crate::yaml::parse_yaml_file::<Requirement>(entry.path()) {
                requirements.push(req);
            }
        }
    }

    requirements
}

pub(crate) fn load_all_tests(project: &Project) -> Vec<Test> {
    let mut tests = Vec::new();

    for subdir in ["verification/protocols", "validation/protocols"] {
        let dir = project.root().join(subdir);
        if dir.exists() {
            for entry in walkdir::WalkDir::new(&dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
            {
                if let Ok(test) = crate::yaml::parse_yaml_file::<Test>(entry.path()) {
                    tests.push(test);
                }
            }
        }
    }

    tests
}

pub(crate) fn load_all_results(project: &Project) -> Vec<TestResult> {
    let mut results = Vec::new();

    for subdir in ["verification/results", "validation/results"] {
        let dir = project.root().join(subdir);
        if dir.exists() {
            for entry in walkdir::WalkDir::new(&dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
            {
                if let Ok(result) = crate::yaml::parse_yaml_file::<TestResult>(entry.path()) {
                    results.push(result);
                }
            }
        }
    }

    results
}

pub(crate) fn load_all_risks(project: &Project) -> Vec<Risk> {
    let mut risks = Vec::new();
    let dir = project.root().join("risks");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(risk) = crate::yaml::parse_yaml_file::<Risk>(entry.path()) {
                risks.push(risk);
            }
        }
    }

    risks
}

pub(crate) fn load_all_components(project: &Project) -> Vec<Component> {
    let mut components = Vec::new();
    let dir = project.root().join("bom/components");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(cmp) = crate::yaml::parse_yaml_file::<Component>(entry.path()) {
                components.push(cmp);
            }
        }
    }

    components
}

pub(crate) fn load_all_assemblies(project: &Project) -> Vec<Assembly> {
    let mut assemblies = Vec::new();
    let dir = project.root().join("bom/assemblies");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(asm) = crate::yaml::parse_yaml_file::<Assembly>(entry.path()) {
                assemblies.push(asm);
            }
        }
    }

    assemblies
}

pub(crate) fn load_all_quotes(project: &Project) -> Vec<Quote> {
    let mut quotes = Vec::new();
    let dir = project.root().join("bom/quotes");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(quote) = crate::yaml::parse_yaml_file::<Quote>(entry.path()) {
                quotes.push(quote);
            }
        }
    }

    quotes
}

pub(crate) fn load_assembly(project: &Project, id: &str) -> Result<Assembly> {
    let dir = project.root().join("bom/assemblies");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(asm) = crate::yaml::parse_yaml_file::<Assembly>(entry.path()) {
                if asm.id.to_string() == id {
                    return Ok(asm);
                }
            }
        }
    }

    Err(miette::miette!("Assembly not found: {}", id))
}

pub(crate) fn load_all_ncrs(project: &Project) -> Vec<crate::entities::ncr::Ncr> {
    let mut ncrs = Vec::new();
    let dir = project.root().join("manufacturing/ncrs");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(ncr) = crate::yaml::parse_yaml_file::<crate::entities::ncr::Ncr>(entry.path())
            {
                ncrs.push(ncr);
            }
        }
    }

    ncrs
}

pub(crate) fn load_all_capas(project: &Project) -> Vec<crate::entities::capa::Capa> {
    let mut capas = Vec::new();
    let dir = project.root().join("manufacturing/capas");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(capa) =
                crate::yaml::parse_yaml_file::<crate::entities::capa::Capa>(entry.path())
            {
                capas.push(capa);
            }
        }
    }

    capas
}

pub(crate) fn load_all_features(project: &Project) -> Vec<Feature> {
    let mut features = Vec::new();
    let dir = project.root().join("tolerances/features");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(feat) = crate::yaml::parse_yaml_file::<Feature>(entry.path()) {
                features.push(feat);
            }
        }
    }

    features
}

pub(crate) fn load_all_mates(project: &Project) -> Vec<Mate> {
    let mut mates = Vec::new();
    let dir = project.root().join("tolerances/mates");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(mate) = crate::yaml::parse_yaml_file::<Mate>(entry.path()) {
                mates.push(mate);
            }
        }
    }

    mates
}

pub(crate) fn load_all_stackups(project: &Project) -> Vec<Stackup> {
    let mut stackups = Vec::new();
    let dir = project.root().join("tolerances/stackups");

    if dir.exists() {
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().to_string_lossy().ends_with(".tdt.yaml"))
        {
            if let Ok(stackup) = crate::yaml::parse_yaml_file::<Stackup>(entry.path()) {
                stackups.push(stackup);
            }
        }
    }

    stackups
}
