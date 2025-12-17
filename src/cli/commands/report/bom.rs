//! BOM (Bill of Materials) report

use miette::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::GlobalOpts;
use crate::core::project::Project;
use crate::core::shortid::ShortIdIndex;
use crate::entities::component::Component;
use crate::entities::quote::Quote;

use super::{
    load_all_assemblies, load_all_components, load_all_quotes, load_assembly, write_output,
};

#[derive(clap::Args, Debug)]
pub struct BomArgs {
    /// Assembly ID to generate BOM for
    pub assembly_id: String,

    /// Output to file instead of stdout
    #[arg(long, short = 'o')]
    pub output: Option<PathBuf>,

    /// Include cost rollup
    #[arg(long)]
    pub with_cost: bool,

    /// Include mass rollup
    #[arg(long)]
    pub with_mass: bool,
}

pub fn run(args: BomArgs, _global: &GlobalOpts) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;
    let short_ids = ShortIdIndex::load(&project);

    // Resolve assembly ID
    let resolved_id = short_ids
        .resolve(&args.assembly_id)
        .unwrap_or_else(|| args.assembly_id.clone());

    // Load assembly
    let assembly = load_assembly(&project, &resolved_id)?;

    // Load all components for lookup
    let components = load_all_components(&project);
    let component_map: HashMap<String, &Component> =
        components.iter().map(|c| (c.id.to_string(), c)).collect();

    // Load all assemblies for subassembly lookup
    let assemblies = load_all_assemblies(&project);
    let assembly_map: HashMap<String, &crate::entities::assembly::Assembly> =
        assemblies.iter().map(|a| (a.id.to_string(), a)).collect();

    // Load quotes for price lookup (used when --with-cost)
    let quotes = load_all_quotes(&project);
    let quote_map: HashMap<String, &Quote> = quotes.iter().map(|q| (q.id.to_string(), q)).collect();

    // Generate indented BOM
    let mut output = String::new();
    output.push_str(&format!("# Bill of Materials: {}\n\n", assembly.title));
    output.push_str(&format!("Assembly ID: {}\n", assembly.id));
    output.push_str(&format!("Part Number: {}\n\n", assembly.part_number));

    let mut total_cost = 0.0;
    let mut total_mass = 0.0;

    output.push_str("```\n");

    // Recursively print BOM
    fn print_bom_item(
        output: &mut String,
        component_map: &HashMap<String, &Component>,
        assembly_map: &HashMap<String, &crate::entities::assembly::Assembly>,
        quote_map: &HashMap<String, &Quote>,
        short_ids: &ShortIdIndex,
        bom: &[crate::entities::assembly::BomItem],
        indent: usize,
        total_cost: &mut f64,
        total_mass: &mut f64,
        with_cost: bool,
        with_mass: bool,
        visited: &mut std::collections::HashSet<String>,
    ) {
        let prefix = "│  ".repeat(indent);
        for (i, item) in bom.iter().enumerate() {
            let is_last = i == bom.len() - 1;
            let branch = if is_last { "└─ " } else { "├─ " };

            let item_id = item.component_id.to_string();
            let item_short = short_ids
                .get_short_id(&item_id)
                .unwrap_or_else(|| item_id.clone());

            // Check if it's a component or subassembly
            if let Some(cmp) = component_map.get(&item_id) {
                let cost_str = if with_cost {
                    // Priority 1: Use selected quote if set
                    let unit_price = if let Some(ref quote_id) = cmp.selected_quote {
                        if let Some(quote) = quote_map.get(quote_id) {
                            quote.price_for_qty(item.quantity).unwrap_or(0.0)
                        } else {
                            cmp.unit_cost.unwrap_or(0.0)
                        }
                    } else {
                        // Priority 2: Fall back to unit_cost
                        cmp.unit_cost.unwrap_or(0.0)
                    };

                    if unit_price > 0.0 {
                        let line_cost = unit_price * item.quantity as f64;
                        *total_cost += line_cost;
                        format!(" ${:.2}", line_cost)
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                };

                let mass_str = if with_mass {
                    cmp.mass_kg.map_or("".to_string(), |m| {
                        let line_mass = m * item.quantity as f64;
                        *total_mass += line_mass;
                        format!(" {:.3}kg", line_mass)
                    })
                } else {
                    "".to_string()
                };

                output.push_str(&format!(
                    "{}{}{}: {} (qty: {}){}{}\n",
                    prefix, branch, item_short, cmp.title, item.quantity, cost_str, mass_str
                ));
            } else if let Some(asm) = assembly_map.get(&item_id) {
                // Subassembly - check for cycles
                if visited.contains(&item_id) {
                    output.push_str(&format!(
                        "{}{}{}: {} (qty: {}) [CYCLE DETECTED]\n",
                        prefix, branch, item_short, asm.title, item.quantity
                    ));
                } else {
                    output.push_str(&format!(
                        "{}{}{}: {} (qty: {})\n",
                        prefix, branch, item_short, asm.title, item.quantity
                    ));

                    visited.insert(item_id.clone());
                    print_bom_item(
                        output,
                        component_map,
                        assembly_map,
                        quote_map,
                        short_ids,
                        &asm.bom,
                        indent + 1,
                        total_cost,
                        total_mass,
                        with_cost,
                        with_mass,
                        visited,
                    );
                    visited.remove(&item_id);
                }
            } else {
                output.push_str(&format!(
                    "{}{}{}: (not found) (qty: {})\n",
                    prefix, branch, item_short, item.quantity
                ));
            }
        }
    }

    let mut visited = std::collections::HashSet::new();
    visited.insert(assembly.id.to_string());
    print_bom_item(
        &mut output,
        &component_map,
        &assembly_map,
        &quote_map,
        &short_ids,
        &assembly.bom,
        0,
        &mut total_cost,
        &mut total_mass,
        args.with_cost,
        args.with_mass,
        &mut visited,
    );

    output.push_str("```\n");

    // Totals
    if args.with_cost {
        output.push_str(&format!("\n**Total Cost:** ${:.2}\n", total_cost));
    }
    if args.with_mass {
        output.push_str(&format!("**Total Mass:** {:.3} kg\n", total_mass));
    }

    write_output(&output, args.output)?;
    Ok(())
}
