//! CLI argument definitions using clap derive

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::cli::commands::{
    asm::AsmCommands,
    baseline::BaselineCommands,
    blame::BlameArgs,
    bulk::BulkCommands,
    capa::CapaCommands,
    cmp::CmpCommands,
    ctrl::CtrlCommands,
    diff::DiffArgs,
    feat::FeatCommands,
    history::HistoryArgs,
    init::InitArgs,
    link::LinkCommands,
    mate::MateCommands,
    ncr::NcrCommands,
    proc::ProcCommands,
    quote::QuoteCommands,
    report::ReportCommands,
    req::ReqCommands,
    risk::RiskCommands,
    rslt::RsltCommands,
    status::StatusArgs,
    sup::SupCommands,
    test::TestCommands,
    tol::TolCommands,
    trace::TraceCommands,
    validate::ValidateArgs,
    where_used::WhereUsedArgs,
    work::WorkCommands,
};

#[derive(Parser)]
#[command(name = "tdt")]
#[command(author, version, about = "Tessera Engineering Toolkit")]
#[command(long_about = "A Unix-style toolkit for managing engineering artifacts as plain text files under git version control.")]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub global: GlobalOpts,
}

#[derive(clap::Args, Clone, Debug)]
pub struct GlobalOpts {
    /// Output format
    #[arg(long, short = 'f', global = true, default_value = "auto")]
    pub format: OutputFormat,

    /// Suppress non-essential output
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,

    /// Enable verbose output
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    /// Project root (default: auto-detect by finding .tdt/)
    #[arg(long, global = true)]
    pub project: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new TDT project
    Init(InitArgs),

    /// Requirement management
    #[command(subcommand)]
    Req(ReqCommands),

    /// Risk/FMEA management
    #[command(subcommand)]
    Risk(RiskCommands),

    /// Test protocol management (verification/validation)
    #[command(subcommand)]
    Test(TestCommands),

    /// Test result management
    #[command(subcommand)]
    Rslt(RsltCommands),

    /// Component management (BOM parts)
    #[command(subcommand)]
    Cmp(CmpCommands),

    /// Assembly management (BOM assemblies)
    #[command(subcommand)]
    Asm(AsmCommands),

    /// Quote management (supplier quotations)
    #[command(subcommand)]
    Quote(QuoteCommands),

    /// Supplier management (approved suppliers)
    #[command(subcommand)]
    Sup(SupCommands),

    /// Manufacturing process management
    #[command(subcommand)]
    Proc(ProcCommands),

    /// Control plan item management (SPC, inspection, etc.)
    #[command(subcommand)]
    Ctrl(CtrlCommands),

    /// Work instruction management (operator procedures)
    #[command(subcommand)]
    Work(WorkCommands),

    /// Non-conformance report management
    #[command(subcommand)]
    Ncr(NcrCommands),

    /// Corrective/preventive action management
    #[command(subcommand)]
    Capa(CapaCommands),

    /// Feature management (dimensional features on components)
    #[command(subcommand)]
    Feat(FeatCommands),

    /// Mate management (1:1 feature contacts with fit calculation)
    #[command(subcommand)]
    Mate(MateCommands),

    /// Tolerance stackup analysis (worst-case, RSS, Monte Carlo)
    #[command(subcommand)]
    Tol(TolCommands),

    /// Validate project files against schemas
    Validate(ValidateArgs),

    /// Manage links between entities
    #[command(subcommand)]
    Link(LinkCommands),

    /// Traceability queries and reports
    #[command(subcommand)]
    Trace(TraceCommands),

    /// Generate engineering reports (RVM, FMEA, BOM, etc.)
    #[command(subcommand)]
    Report(ReportCommands),

    /// Find where an entity is used/referenced
    WhereUsed(WhereUsedArgs),

    /// View git history for an entity
    History(HistoryArgs),

    /// View git blame for an entity
    Blame(BlameArgs),

    /// View git diff for an entity
    Diff(DiffArgs),

    /// Baseline management (git tags with validation)
    #[command(subcommand)]
    Baseline(BaselineCommands),

    /// Bulk operations on multiple entities
    #[command(subcommand)]
    Bulk(BulkCommands),

    /// Show project status dashboard
    Status(StatusArgs),
}

#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Automatically detect based on context (yaml for show, tsv for list)
    #[default]
    Auto,
    /// YAML format (full fidelity)
    Yaml,
    /// Tab-separated values (for piping)
    Tsv,
    /// JSON format (for programming)
    Json,
    /// CSV format (for spreadsheets)
    Csv,
    /// Markdown tables
    Md,
    /// Just IDs, one per line
    Id,
}
