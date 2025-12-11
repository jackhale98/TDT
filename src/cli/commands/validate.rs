//! `tdt validate` command - Validate project files against schemas

use console::style;
use miette::Result;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::core::project::Project;
use crate::core::EntityPrefix;
use crate::entities::feature::Feature;
use crate::entities::mate::{FitAnalysis, Mate};
use crate::entities::risk::Risk;
use crate::entities::stackup::Stackup;
use crate::schema::registry::SchemaRegistry;
use crate::schema::validator::Validator;

#[derive(clap::Args, Debug)]
pub struct ValidateArgs {
    /// Paths to validate (default: entire project)
    #[arg()]
    pub paths: Vec<PathBuf>,

    /// Strict mode - warnings become errors
    #[arg(long)]
    pub strict: bool,

    /// Only validate git-staged files
    #[arg(long)]
    pub staged: bool,

    /// Specific entity types to validate (e.g., req, risk)
    #[arg(long, short = 't')]
    pub entity_type: Option<String>,

    /// Continue validation after first error
    #[arg(long)]
    pub keep_going: bool,

    /// Show summary only, don't show individual errors
    #[arg(long)]
    pub summary: bool,

    /// Fix calculated values (RPN, risk level) in-place
    #[arg(long)]
    pub fix: bool,
}

/// Validation statistics
#[derive(Default)]
struct ValidationStats {
    files_checked: usize,
    files_passed: usize,
    files_failed: usize,
    total_errors: usize,
    total_warnings: usize,
    files_fixed: usize,
}

pub fn run(args: ValidateArgs) -> Result<()> {
    let project = Project::discover().map_err(|e| miette::miette!("{}", e))?;
    let registry = SchemaRegistry::default();
    let validator = Validator::new(&registry);

    let mut stats = ValidationStats::default();
    let mut had_error = false;

    // Determine which files to validate
    let files_to_validate: Vec<PathBuf> = if args.staged {
        get_staged_files(&project)?
    } else if args.paths.is_empty() {
        get_all_tdt_files(&project)
    } else {
        expand_paths(&args.paths)
    };

    // Filter by entity type if specified
    let entity_filter: Option<EntityPrefix> = args.entity_type.as_ref().and_then(|t| {
        t.to_uppercase().parse().ok()
    });

    println!(
        "{} Validating {} file(s)...\n",
        style("→").blue(),
        files_to_validate.len()
    );

    for path in &files_to_validate {
        // Skip non-.tdt.yaml files
        if !path.to_string_lossy().ends_with(".tdt.yaml") {
            continue;
        }

        // Determine entity type from path
        let prefix = EntityPrefix::from_filename(&path.file_name().unwrap_or_default().to_string_lossy())
            .or_else(|| EntityPrefix::from_path(path));

        // Skip if filtering by entity type and this doesn't match
        if let Some(filter) = entity_filter {
            if prefix != Some(filter) {
                continue;
            }
        }

        stats.files_checked += 1;

        // Read file content
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                if !args.summary {
                    println!(
                        "{} {} - {}",
                        style("✗").red(),
                        path.display(),
                        e
                    );
                }
                stats.files_failed += 1;
                stats.total_errors += 1;
                had_error = true;
                if !args.keep_going {
                    break;
                }
                continue;
            }
        };

        let filename = path.file_name().unwrap_or_default().to_string_lossy();

        // Skip if we can't determine entity type
        let entity_prefix = match prefix {
            Some(p) => p,
            None => {
                if !args.summary {
                    println!(
                        "{} {} - {}",
                        style("?").yellow(),
                        path.display(),
                        "unknown entity type (skipped)"
                    );
                }
                continue;
            }
        };

        // Validate schema
        match validator.iter_errors(&content, &filename, entity_prefix) {
            Ok(_) => {
                // Schema validation passed - now check calculated values
                let calc_issues = match entity_prefix {
                    EntityPrefix::Risk => {
                        check_risk_calculations(&content, path, args.fix, &mut stats)?
                    }
                    EntityPrefix::Mate => {
                        check_mate_values(&content, path, args.fix, &mut stats, project.root())?
                    }
                    EntityPrefix::Tol => {
                        check_stackup_values(&content, path, args.fix, &mut stats, project.root())?
                    }
                    _ => vec![],
                };

                if calc_issues.is_empty() {
                    stats.files_passed += 1;
                    if !args.summary {
                        println!(
                            "{} {}",
                            style("✓").green(),
                            path.display()
                        );
                    }
                } else {
                    // Has calculation issues but schema is valid
                    if args.fix {
                        stats.files_passed += 1;
                        if !args.summary {
                            println!(
                                "{} {} (fixed)",
                                style("✓").green(),
                                path.display()
                            );
                        }
                    } else {
                        stats.total_warnings += calc_issues.len();
                        if !args.summary {
                            println!(
                                "{} {} - {} calculation warning(s)",
                                style("!").yellow(),
                                path.display(),
                                calc_issues.len()
                            );
                            for issue in &calc_issues {
                                println!("    {}", style(issue).yellow());
                            }
                        }
                        if args.strict {
                            stats.files_failed += 1;
                            had_error = true;
                        } else {
                            stats.files_passed += 1;
                        }
                    }
                }
            }
            Err(e) => {
                stats.files_failed += 1;
                stats.total_errors += e.violation_count();
                had_error = true;

                if !args.summary {
                    println!(
                        "{} {} - {} error(s)",
                        style("✗").red(),
                        path.display(),
                        e.violation_count()
                    );

                    // Print detailed error using miette
                    let report = miette::Report::new(e);
                    println!("{:?}", report);
                }

                if !args.keep_going {
                    break;
                }
            }
        }
    }

    // Print summary
    println!();
    println!("{}", style("─".repeat(60)).dim());
    println!(
        "{}",
        style("Validation Summary").bold()
    );
    println!("{}", style("─".repeat(60)).dim());
    println!(
        "  Files checked:  {}",
        style(stats.files_checked).cyan()
    );
    println!(
        "  Files passed:   {}",
        style(stats.files_passed).green()
    );
    println!(
        "  Files failed:   {}",
        style(stats.files_failed).red()
    );
    println!(
        "  Total errors:   {}",
        style(stats.total_errors).red()
    );

    if stats.total_warnings > 0 {
        println!(
            "  Total warnings: {}",
            style(stats.total_warnings).yellow()
        );
    }

    if stats.files_fixed > 0 {
        println!(
            "  Files fixed:    {}",
            style(stats.files_fixed).cyan()
        );
    }

    println!();

    if had_error {
        if stats.files_failed == 1 {
            Err(miette::miette!(
                "Validation failed: 1 file has errors"
            ))
        } else {
            Err(miette::miette!(
                "Validation failed: {} files have errors",
                stats.files_failed
            ))
        }
    } else {
        println!(
            "{} All files passed validation!",
            style("✓").green().bold()
        );
        Ok(())
    }
}

/// Get all .tdt.yaml files in the project
fn get_all_tdt_files(project: &Project) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for entry in WalkDir::new(project.root())
        .into_iter()
        .filter_entry(|e| {
            // Skip .git and .tdt directories
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') || e.depth() == 0
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if path.to_string_lossy().ends_with(".tdt.yaml") {
            files.push(path.to_path_buf());
        }
    }

    files.sort();
    files
}

/// Get git-staged .tdt.yaml files
fn get_staged_files(project: &Project) -> Result<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACM"])
        .current_dir(project.root())
        .output()
        .map_err(|e| miette::miette!("Failed to run git: {}", e))?;

    if !output.status.success() {
        return Err(miette::miette!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let files: Vec<PathBuf> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.ends_with(".tdt.yaml"))
        .map(|line| project.root().join(line))
        .filter(|path| path.exists())
        .collect();

    Ok(files)
}

/// Expand paths - if a directory is given, find all .tdt.yaml files in it
fn expand_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_dir() {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                if entry.path().to_string_lossy().ends_with(".tdt.yaml") {
                    files.push(entry.path().to_path_buf());
                }
            }
        } else if path.exists() {
            files.push(path.clone());
        }
    }

    files.sort();
    files
}

/// Check and optionally fix calculated values in RISK entities
fn check_risk_calculations(
    content: &str,
    path: &PathBuf,
    fix: bool,
    stats: &mut ValidationStats,
) -> Result<Vec<String>> {
    let mut issues = Vec::new();

    // Parse the risk
    let risk: Risk = match serde_yml::from_str(content) {
        Ok(r) => r,
        Err(_) => return Ok(issues), // Already reported by schema validation
    };

    // Check RPN calculation
    if let Some(expected_rpn) = risk.calculate_rpn() {
        if let Some(actual_rpn) = risk.rpn {
            if actual_rpn != expected_rpn {
                issues.push(format!(
                    "RPN mismatch: stored {} but calculated {} ({}×{}×{})",
                    actual_rpn,
                    expected_rpn,
                    risk.severity.unwrap_or(0),
                    risk.occurrence.unwrap_or(0),
                    risk.detection.unwrap_or(0)
                ));
            }
        }
    }

    // Check risk level calculation
    if let Some(expected_level) = risk.determine_risk_level() {
        if let Some(actual_level) = risk.risk_level {
            if actual_level != expected_level {
                issues.push(format!(
                    "risk_level mismatch: stored '{}' but calculated '{}'",
                    actual_level, expected_level
                ));
            }
        }
    }

    // Fix if requested and there are issues
    if fix && !issues.is_empty() {
        // Re-parse as a mutable value to fix
        let mut value: serde_yml::Value = serde_yml::from_str(content)
            .map_err(|e| miette::miette!("Failed to re-parse YAML: {}", e))?;

        // Update RPN
        if let Some(expected_rpn) = risk.calculate_rpn() {
            value["rpn"] = serde_yml::Value::Number(expected_rpn.into());

            // Update risk level based on the calculated RPN
            let expected_level = match expected_rpn {
                0..=50 => "low",
                51..=150 => "medium",
                151..=400 => "high",
                _ => "critical",
            };
            value["risk_level"] = serde_yml::Value::String(expected_level.to_string());
        }

        // Write back
        let updated_content = serde_yml::to_string(&value)
            .map_err(|e| miette::miette!("Failed to serialize YAML: {}", e))?;
        fs::write(path, updated_content)
            .map_err(|e| miette::miette!("Failed to write file: {}", e))?;

        stats.files_fixed += 1;
        issues.clear(); // Clear issues since we fixed them
    }

    Ok(issues)
}

/// Load a feature entity by ID from the project
fn load_feature(feature_id: &str, project_root: &Path) -> Option<Feature> {
    // Find the feature file by searching in the tolerances/features directory
    let features_dir = project_root.join("tolerances").join("features");

    if !features_dir.exists() {
        return None;
    }

    for entry in WalkDir::new(&features_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if !path.to_string_lossy().ends_with(".tdt.yaml") {
            continue;
        }

        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(feature) = serde_yml::from_str::<Feature>(&content) {
                if feature.id.to_string() == feature_id {
                    return Some(feature);
                }
            }
        }
    }

    None
}

/// Check and optionally fix calculated values in MATE entities
fn check_mate_values(
    content: &str,
    path: &PathBuf,
    fix: bool,
    stats: &mut ValidationStats,
    project_root: &Path,
) -> Result<Vec<String>> {
    let mut issues = Vec::new();

    // Parse the mate
    let mate: Mate = match serde_yml::from_str(content) {
        Ok(m) => m,
        Err(_) => return Ok(issues), // Already reported by schema validation
    };

    // Load linked features
    let feat_a = match load_feature(&mate.feature_a, project_root) {
        Some(f) => f,
        None => {
            issues.push(format!("Cannot find feature_a: {}", mate.feature_a));
            return Ok(issues);
        }
    };

    let feat_b = match load_feature(&mate.feature_b, project_root) {
        Some(f) => f,
        None => {
            issues.push(format!("Cannot find feature_b: {}", mate.feature_b));
            return Ok(issues);
        }
    };

    // Get primary dimensions
    let dim_a = match feat_a.primary_dimension() {
        Some(d) => d,
        None => {
            issues.push(format!("Feature {} has no dimension", mate.feature_a));
            return Ok(issues);
        }
    };

    let dim_b = match feat_b.primary_dimension() {
        Some(d) => d,
        None => {
            issues.push(format!("Feature {} has no dimension", mate.feature_b));
            return Ok(issues);
        }
    };

    // Check that features form a valid mate (one internal, one external)
    if dim_a.internal == dim_b.internal {
        if dim_a.internal {
            issues.push("Both features are internal - mate requires one internal and one external".to_string());
        } else {
            issues.push("Both features are external - mate requires one internal and one external".to_string());
        }
        return Ok(issues);
    }

    // Calculate expected fit analysis
    let expected_analysis = match FitAnalysis::from_dimensions(dim_a, dim_b) {
        Ok(a) => a,
        Err(e) => {
            issues.push(format!("Cannot calculate fit: {}", e));
            return Ok(issues);
        }
    };

    // Compare with stored analysis
    if let Some(actual) = &mate.fit_analysis {
        let min_diff = (actual.worst_case_min_clearance - expected_analysis.worst_case_min_clearance).abs();
        let max_diff = (actual.worst_case_max_clearance - expected_analysis.worst_case_max_clearance).abs();

        if min_diff > 1e-6 || max_diff > 1e-6 || actual.fit_result != expected_analysis.fit_result {
            issues.push(format!(
                "fit_analysis mismatch: stored ({:.4} to {:.4}, {}) but calculated ({:.4} to {:.4}, {})",
                actual.worst_case_min_clearance,
                actual.worst_case_max_clearance,
                actual.fit_result,
                expected_analysis.worst_case_min_clearance,
                expected_analysis.worst_case_max_clearance,
                expected_analysis.fit_result
            ));
        }
    } else {
        issues.push("fit_analysis not calculated".to_string());
    }

    // Fix if requested and there are issues
    if fix && !issues.is_empty() {
        let mut value: serde_yml::Value = serde_yml::from_str(content)
            .map_err(|e| miette::miette!("Failed to re-parse YAML: {}", e))?;

        // Update fit_analysis
        value["fit_analysis"] = serde_yml::to_value(&expected_analysis)
            .map_err(|e| miette::miette!("Failed to serialize fit_analysis: {}", e))?;

        // Write back
        let updated_content = serde_yml::to_string(&value)
            .map_err(|e| miette::miette!("Failed to serialize YAML: {}", e))?;
        fs::write(path, updated_content)
            .map_err(|e| miette::miette!("Failed to write file: {}", e))?;

        stats.files_fixed += 1;
        issues.clear();
    }

    Ok(issues)
}

/// Check and optionally fix contributor values in stackup entities
fn check_stackup_values(
    content: &str,
    path: &PathBuf,
    fix: bool,
    stats: &mut ValidationStats,
    project_root: &Path,
) -> Result<Vec<String>> {
    let mut issues = Vec::new();

    // Parse the stackup
    let mut stackup: Stackup = match serde_yml::from_str(content) {
        Ok(s) => s,
        Err(_) => return Ok(issues), // Already reported by schema validation
    };

    let mut any_synced = false;

    // Check each contributor that has a feature_id
    for contributor in stackup.contributors.iter_mut() {
        if let Some(feature_id) = &contributor.feature_id {
            if let Some(feature) = load_feature(feature_id, project_root) {
                if contributor.is_out_of_sync(&feature) {
                    if fix {
                        contributor.sync_from_feature(&feature);
                        any_synced = true;
                    } else {
                        if let Some(dim) = feature.primary_dimension() {
                            issues.push(format!(
                                "Contributor '{}' out of sync with {}: stored ({:.4} +{:.4}/-{:.4}) vs feature ({:.4} +{:.4}/-{:.4})",
                                contributor.name,
                                feature_id,
                                contributor.nominal,
                                contributor.plus_tol,
                                contributor.minus_tol,
                                dim.nominal,
                                dim.plus_tol,
                                dim.minus_tol
                            ));
                        }
                    }
                }
            } else {
                issues.push(format!(
                    "Contributor '{}' references unknown feature: {}",
                    contributor.name, feature_id
                ));
            }
        }
    }

    // Write back if we synced any contributors
    if fix && any_synced {
        let updated_content = serde_yml::to_string(&stackup)
            .map_err(|e| miette::miette!("Failed to serialize YAML: {}", e))?;
        fs::write(path, updated_content)
            .map_err(|e| miette::miette!("Failed to write file: {}", e))?;

        stats.files_fixed += 1;
    }

    Ok(issues)
}
