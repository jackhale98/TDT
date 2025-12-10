//! `pdt asm` command - Assembly management

use clap::{Subcommand, ValueEnum};
use console::style;
use miette::{IntoDiagnostic, Result};
use std::fs;

use crate::cli::{GlobalOpts, OutputFormat};
use crate::core::identity::{EntityId, EntityPrefix};
use crate::core::project::Project;
use crate::core::shortid::ShortIdIndex;
use crate::core::Config;
use crate::entities::assembly::Assembly;
use crate::entities::component::Component;
use crate::schema::template::{TemplateContext, TemplateGenerator};

#[derive(Subcommand, Debug)]
pub enum AsmCommands {
    /// List assemblies with filtering
    List(ListArgs),

    /// Create a new assembly
    New(NewArgs),

    /// Show an assembly's details
    Show(ShowArgs),

    /// Edit an assembly in your editor
    Edit(EditArgs),

    /// Show expanded BOM for an assembly
    Bom(BomArgs),
}

/// Status filter
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StatusFilter {
    Draft,
    Review,
    Approved,
    Released,
    Obsolete,
    /// All statuses
    All,
}

#[derive(clap::Args, Debug)]
pub struct ListArgs {
    /// Filter by status
    #[arg(long, short = 's', default_value = "all")]
    pub status: StatusFilter,

    /// Search in part number and title
    #[arg(long)]
    pub search: Option<String>,

    /// Sort by field
    #[arg(long, default_value = "part-number")]
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
    PartNumber,
    Title,
    Status,
    Created,
}

#[derive(clap::Args, Debug)]
pub struct NewArgs {
    /// Part number (required)
    #[arg(long, short = 'p')]
    pub part_number: Option<String>,

    /// Title/description
    #[arg(long, short = 't')]
    pub title: Option<String>,

    /// Part revision
    #[arg(long)]
    pub revision: Option<String>,

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
    /// Assembly ID or short ID (ASM@N)
    pub id: String,

    /// Output format
    #[arg(long, short = 'o', default_value = "yaml")]
    pub format: OutputFormat,
}

#[derive(clap::Args, Debug)]
pub struct EditArgs {
    /// Assembly ID or short ID (ASM@N)
    pub id: String,
}

#[derive(clap::Args, Debug)]
pub struct BomArgs {
    /// Assembly ID or short ID (ASM@N)
    pub id: String,

    /// Flatten nested assemblies (show all components)
    #[arg(long)]
    pub flat: bool,

    /// Output format
    #[arg(long, short = 'o', default_value = "auto")]
    pub format: OutputFormat,
}

/// Run an assembly subcommand
pub fn run(cmd: AsmCommands, _global: &GlobalOpts) -> Result<()> {
    match cmd {
        AsmCommands::List(args) => run_list(args),
        AsmCommands::New(args) => run_new(args),
        AsmCommands::Show(args) => run_show(args),
        AsmCommands::Edit(args) => run_edit(args),
        AsmCommands::Bom(args) => run_bom(args),
    }
}

fn run_list(args: ListArgs) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;
    let asm_dir = project.root().join("bom/assemblies");

    if !asm_dir.exists() {
        if args.count {
            println!("0");
        } else {
            println!("No assemblies found.");
        }
        return Ok(());
    }

    // Load and parse all assemblies
    let mut assemblies: Vec<Assembly> = Vec::new();

    for entry in fs::read_dir(&asm_dir).into_diagnostic()? {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "yaml") {
            let content = fs::read_to_string(&path).into_diagnostic()?;
            if let Ok(asm) = serde_yml::from_str::<Assembly>(&content) {
                assemblies.push(asm);
            }
        }
    }

    // Apply filters
    let assemblies: Vec<Assembly> = assemblies
        .into_iter()
        .filter(|a| match args.status {
            StatusFilter::Draft => a.status == crate::core::entity::Status::Draft,
            StatusFilter::Review => a.status == crate::core::entity::Status::Review,
            StatusFilter::Approved => a.status == crate::core::entity::Status::Approved,
            StatusFilter::Released => a.status == crate::core::entity::Status::Released,
            StatusFilter::Obsolete => a.status == crate::core::entity::Status::Obsolete,
            StatusFilter::All => true,
        })
        .filter(|a| {
            if let Some(ref search) = args.search {
                let search_lower = search.to_lowercase();
                a.part_number.to_lowercase().contains(&search_lower)
                    || a.title.to_lowercase().contains(&search_lower)
                    || a.description
                        .as_ref()
                        .map_or(false, |d| d.to_lowercase().contains(&search_lower))
            } else {
                true
            }
        })
        .collect();

    // Sort
    let mut assemblies = assemblies;
    match args.sort {
        SortField::PartNumber => assemblies.sort_by(|a, b| a.part_number.cmp(&b.part_number)),
        SortField::Title => assemblies.sort_by(|a, b| a.title.cmp(&b.title)),
        SortField::Status => {
            assemblies.sort_by(|a, b| format!("{:?}", a.status).cmp(&format!("{:?}", b.status)))
        }
        SortField::Created => assemblies.sort_by(|a, b| a.created.cmp(&b.created)),
    }

    if args.reverse {
        assemblies.reverse();
    }

    // Apply limit
    if let Some(limit) = args.limit {
        assemblies.truncate(limit);
    }

    // Count only
    if args.count {
        println!("{}", assemblies.len());
        return Ok(());
    }

    // No results
    if assemblies.is_empty() {
        println!("No assemblies found.");
        return Ok(());
    }

    // Update short ID index
    let mut short_ids = ShortIdIndex::load(&project);
    short_ids.ensure_all(assemblies.iter().map(|a| a.id.to_string()));
    let _ = short_ids.save(&project);

    // Output based on format
    let format = if args.format == OutputFormat::Auto {
        OutputFormat::Tsv
    } else {
        args.format
    };

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&assemblies).into_diagnostic()?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yml::to_string(&assemblies).into_diagnostic()?;
            print!("{}", yaml);
        }
        OutputFormat::Csv => {
            println!("short_id,id,part_number,revision,title,bom_items,status");
            for asm in &assemblies {
                let short_id = short_ids.get_short_id(&asm.id.to_string()).unwrap_or_default();
                println!(
                    "{},{},{},{},{},{},{}",
                    short_id,
                    asm.id,
                    asm.part_number,
                    asm.revision.as_deref().unwrap_or(""),
                    escape_csv(&asm.title),
                    asm.bom.len(),
                    asm.status
                );
            }
        }
        OutputFormat::Tsv => {
            println!(
                "{:<8} {:<17} {:<12} {:<30} {:<6} {:<10}",
                style("SHORT").bold().dim(),
                style("ID").bold(),
                style("PART #").bold(),
                style("TITLE").bold(),
                style("BOM").bold(),
                style("STATUS").bold()
            );
            println!("{}", "-".repeat(90));

            for asm in &assemblies {
                let short_id = short_ids.get_short_id(&asm.id.to_string()).unwrap_or_default();
                let id_display = format_short_id(&asm.id);
                let title_truncated = truncate_str(&asm.title, 28);

                println!(
                    "{:<8} {:<17} {:<12} {:<30} {:<6} {:<10}",
                    style(&short_id).cyan(),
                    id_display,
                    truncate_str(&asm.part_number, 10),
                    title_truncated,
                    asm.bom.len(),
                    asm.status
                );
            }

            println!();
            println!(
                "{} assembly(s) found. Use {} to reference by short ID.",
                style(assemblies.len()).cyan(),
                style("ASM@N").cyan()
            );
        }
        OutputFormat::Id => {
            for asm in &assemblies {
                println!("{}", asm.id);
            }
        }
        OutputFormat::Md => {
            println!("| Short | ID | Part # | Title | BOM Items | Status |");
            println!("|---|---|---|---|---|---|");
            for asm in &assemblies {
                let short_id = short_ids.get_short_id(&asm.id.to_string()).unwrap_or_default();
                println!(
                    "| {} | {} | {} | {} | {} | {} |",
                    short_id,
                    format_short_id(&asm.id),
                    asm.part_number,
                    asm.title,
                    asm.bom.len(),
                    asm.status
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

    let part_number: String;
    let title: String;

    if args.interactive || (args.part_number.is_none() && args.title.is_none()) {
        // Interactive mode
        use dialoguer::Input;

        part_number = Input::new()
            .with_prompt("Part number")
            .interact_text()
            .into_diagnostic()?;

        title = Input::new()
            .with_prompt("Title")
            .interact_text()
            .into_diagnostic()?;
    } else {
        part_number = args
            .part_number
            .ok_or_else(|| miette::miette!("Part number is required (use --part-number or -p)"))?;
        title = args
            .title
            .ok_or_else(|| miette::miette!("Title is required (use --title or -t)"))?;
    }

    // Generate ID
    let id = EntityId::new(EntityPrefix::Asm);

    // Generate template
    let generator = TemplateGenerator::new().map_err(|e| miette::miette!("{}", e))?;
    let ctx = TemplateContext::new(id.clone(), config.author())
        .with_title(&title)
        .with_part_number(&part_number);

    let ctx = if let Some(ref rev) = args.revision {
        ctx.with_part_revision(rev)
    } else {
        ctx
    };

    let yaml_content = generator
        .generate_assembly(&ctx)
        .map_err(|e| miette::miette!("{}", e))?;

    // Write file
    let output_dir = project.root().join("bom/assemblies");
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
        "{} Created assembly {}",
        style("✓").green(),
        style(short_id.unwrap_or_else(|| format_short_id(&id))).cyan()
    );
    println!("   {}", style(file_path.display()).dim());
    println!(
        "   Part: {} | {}",
        style(&part_number).yellow(),
        style(&title).white()
    );

    // Open in editor if requested
    if args.edit || (!args.no_edit && !args.interactive) {
        let editor = config.editor();
        println!();
        println!("Opening in {}...", style(&editor).yellow());

        std::process::Command::new(&editor)
            .arg(&file_path)
            .status()
            .into_diagnostic()?;
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

    // Find the assembly file
    let asm_dir = project.root().join("bom/assemblies");
    let mut found_path = None;

    if asm_dir.exists() {
        for entry in fs::read_dir(&asm_dir).into_diagnostic()? {
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

    let path = found_path.ok_or_else(|| miette::miette!("No assembly found matching '{}'", args.id))?;

    // Read and display
    let content = fs::read_to_string(&path).into_diagnostic()?;

    match args.format {
        OutputFormat::Yaml | OutputFormat::Auto => {
            print!("{}", content);
        }
        OutputFormat::Json => {
            let asm: Assembly = serde_yml::from_str(&content).into_diagnostic()?;
            let json = serde_json::to_string_pretty(&asm).into_diagnostic()?;
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

    // Find the assembly file
    let asm_dir = project.root().join("bom/assemblies");
    let mut found_path = None;

    if asm_dir.exists() {
        for entry in fs::read_dir(&asm_dir).into_diagnostic()? {
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

    let path = found_path.ok_or_else(|| miette::miette!("No assembly found matching '{}'", args.id))?;

    let editor = config.editor();
    println!("Opening {} in {}...", style(path.display()).cyan(), style(&editor).yellow());

    std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .into_diagnostic()?;

    Ok(())
}

fn run_bom(args: BomArgs) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;

    // Resolve short ID if needed
    let short_ids = ShortIdIndex::load(&project);
    let resolved_id = short_ids
        .resolve(&args.id)
        .unwrap_or_else(|| args.id.clone());

    // Find and load the assembly
    let asm_dir = project.root().join("bom/assemblies");
    let mut found_asm = None;

    if asm_dir.exists() {
        for entry in fs::read_dir(&asm_dir).into_diagnostic()? {
            let entry = entry.into_diagnostic()?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "yaml") {
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if filename.contains(&resolved_id) || filename.starts_with(&resolved_id) {
                    let content = fs::read_to_string(&path).into_diagnostic()?;
                    if let Ok(asm) = serde_yml::from_str::<Assembly>(&content) {
                        found_asm = Some(asm);
                        break;
                    }
                }
            }
        }
    }

    let assembly = found_asm.ok_or_else(|| miette::miette!("No assembly found matching '{}'", args.id))?;

    // Load component index for resolving names
    let cmp_dir = project.root().join("bom/components");
    let mut components: std::collections::HashMap<String, Component> = std::collections::HashMap::new();

    if cmp_dir.exists() {
        for entry in fs::read_dir(&cmp_dir).into_diagnostic()? {
            let entry = entry.into_diagnostic()?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "yaml") {
                let content = fs::read_to_string(&path).into_diagnostic()?;
                if let Ok(cmp) = serde_yml::from_str::<Component>(&content) {
                    components.insert(cmp.id.to_string(), cmp);
                }
            }
        }
    }

    // Display BOM
    let format = if args.format == OutputFormat::Auto {
        OutputFormat::Tsv
    } else {
        args.format
    };

    println!();
    println!(
        "{} BOM for {} - {}",
        style("Assembly").bold(),
        style(&assembly.part_number).yellow(),
        style(&assembly.title).white()
    );
    println!();

    match format {
        OutputFormat::Tsv | OutputFormat::Auto => {
            println!(
                "{:<6} {:<15} {:<12} {:<30} {:<20}",
                style("QTY").bold(),
                style("COMPONENT ID").bold(),
                style("PART #").bold(),
                style("TITLE").bold(),
                style("REFERENCES").bold()
            );
            println!("{}", "-".repeat(85));

            for item in &assembly.bom {
                let cmp_info = components.get(&item.component_id);
                let part_number = cmp_info.map(|c| c.part_number.as_str()).unwrap_or("-");
                let title = cmp_info.map(|c| c.title.as_str()).unwrap_or("(not found)");
                let refs = if item.reference_designators.is_empty() {
                    String::new()
                } else {
                    item.reference_designators.join(", ")
                };

                println!(
                    "{:<6} {:<15} {:<12} {:<30} {:<20}",
                    item.quantity,
                    truncate_str(&item.component_id, 13),
                    truncate_str(part_number, 10),
                    truncate_str(title, 28),
                    truncate_str(&refs, 18)
                );
            }

            if !assembly.subassemblies.is_empty() {
                println!();
                println!("{}", style("Sub-assemblies:").bold());
                for sub_id in &assembly.subassemblies {
                    println!("  - {}", sub_id);
                }
            }

            println!();
            println!(
                "{} Total: {} line items, {} total components",
                style("Summary").bold(),
                assembly.bom.len(),
                assembly.total_component_count()
            );
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&assembly.bom).into_diagnostic()?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yml::to_string(&assembly.bom).into_diagnostic()?;
            print!("{}", yaml);
        }
        OutputFormat::Csv => {
            println!("quantity,component_id,part_number,title,reference_designators,notes");
            for item in &assembly.bom {
                let cmp_info = components.get(&item.component_id);
                let part_number = cmp_info.map(|c| c.part_number.as_str()).unwrap_or("");
                let title = cmp_info.map(|c| c.title.as_str()).unwrap_or("");
                let refs = item.reference_designators.join(";");
                let notes = item.notes.as_deref().unwrap_or("");

                println!(
                    "{},{},{},{},{},{}",
                    item.quantity,
                    item.component_id,
                    escape_csv(part_number),
                    escape_csv(title),
                    escape_csv(&refs),
                    escape_csv(notes)
                );
            }
        }
        _ => {}
    }

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
