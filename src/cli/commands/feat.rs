//! `pdt feat` command - Feature management (dimensional features on components)

use clap::{Subcommand, ValueEnum};
use console::style;
use miette::{IntoDiagnostic, Result};
use std::fs;

use crate::cli::{GlobalOpts, OutputFormat};
use crate::core::entity::Entity;
use crate::core::identity::{EntityId, EntityPrefix};
use crate::core::project::Project;
use crate::core::shortid::ShortIdIndex;
use crate::core::Config;
use crate::entities::feature::{Feature, FeatureType};
use crate::schema::template::{TemplateContext, TemplateGenerator};

#[derive(Subcommand, Debug)]
pub enum FeatCommands {
    /// List features with filtering
    List(ListArgs),

    /// Create a new feature (requires --component)
    New(NewArgs),

    /// Show a feature's details
    Show(ShowArgs),

    /// Edit a feature in your editor
    Edit(EditArgs),
}

/// Feature type filter
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TypeFilter {
    Hole,
    Shaft,
    PlanarSurface,
    Slot,
    Thread,
    Counterbore,
    Countersink,
    Boss,
    Pocket,
    Edge,
    Other,
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
    /// Filter by parent component (CMP@N or full ID)
    #[arg(long, short = 'c')]
    pub component: Option<String>,

    /// Filter by feature type
    #[arg(long, short = 't', default_value = "all")]
    pub feature_type: TypeFilter,

    /// Filter by status
    #[arg(long, short = 's', default_value = "all")]
    pub status: StatusFilter,

    /// Search in title
    #[arg(long)]
    pub search: Option<String>,

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

#[derive(clap::Args, Debug)]
pub struct NewArgs {
    /// Parent component ID (REQUIRED) - CMP@N or full ID
    #[arg(long, short = 'c', required = true)]
    pub component: String,

    /// Feature type
    #[arg(long, short = 't', default_value = "hole")]
    pub feature_type: String,

    /// Title/description
    #[arg(long)]
    pub title: Option<String>,

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
    /// Feature ID or short ID (FEAT@N)
    pub id: String,

    /// Output format
    #[arg(long, short = 'o', default_value = "yaml")]
    pub format: OutputFormat,
}

#[derive(clap::Args, Debug)]
pub struct EditArgs {
    /// Feature ID or short ID (FEAT@N)
    pub id: String,
}

/// Run a feature subcommand
pub fn run(cmd: FeatCommands, _global: &GlobalOpts) -> Result<()> {
    match cmd {
        FeatCommands::List(args) => run_list(args),
        FeatCommands::New(args) => run_new(args),
        FeatCommands::Show(args) => run_show(args),
        FeatCommands::Edit(args) => run_edit(args),
    }
}

fn run_list(args: ListArgs) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;
    let feat_dir = project.root().join("tolerances/features");

    if !feat_dir.exists() {
        if args.count {
            println!("0");
        } else {
            println!("No features found.");
        }
        return Ok(());
    }

    // Resolve component filter if provided
    let short_ids = ShortIdIndex::load(&project);
    let component_filter = args.component.as_ref().map(|c| {
        short_ids.resolve(c).unwrap_or_else(|| c.clone())
    });

    // Load and parse all features
    let mut features: Vec<Feature> = Vec::new();

    for entry in fs::read_dir(&feat_dir).into_diagnostic()? {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "yaml") {
            let content = fs::read_to_string(&path).into_diagnostic()?;
            if let Ok(feat) = serde_yml::from_str::<Feature>(&content) {
                features.push(feat);
            }
        }
    }

    // Apply filters
    let features: Vec<Feature> = features
        .into_iter()
        .filter(|f| {
            if let Some(ref cmp_id) = component_filter {
                f.component.contains(cmp_id) || f.component == *cmp_id
            } else {
                true
            }
        })
        .filter(|f| match args.feature_type {
            TypeFilter::Hole => f.feature_type == FeatureType::Hole,
            TypeFilter::Shaft => f.feature_type == FeatureType::Shaft,
            TypeFilter::PlanarSurface => f.feature_type == FeatureType::PlanarSurface,
            TypeFilter::Slot => f.feature_type == FeatureType::Slot,
            TypeFilter::Thread => f.feature_type == FeatureType::Thread,
            TypeFilter::Counterbore => f.feature_type == FeatureType::Counterbore,
            TypeFilter::Countersink => f.feature_type == FeatureType::Countersink,
            TypeFilter::Boss => f.feature_type == FeatureType::Boss,
            TypeFilter::Pocket => f.feature_type == FeatureType::Pocket,
            TypeFilter::Edge => f.feature_type == FeatureType::Edge,
            TypeFilter::Other => f.feature_type == FeatureType::Other,
            TypeFilter::All => true,
        })
        .filter(|f| match args.status {
            StatusFilter::Draft => f.status == crate::core::entity::Status::Draft,
            StatusFilter::Review => f.status == crate::core::entity::Status::Review,
            StatusFilter::Approved => f.status == crate::core::entity::Status::Approved,
            StatusFilter::Released => f.status == crate::core::entity::Status::Released,
            StatusFilter::Obsolete => f.status == crate::core::entity::Status::Obsolete,
            StatusFilter::All => true,
        })
        .filter(|f| {
            if let Some(ref search) = args.search {
                let search_lower = search.to_lowercase();
                f.title.to_lowercase().contains(&search_lower)
                    || f.description
                        .as_ref()
                        .map_or(false, |d| d.to_lowercase().contains(&search_lower))
            } else {
                true
            }
        })
        .collect();

    // Apply limit
    let mut features = features;
    if let Some(limit) = args.limit {
        features.truncate(limit);
    }

    // Count only
    if args.count {
        println!("{}", features.len());
        return Ok(());
    }

    // No results
    if features.is_empty() {
        println!("No features found.");
        return Ok(());
    }

    // Update short ID index
    let mut short_ids = ShortIdIndex::load(&project);
    short_ids.ensure_all(features.iter().map(|f| f.id.to_string()));
    let _ = short_ids.save(&project);

    // Output based on format
    let format = if args.format == OutputFormat::Auto {
        OutputFormat::Tsv
    } else {
        args.format
    };

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&features).into_diagnostic()?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yml::to_string(&features).into_diagnostic()?;
            print!("{}", yaml);
        }
        OutputFormat::Csv => {
            println!("short_id,id,component,feature_type,title,dims,status");
            for feat in &features {
                let short_id = short_ids.get_short_id(&feat.id.to_string()).unwrap_or_default();
                println!(
                    "{},{},{},{},{},{},{}",
                    short_id,
                    feat.id,
                    truncate_str(&feat.component, 13),
                    feat.feature_type,
                    escape_csv(&feat.title),
                    feat.dimensions.len(),
                    feat.status()
                );
            }
        }
        OutputFormat::Tsv => {
            println!(
                "{:<8} {:<17} {:<15} {:<12} {:<25} {:<5} {:<10}",
                style("SHORT").bold().dim(),
                style("ID").bold(),
                style("COMPONENT").bold(),
                style("TYPE").bold(),
                style("TITLE").bold(),
                style("DIMS").bold(),
                style("STATUS").bold()
            );
            println!("{}", "-".repeat(95));

            for feat in &features {
                let short_id = short_ids.get_short_id(&feat.id.to_string()).unwrap_or_default();
                let id_display = format_short_id(&feat.id);

                println!(
                    "{:<8} {:<17} {:<15} {:<12} {:<25} {:<5} {:<10}",
                    style(&short_id).cyan(),
                    id_display,
                    truncate_str(&feat.component, 13),
                    feat.feature_type,
                    truncate_str(&feat.title, 23),
                    feat.dimensions.len(),
                    feat.status()
                );
            }

            println!();
            println!(
                "{} feature(s) found. Use {} to reference by short ID.",
                style(features.len()).cyan(),
                style("FEAT@N").cyan()
            );
        }
        OutputFormat::Id => {
            for feat in &features {
                println!("{}", feat.id);
            }
        }
        OutputFormat::Md => {
            println!("| Short | ID | Component | Type | Title | Dims | Status |");
            println!("|---|---|---|---|---|---|---|");
            for feat in &features {
                let short_id = short_ids.get_short_id(&feat.id.to_string()).unwrap_or_default();
                println!(
                    "| {} | {} | {} | {} | {} | {} | {} |",
                    short_id,
                    format_short_id(&feat.id),
                    truncate_str(&feat.component, 13),
                    feat.feature_type,
                    feat.title,
                    feat.dimensions.len(),
                    feat.status()
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

    // Resolve component ID
    let short_ids = ShortIdIndex::load(&project);
    let component_id = short_ids.resolve(&args.component).unwrap_or_else(|| args.component.clone());

    // Validate component exists
    let cmp_dir = project.root().join("bom/components");
    let mut component_found = false;
    if cmp_dir.exists() {
        for entry in fs::read_dir(&cmp_dir).into_diagnostic()? {
            let entry = entry.into_diagnostic()?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "yaml") {
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if filename.contains(&component_id) {
                    component_found = true;
                    break;
                }
            }
        }
    }

    if !component_found {
        return Err(miette::miette!(
            "Component '{}' not found. Create it first with: pdt cmp new",
            args.component
        ));
    }

    let title: String;
    let feature_type: String;

    if args.interactive || args.title.is_none() {
        use dialoguer::{Input, Select};

        title = Input::new()
            .with_prompt("Feature title")
            .interact_text()
            .into_diagnostic()?;

        let type_options = ["hole", "shaft", "planar_surface", "slot", "thread", "counterbore", "countersink", "boss", "pocket", "edge", "other"];
        let type_idx = Select::new()
            .with_prompt("Feature type")
            .items(&type_options)
            .default(0)
            .interact()
            .into_diagnostic()?;
        feature_type = type_options[type_idx].to_string();
    } else {
        title = args.title.unwrap();
        feature_type = args.feature_type;
    }

    // Generate ID
    let id = EntityId::new(EntityPrefix::Feat);

    // Generate template
    let generator = TemplateGenerator::new().map_err(|e| miette::miette!("{}", e))?;
    let ctx = TemplateContext::new(id.clone(), config.author())
        .with_title(&title)
        .with_component_id(&component_id)
        .with_feature_type(&feature_type);

    let yaml_content = generator
        .generate_feature(&ctx)
        .map_err(|e| miette::miette!("{}", e))?;

    // Write file
    let output_dir = project.root().join("tolerances/features");
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
        "{} Created feature {}",
        style("✓").green(),
        style(short_id.unwrap_or_else(|| format_short_id(&id))).cyan()
    );
    println!("   {}", style(file_path.display()).dim());
    println!(
        "   Parent: {} | Type: {} | {}",
        style(truncate_str(&component_id, 13)).yellow(),
        style(&feature_type).cyan(),
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

    // Find the feature file
    let feat_dir = project.root().join("tolerances/features");
    let mut found_path = None;

    if feat_dir.exists() {
        for entry in fs::read_dir(&feat_dir).into_diagnostic()? {
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

    let path = found_path.ok_or_else(|| miette::miette!("No feature found matching '{}'", args.id))?;

    // Read and display
    let content = fs::read_to_string(&path).into_diagnostic()?;

    match args.format {
        OutputFormat::Yaml | OutputFormat::Auto => {
            print!("{}", content);
        }
        OutputFormat::Json => {
            let feat: Feature = serde_yml::from_str(&content).into_diagnostic()?;
            let json = serde_json::to_string_pretty(&feat).into_diagnostic()?;
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

    // Find the feature file
    let feat_dir = project.root().join("tolerances/features");
    let mut found_path = None;

    if feat_dir.exists() {
        for entry in fs::read_dir(&feat_dir).into_diagnostic()? {
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

    let path = found_path.ok_or_else(|| miette::miette!("No feature found matching '{}'", args.id))?;

    let editor = config.editor();
    println!("Opening {} in {}...", style(path.display()).cyan(), style(&editor).yellow());

    std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .into_diagnostic()?;

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
