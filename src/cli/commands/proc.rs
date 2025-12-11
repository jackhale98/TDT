//! `pdt proc` command - Manufacturing process management

use clap::{Subcommand, ValueEnum};
use console::style;
use miette::{IntoDiagnostic, Result};
use std::fs;

use crate::cli::{GlobalOpts, OutputFormat};
use crate::core::identity::{EntityId, EntityPrefix};
use crate::core::project::Project;
use crate::core::shortid::ShortIdIndex;
use crate::core::Config;
use crate::entities::process::{Process, ProcessType};
use crate::schema::template::{TemplateContext, TemplateGenerator};

#[derive(Subcommand, Debug)]
pub enum ProcCommands {
    /// List manufacturing processes with filtering
    List(ListArgs),

    /// Create a new manufacturing process
    New(NewArgs),

    /// Show a process's details
    Show(ShowArgs),

    /// Edit a process in your editor
    Edit(EditArgs),
}

/// Process type filter
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ProcessTypeFilter {
    Machining,
    Assembly,
    Inspection,
    Test,
    Finishing,
    Packaging,
    Handling,
    HeatTreat,
    Welding,
    Coating,
    All,
}

/// Status filter
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StatusFilter {
    Draft,
    Review,
    Approved,
    Released,
    Obsolete,
    All,
}

#[derive(clap::Args, Debug)]
pub struct ListArgs {
    /// Filter by process type
    #[arg(long, short = 't', default_value = "all")]
    pub r#type: ProcessTypeFilter,

    /// Filter by status
    #[arg(long, short = 's', default_value = "all")]
    pub status: StatusFilter,

    /// Search in title and description
    #[arg(long)]
    pub search: Option<String>,

    /// Sort by field
    #[arg(long, default_value = "title")]
    pub sort: SortField,

    /// Reverse sort order
    #[arg(long, short = 'r')]
    pub reverse: bool,

    /// Limit number of results
    #[arg(long, short = 'n')]
    pub limit: Option<usize>,

    /// Show only count
    #[arg(long)]
    pub count: bool,

    /// Output format
    #[arg(long, short = 'o', default_value = "auto")]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortField {
    Title,
    Type,
    Status,
    Created,
}

#[derive(clap::Args, Debug)]
pub struct NewArgs {
    /// Process title (required)
    #[arg(long, short = 't')]
    pub title: Option<String>,

    /// Process type
    #[arg(long, short = 'T', default_value = "machining")]
    pub r#type: String,

    /// Operation number (e.g., "OP-010")
    #[arg(long, short = 'n')]
    pub op_number: Option<String>,

    /// Cycle time in minutes
    #[arg(long)]
    pub cycle_time: Option<f64>,

    /// Setup time in minutes
    #[arg(long)]
    pub setup_time: Option<f64>,

    /// Open in editor after creation
    #[arg(long, short = 'e')]
    pub edit: bool,

    /// Skip opening in editor
    #[arg(long)]
    pub no_edit: bool,

    /// Interactive mode (prompt for fields)
    #[arg(long, short = 'i')]
    pub interactive: bool,
}

#[derive(clap::Args, Debug)]
pub struct ShowArgs {
    /// Process ID or short ID (PROC@N)
    pub id: String,

    /// Output format
    #[arg(long, short = 'o', default_value = "yaml")]
    pub format: OutputFormat,
}

#[derive(clap::Args, Debug)]
pub struct EditArgs {
    /// Process ID or short ID (PROC@N)
    pub id: String,
}

/// Run a process subcommand
pub fn run(cmd: ProcCommands, _global: &GlobalOpts) -> Result<()> {
    match cmd {
        ProcCommands::List(args) => run_list(args),
        ProcCommands::New(args) => run_new(args),
        ProcCommands::Show(args) => run_show(args),
        ProcCommands::Edit(args) => run_edit(args),
    }
}

fn run_list(args: ListArgs) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;
    let proc_dir = project.root().join("manufacturing/processes");

    if !proc_dir.exists() {
        if args.count {
            println!("0");
        } else {
            println!("No processes found.");
        }
        return Ok(());
    }

    // Load and parse all processes
    let mut processes: Vec<Process> = Vec::new();

    for entry in fs::read_dir(&proc_dir).into_diagnostic()? {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "yaml") {
            let content = fs::read_to_string(&path).into_diagnostic()?;
            if let Ok(proc) = serde_yml::from_str::<Process>(&content) {
                processes.push(proc);
            }
        }
    }

    // Apply filters
    let processes: Vec<Process> = processes
        .into_iter()
        .filter(|p| match args.r#type {
            ProcessTypeFilter::Machining => p.process_type == ProcessType::Machining,
            ProcessTypeFilter::Assembly => p.process_type == ProcessType::Assembly,
            ProcessTypeFilter::Inspection => p.process_type == ProcessType::Inspection,
            ProcessTypeFilter::Test => p.process_type == ProcessType::Test,
            ProcessTypeFilter::Finishing => p.process_type == ProcessType::Finishing,
            ProcessTypeFilter::Packaging => p.process_type == ProcessType::Packaging,
            ProcessTypeFilter::Handling => p.process_type == ProcessType::Handling,
            ProcessTypeFilter::HeatTreat => p.process_type == ProcessType::HeatTreat,
            ProcessTypeFilter::Welding => p.process_type == ProcessType::Welding,
            ProcessTypeFilter::Coating => p.process_type == ProcessType::Coating,
            ProcessTypeFilter::All => true,
        })
        .filter(|p| match args.status {
            StatusFilter::Draft => p.status == crate::core::entity::Status::Draft,
            StatusFilter::Review => p.status == crate::core::entity::Status::Review,
            StatusFilter::Approved => p.status == crate::core::entity::Status::Approved,
            StatusFilter::Released => p.status == crate::core::entity::Status::Released,
            StatusFilter::Obsolete => p.status == crate::core::entity::Status::Obsolete,
            StatusFilter::All => true,
        })
        .filter(|p| {
            if let Some(ref search) = args.search {
                let search_lower = search.to_lowercase();
                p.title.to_lowercase().contains(&search_lower)
                    || p.description
                        .as_ref()
                        .map_or(false, |d| d.to_lowercase().contains(&search_lower))
            } else {
                true
            }
        })
        .collect();

    // Sort
    let mut processes = processes;
    match args.sort {
        SortField::Title => processes.sort_by(|a, b| a.title.cmp(&b.title)),
        SortField::Type => processes.sort_by(|a, b| {
            format!("{:?}", a.process_type).cmp(&format!("{:?}", b.process_type))
        }),
        SortField::Status => {
            processes.sort_by(|a, b| format!("{:?}", a.status).cmp(&format!("{:?}", b.status)))
        }
        SortField::Created => processes.sort_by(|a, b| a.created.cmp(&b.created)),
    }

    if args.reverse {
        processes.reverse();
    }

    // Apply limit
    if let Some(limit) = args.limit {
        processes.truncate(limit);
    }

    // Count only
    if args.count {
        println!("{}", processes.len());
        return Ok(());
    }

    // No results
    if processes.is_empty() {
        println!("No processes found.");
        return Ok(());
    }

    // Update short ID index
    let mut short_ids = ShortIdIndex::load(&project);
    short_ids.ensure_all(processes.iter().map(|p| p.id.to_string()));
    let _ = short_ids.save(&project);

    // Output based on format
    let format = if args.format == OutputFormat::Auto {
        OutputFormat::Tsv
    } else {
        args.format
    };

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&processes).into_diagnostic()?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yml::to_string(&processes).into_diagnostic()?;
            print!("{}", yaml);
        }
        OutputFormat::Csv => {
            println!("short_id,id,title,type,op_number,cycle_time,status");
            for proc in &processes {
                let short_id = short_ids.get_short_id(&proc.id.to_string()).unwrap_or_default();
                println!(
                    "{},{},{},{},{},{},{}",
                    short_id,
                    proc.id,
                    escape_csv(&proc.title),
                    proc.process_type,
                    proc.operation_number.as_deref().unwrap_or(""),
                    proc.cycle_time_minutes.map_or(String::new(), |t| t.to_string()),
                    proc.status
                );
            }
        }
        OutputFormat::Tsv => {
            println!(
                "{:<8} {:<17} {:<30} {:<12} {:<8} {:<8} {:<10}",
                style("SHORT").bold().dim(),
                style("ID").bold(),
                style("TITLE").bold(),
                style("TYPE").bold(),
                style("OP #").bold(),
                style("CYCLE").bold(),
                style("STATUS").bold()
            );
            println!("{}", "-".repeat(95));

            for proc in &processes {
                let short_id = short_ids.get_short_id(&proc.id.to_string()).unwrap_or_default();
                let id_display = format_short_id(&proc.id);
                let title_truncated = truncate_str(&proc.title, 28);
                let cycle_str = proc
                    .cycle_time_minutes
                    .map_or("-".to_string(), |t| format!("{:.1}m", t));

                println!(
                    "{:<8} {:<17} {:<30} {:<12} {:<8} {:<8} {:<10}",
                    style(&short_id).cyan(),
                    id_display,
                    title_truncated,
                    proc.process_type,
                    proc.operation_number.as_deref().unwrap_or("-"),
                    cycle_str,
                    proc.status
                );
            }

            println!();
            println!(
                "{} process(es) found. Use {} to reference by short ID.",
                style(processes.len()).cyan(),
                style("PROC@N").cyan()
            );
        }
        OutputFormat::Id => {
            for proc in &processes {
                println!("{}", proc.id);
            }
        }
        OutputFormat::Md => {
            println!("| Short | ID | Title | Type | Op # | Cycle | Status |");
            println!("|---|---|---|---|---|---|---|");
            for proc in &processes {
                let short_id = short_ids.get_short_id(&proc.id.to_string()).unwrap_or_default();
                let cycle_str = proc
                    .cycle_time_minutes
                    .map_or("-".to_string(), |t| format!("{:.1}m", t));
                println!(
                    "| {} | {} | {} | {} | {} | {} | {} |",
                    short_id,
                    format_short_id(&proc.id),
                    proc.title,
                    proc.process_type,
                    proc.operation_number.as_deref().unwrap_or("-"),
                    cycle_str,
                    proc.status
                );
            }
        }
        OutputFormat::Auto => unreachable!(),
    }

    Ok(())
}

fn run_new(args: NewArgs) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;
    let config = Config::load();

    let title: String;
    let process_type: String;

    if args.interactive || args.title.is_none() {
        // Interactive mode
        use dialoguer::{Input, Select};

        title = Input::new()
            .with_prompt("Process title")
            .interact_text()
            .into_diagnostic()?;

        let type_options = [
            "machining",
            "assembly",
            "inspection",
            "test",
            "finishing",
            "packaging",
            "handling",
            "heat_treat",
            "welding",
            "coating",
        ];
        let type_idx = Select::new()
            .with_prompt("Process type")
            .items(&type_options)
            .default(0)
            .interact()
            .into_diagnostic()?;
        process_type = type_options[type_idx].to_string();
    } else {
        title = args
            .title
            .ok_or_else(|| miette::miette!("Title is required (use --title or -t)"))?;
        process_type = args.r#type;
    }

    // Validate process type
    process_type
        .parse::<ProcessType>()
        .map_err(|e| miette::miette!("{}", e))?;

    // Generate ID
    let id = EntityId::new(EntityPrefix::Proc);

    // Generate template
    let generator = TemplateGenerator::new().map_err(|e| miette::miette!("{}", e))?;
    let mut ctx = TemplateContext::new(id.clone(), config.author())
        .with_title(&title)
        .with_process_type(&process_type);

    if let Some(ref op) = args.op_number {
        ctx = ctx.with_operation_number(op);
    }
    if let Some(cycle) = args.cycle_time {
        ctx = ctx.with_cycle_time(cycle);
    }
    if let Some(setup) = args.setup_time {
        ctx = ctx.with_setup_time(setup);
    }

    let yaml_content = generator
        .generate_process(&ctx)
        .map_err(|e| miette::miette!("{}", e))?;

    // Write file
    let output_dir = project.root().join("manufacturing/processes");
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir).into_diagnostic()?;
    }

    let file_path = output_dir.join(format!("{}.pdt.yaml", id));
    fs::write(&file_path, &yaml_content).into_diagnostic()?;

    // Add to short ID index
    let mut short_ids = ShortIdIndex::load(&project);
    let short_id = short_ids.add(id.to_string());
    let _ = short_ids.save(&project);

    println!(
        "{} Created process {}",
        style("✓").green(),
        style(short_id.unwrap_or_else(|| format_short_id(&id))).cyan()
    );
    println!("   {}", style(file_path.display()).dim());
    println!(
        "   Type: {} | {}",
        style(&process_type).yellow(),
        style(&title).white()
    );

    // Open in editor if requested
    if args.edit || (!args.no_edit && !args.interactive) {
        println!();
        println!("Opening in {}...", style(config.editor()).yellow());

        config.run_editor(&file_path).into_diagnostic()?;
    }

    Ok(())
}

fn run_show(args: ShowArgs) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;

    // Resolve short ID if needed
    let short_ids = ShortIdIndex::load(&project);
    let resolved_id = short_ids
        .resolve(&args.id)
        .unwrap_or_else(|| args.id.clone());

    // Find the process file
    let proc_dir = project.root().join("manufacturing/processes");
    let mut found_path = None;

    if proc_dir.exists() {
        for entry in fs::read_dir(&proc_dir).into_diagnostic()? {
            let entry = entry.into_diagnostic()?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "yaml") {
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if filename.contains(&resolved_id) || filename.starts_with(&resolved_id) {
                    found_path = Some(path);
                    break;
                }
            }
        }
    }

    let path = found_path.ok_or_else(|| miette::miette!("No process found matching '{}'", args.id))?;

    // Read and display
    let content = fs::read_to_string(&path).into_diagnostic()?;

    match args.format {
        OutputFormat::Yaml | OutputFormat::Auto => {
            print!("{}", content);
        }
        OutputFormat::Json => {
            let proc: Process = serde_yml::from_str(&content).into_diagnostic()?;
            let json = serde_json::to_string_pretty(&proc).into_diagnostic()?;
            println!("{}", json);
        }
        _ => {
            print!("{}", content);
        }
    }

    Ok(())
}

fn run_edit(args: EditArgs) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;
    let config = Config::load();

    // Resolve short ID if needed
    let short_ids = ShortIdIndex::load(&project);
    let resolved_id = short_ids
        .resolve(&args.id)
        .unwrap_or_else(|| args.id.clone());

    // Find the process file
    let proc_dir = project.root().join("manufacturing/processes");
    let mut found_path = None;

    if proc_dir.exists() {
        for entry in fs::read_dir(&proc_dir).into_diagnostic()? {
            let entry = entry.into_diagnostic()?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "yaml") {
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if filename.contains(&resolved_id) || filename.starts_with(&resolved_id) {
                    found_path = Some(path);
                    break;
                }
            }
        }
    }

    let path = found_path.ok_or_else(|| miette::miette!("No process found matching '{}'", args.id))?;

    println!(
        "Opening {} in {}...",
        style(path.display()).cyan(),
        style(config.editor()).yellow()
    );

    config.run_editor(&path).into_diagnostic()?;

    Ok(())
}

// Helper functions

fn format_short_id(id: &EntityId) -> String {
    let s = id.to_string();
    if s.len() > 16 {
        format!("{}...", &s[..13])
    } else {
        s
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
