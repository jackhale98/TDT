//! `tdt import` command - Import entities from CSV files

mod asm;
mod capa;
mod cmp;
mod common;
mod ctrl;
mod feat;
mod ncr;
mod proc;
mod quote;
mod req;
mod risk;
mod rslt;
mod sup;
mod test;

use console::style;
use miette::Result;
use std::path::PathBuf;

use crate::core::identity::EntityPrefix;
use crate::core::project::Project;

pub use common::{generate_template, ImportStats};

#[derive(clap::Args, Debug)]
pub struct ImportArgs {
    /// Entity type to import (req, risk, cmp, sup, feat, rslt, asm, etc.)
    #[arg(value_parser = parse_entity_type)]
    pub entity_type: Option<EntityPrefix>,

    /// CSV file to import
    pub file: Option<PathBuf>,

    /// Generate a CSV template for the entity type
    #[arg(long)]
    pub template: bool,

    /// Validate CSV without creating files
    #[arg(long)]
    pub dry_run: bool,

    /// Continue importing after errors (default: stop on first error)
    #[arg(long)]
    pub skip_errors: bool,

    /// Update existing entities if ID column matches
    #[arg(long)]
    pub update: bool,

    /// Default component ID for feature imports (used when CSV row lacks component column)
    #[arg(long)]
    pub component: Option<String>,

    /// Default supplier ID for quote imports (used when CSV row lacks supplier column)
    #[arg(long)]
    pub supplier: Option<String>,

    /// Default test ID for result imports (used when CSV row lacks test column)
    #[arg(long)]
    pub test: Option<String>,

    /// Default process ID for control imports (used when CSV row lacks process column)
    #[arg(long)]
    pub process: Option<String>,

    /// Default assembly ID for component imports (used when CSV row lacks assembly column)
    #[arg(long)]
    pub assembly: Option<String>,
}

fn parse_entity_type(s: &str) -> Result<EntityPrefix, String> {
    match s.to_lowercase().as_str() {
        "req" => Ok(EntityPrefix::Req),
        "risk" => Ok(EntityPrefix::Risk),
        "cmp" => Ok(EntityPrefix::Cmp),
        "asm" => Ok(EntityPrefix::Asm),
        "sup" => Ok(EntityPrefix::Sup),
        "test" => Ok(EntityPrefix::Test),
        "rslt" | "result" => Ok(EntityPrefix::Rslt),
        "proc" => Ok(EntityPrefix::Proc),
        "ctrl" => Ok(EntityPrefix::Ctrl),
        "ncr" => Ok(EntityPrefix::Ncr),
        "capa" => Ok(EntityPrefix::Capa),
        "quote" | "quot" => Ok(EntityPrefix::Quot),
        "feat" | "feature" => Ok(EntityPrefix::Feat),
        _ => Err(format!(
            "Unsupported entity type: '{}'. Supported: req, risk, cmp, asm, sup, test, rslt, proc, ctrl, ncr, capa, quote, feat",
            s
        )),
    }
}

pub fn run(args: ImportArgs) -> Result<()> {
    // Handle template generation
    if args.template {
        let entity_type = args.entity_type.ok_or_else(|| {
            miette::miette!(
                "Entity type required for template generation. Usage: tdt import --template req"
            )
        })?;
        return generate_template(entity_type);
    }

    // Require both entity type and file for import
    let entity_type = args
        .entity_type
        .ok_or_else(|| miette::miette!("Entity type required. Usage: tdt import req data.csv"))?;

    let file_path = args
        .file
        .clone()
        .ok_or_else(|| miette::miette!("CSV file required. Usage: tdt import req data.csv"))?;

    if !file_path.exists() {
        return Err(miette::miette!("File not found: {}", file_path.display()));
    }

    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;

    println!(
        "{} Importing {} entities from {}{}",
        style("→").blue(),
        style(entity_type.as_str()).cyan(),
        style(file_path.display()).yellow(),
        if args.dry_run {
            style(" (dry run)").dim().to_string()
        } else {
            String::new()
        }
    );
    println!();

    // Convert clap args to internal args struct
    let internal_args = common::ImportArgs {
        dry_run: args.dry_run,
        skip_errors: args.skip_errors,
        component: args.component.clone(),
        supplier: args.supplier.clone(),
        test: args.test.clone(),
        process: args.process.clone(),
        assembly: args.assembly.clone(),
    };

    let stats = match entity_type {
        EntityPrefix::Req => req::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Risk => risk::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Cmp => cmp::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Asm => asm::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Sup => sup::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Test => test::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Rslt => rslt::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Proc => proc::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Ctrl => ctrl::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Ncr => ncr::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Capa => capa::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Quot => quote::import(&project, &file_path, &internal_args)?,
        EntityPrefix::Feat => feat::import(&project, &file_path, &internal_args)?,
        _ => {
            return Err(miette::miette!(
                "Import not yet implemented for {}",
                entity_type.as_str()
            ));
        }
    };

    // Print summary
    println!();
    println!("{}", style("─".repeat(50)).dim());
    println!("{}", style("Import Summary").bold());
    println!("{}", style("─".repeat(50)).dim());
    println!("  Rows processed:   {}", style(stats.rows_processed).cyan());
    println!(
        "  Entities created: {}",
        style(stats.entities_created).green()
    );
    if stats.entities_updated > 0 {
        println!(
            "  Entities updated: {}",
            style(stats.entities_updated).yellow()
        );
    }
    if stats.errors > 0 {
        println!("  Errors:           {}", style(stats.errors).red());
    }
    if stats.skipped > 0 {
        println!("  Skipped:          {}", style(stats.skipped).dim());
    }

    if args.dry_run {
        println!();
        println!(
            "{}",
            style("Dry run complete. No files were created.").yellow()
        );
    }

    if stats.errors > 0 && !args.skip_errors {
        return Err(miette::miette!(
            "Import completed with {} error(s)",
            stats.errors
        ));
    }

    Ok(())
}
