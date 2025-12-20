//! Approve command - Approve entities under review

use clap::Args;
use miette::{bail, IntoDiagnostic, Result};
use std::io::{self, BufRead};
use std::path::PathBuf;

use crate::cli::args::GlobalOpts;
use crate::core::entity::Status;
use crate::core::shortid::ShortIdIndex;
use crate::core::workflow::{
    get_entity_info, get_prefix_from_id, record_approval, truncate_id,
};
use crate::core::{Config, Git, Project, Provider, ProviderClient, TeamRoster, WorkflowEngine};

/// Approve entities under review
#[derive(Debug, Args)]
pub struct ApproveArgs {
    /// Entity IDs to approve (accepts multiple, or - for stdin)
    #[arg(required_unless_present = "pr")]
    pub ids: Vec<String>,

    /// Approve all entities in a PR by PR number
    #[arg(long)]
    pub pr: Option<u64>,

    /// Approval comment/message
    #[arg(long, short = 'm')]
    pub message: Option<String>,

    /// Merge PR after approval
    #[arg(long)]
    pub merge: bool,

    /// Skip merge even if auto_merge enabled
    #[arg(long)]
    pub no_merge: bool,

    /// Skip authorization check (admin only)
    #[arg(long)]
    pub force: bool,

    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Show what would be done without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Print commands as they run
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

impl ApproveArgs {
    pub fn run(&self, _global: &GlobalOpts) -> Result<()> {
        let project = Project::discover().into_diagnostic()?;
        let config = Config::load();

        // Check if workflow is enabled
        if !config.workflow.enabled {
            bail!(
                "Workflow features are not enabled.\n\
                 Add the following to .tdt/config.yaml:\n\n\
                 workflow:\n\
                 \x20 enabled: true\n\
                 \x20 provider: github  # or gitlab, or none"
            );
        }

        let git = Git::new(project.root());

        // Verify we're in a git repo
        if !git.is_repo() {
            bail!("Not a git repository.");
        }

        // Load team roster (optional)
        let roster = TeamRoster::load(&project);
        let engine = WorkflowEngine::new(roster.clone(), config.workflow.clone());

        // Get current user
        let current_user = engine.current_user();
        let approver_name = current_user
            .map(|u| u.name.clone())
            .or_else(|| git.user_name().ok())
            .unwrap_or_else(|| "Unknown".to_string());

        let approver_role = current_user.and_then(|u| u.roles.first().copied());

        if self.verbose {
            if let Some(user) = current_user {
                eprintln!(
                    "Approving as {} ({:?})",
                    user.name,
                    user.roles
                );
            }
        }

        // Collect entity IDs
        let ids = self.collect_entity_ids()?;
        if ids.is_empty() {
            bail!("No entities to approve. Specify IDs or use --pr");
        }

        // Resolve short IDs to full IDs and validate
        let short_index = ShortIdIndex::load(&project);
        let mut entities: Vec<(PathBuf, String, String, Status)> = Vec::new();

        for id in &ids {
            let full_id = short_index.resolve(id)
                .ok_or_else(|| miette::miette!("Cannot resolve ID: {}", id))?;
            let file_path = self.find_entity_file(&project, &full_id)?;
            let (entity_id, title, status) = get_entity_info(&file_path).into_diagnostic()?;

            if status != Status::Review {
                bail!(
                    "Entity {} is not in review status (current: {})",
                    entity_id,
                    status
                );
            }

            // Check authorization
            if !self.force {
                if let Some(prefix) = get_prefix_from_id(&entity_id) {
                    if let Err(e) = engine.can_transition(Status::Review, Status::Approved, prefix, current_user) {
                        bail!("{}", e);
                    }
                }
            }

            entities.push((file_path, entity_id, title, status));
        }

        // Show what we're about to do
        println!(
            "Approving {} entities as {}...",
            entities.len(),
            approver_name
        );
        if self.verbose || self.dry_run {
            for (_, id, title, _) in &entities {
                println!("  {}  {}", truncate_id(id), title);
            }
        }

        if self.dry_run {
            self.print_dry_run(&entities, &config)?;
            println!("\nNo changes made (dry run).");
            return Ok(());
        }

        // Confirm if not --yes
        if !self.yes {
            print!("Proceed? [y/N] ");
            std::io::Write::flush(&mut std::io::stdout()).into_diagnostic()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).into_diagnostic()?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Aborted.");
                return Ok(());
            }
        }

        // Execute the approval
        self.execute_approve(&project, &config, &git, &entities, &approver_name, approver_role)?;

        Ok(())
    }

    fn collect_entity_ids(&self) -> Result<Vec<String>> {
        // Check for stdin
        if self.ids.len() == 1 && self.ids[0] == "-" {
            let stdin = io::stdin();
            return Ok(stdin
                .lock()
                .lines()
                .map_while(Result::ok)
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect());
        }

        // TODO: If --pr is set, fetch PR and extract entity IDs from branch name
        if self.pr.is_some() {
            bail!("--pr flag not yet implemented. Please specify entity IDs directly.");
        }

        // Otherwise use provided IDs
        Ok(self.ids.clone())
    }

    fn find_entity_file(&self, project: &Project, id: &str) -> Result<PathBuf> {
        use walkdir::WalkDir;

        let file_name = format!("{}.tdt.yaml", id);

        for entry in WalkDir::new(project.root())
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name().to_string_lossy() == file_name {
                return Ok(entry.path().to_path_buf());
            }
        }

        bail!("Entity file not found: {}", id)
    }

    fn print_dry_run(
        &self,
        entities: &[(PathBuf, String, String, Status)],
        config: &Config,
    ) -> Result<()> {
        println!("\nWould execute:");

        for (path, _id, _, _) in entities {
            let rel_path = path
                .strip_prefix(std::env::current_dir().into_diagnostic()?)
                .unwrap_or(path)
                .display();
            println!("  [record approval in {}]", rel_path);
            println!("  git add {}", rel_path);
        }

        let commit_message = if entities.len() == 1 {
            let (_, id, title, _) = &entities[0];
            config.workflow.format_approve_message(&truncate_id(id), title)
        } else {
            format!("Approve {} entities", entities.len())
        };
        println!("  git commit -m \"{}\"", commit_message);

        if config.workflow.provider != Provider::None {
            if let Some(pr) = self.pr {
                let provider = ProviderClient::new(config.workflow.provider, std::path::Path::new("."));
                println!("  {}", provider.format_command(&["pr", "review", &pr.to_string(), "--approve"]));
                if self.merge || (config.workflow.auto_merge && !self.no_merge) {
                    println!("  {}", provider.format_command(&["pr", "merge", &pr.to_string()]));
                }
            }
        }

        Ok(())
    }

    fn execute_approve(
        &self,
        project: &Project,
        config: &Config,
        git: &Git,
        entities: &[(PathBuf, String, String, Status)],
        approver_name: &str,
        approver_role: Option<crate::core::team::Role>,
    ) -> Result<()> {
        // Record approval in each entity
        for (path, id, _, _) in entities {
            record_approval(path, approver_name, approver_role, self.message.as_deref()).into_diagnostic()?;
            if self.verbose {
                eprintln!("  Recorded approval in {}", truncate_id(id));
            }
        }
        println!(
            "  Approved {} entities by {}",
            entities.len(),
            approver_name
        );

        // Stage files
        let paths: Vec<&std::path::Path> = entities.iter().map(|(p, _, _, _)| p.as_path()).collect();
        git.stage_files(&paths).into_diagnostic()?;

        // Commit
        let commit_message = if entities.len() == 1 {
            let (_, id, title, _) = &entities[0];
            config.workflow.format_approve_message(&truncate_id(id), title)
        } else {
            format!("Approve {} entities", entities.len())
        };
        let _hash = git.commit(&commit_message).into_diagnostic()?;
        println!("  Committed: \"{}\"", commit_message);

        // PR operations if provider is configured
        if config.workflow.provider != Provider::None {
            let provider = ProviderClient::new(config.workflow.provider, project.root())
                .with_verbose(self.verbose);

            // Find PR for current branch
            let current_branch = git.current_branch().unwrap_or_default();
            if let Ok(Some(pr_info)) = provider.get_pr_for_branch(&current_branch) {
                // Add approval review
                if let Err(e) = provider.approve_pr(pr_info.number, self.message.as_deref()) {
                    eprintln!("  Warning: Failed to add PR approval: {}", e);
                } else {
                    println!("  Added approval to PR #{}", pr_info.number);
                }

                // Merge if requested
                let should_merge = self.merge || (config.workflow.auto_merge && !self.no_merge);
                if should_merge {
                    if let Err(e) = provider.merge_pr(pr_info.number, true) {
                        eprintln!("  Warning: Failed to merge PR: {}", e);
                    } else {
                        println!("  Merged PR #{}", pr_info.number);
                    }
                }
            }
        }

        println!("\n{} entities approved.", entities.len());

        Ok(())
    }
}
