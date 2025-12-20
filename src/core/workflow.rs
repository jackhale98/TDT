//! Workflow engine for status transitions and approvals
//!
//! Provides status transition validation and YAML manipulation for entity approval workflows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

use crate::core::entity::Status;
use crate::core::identity::EntityPrefix;
use crate::core::provider::Provider;
use crate::core::team::{Role, TeamMember, TeamRoster};

/// Workflow configuration from project config
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WorkflowConfig {
    /// Whether workflow features are enabled
    pub enabled: bool,

    /// Git hosting provider (github, gitlab, or none)
    pub provider: Provider,

    /// Require feature branch for submit (default: true)
    pub require_branch: bool,

    /// Auto-commit on status change (default: true)
    pub auto_commit: bool,

    /// Merge PR on approve (default: false)
    pub auto_merge: bool,

    /// Branch naming pattern (default: "review/{prefix}-{short_id}")
    pub branch_pattern: String,

    /// Commit message for submit
    pub submit_message: String,

    /// Commit message for approve
    pub approve_message: String,

    /// Target branch for PRs (default: "main")
    pub base_branch: String,
}

impl WorkflowConfig {
    /// Create workflow config with sensible defaults
    pub fn with_defaults() -> Self {
        Self {
            enabled: false,
            provider: Provider::None,
            require_branch: true,
            auto_commit: true,
            auto_merge: false,
            branch_pattern: "review/{prefix}-{short_id}".to_string(),
            submit_message: "Submit {id}: {title}".to_string(),
            approve_message: "Approve {id}: {title}".to_string(),
            base_branch: "main".to_string(),
        }
    }

    /// Format a branch name for the given entity
    pub fn format_branch(&self, prefix: &str, short_id: &str) -> String {
        self.branch_pattern
            .replace("{prefix}", prefix)
            .replace("{short_id}", short_id)
    }

    /// Format a commit message for submit
    pub fn format_submit_message(&self, id: &str, title: &str) -> String {
        self.submit_message
            .replace("{id}", id)
            .replace("{title}", title)
    }

    /// Format a commit message for approval
    pub fn format_approve_message(&self, id: &str, title: &str) -> String {
        self.approve_message
            .replace("{id}", id)
            .replace("{title}", title)
    }
}

/// Errors that can occur during workflow operations
#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("Workflow features are not enabled. Add 'workflow.enabled: true' to .tdt/config.yaml")]
    NotEnabled,

    #[error("Invalid status transition: {from} → {to}")]
    InvalidTransition { from: Status, to: Status },

    #[error("Authorization required: {entity_type} approval requires role {required_role}")]
    Unauthorized {
        entity_type: String,
        required_role: String,
    },

    #[error("Entity is not in {expected} status (current: {current})")]
    WrongStatus { expected: Status, current: Status },

    #[error("Team roster not configured. Run 'tdt team init' first")]
    NoTeamRoster,

    #[error("Current user not found in team roster")]
    UserNotInRoster,

    #[error("Failed to parse YAML: {message}")]
    YamlError { message: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Workflow engine for managing status transitions
pub struct WorkflowEngine {
    roster: Option<TeamRoster>,
    config: WorkflowConfig,
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub fn new(roster: Option<TeamRoster>, config: WorkflowConfig) -> Self {
        Self { roster, config }
    }

    /// Check if workflow features are enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the workflow configuration
    pub fn config(&self) -> &WorkflowConfig {
        &self.config
    }

    /// Get the team roster
    pub fn roster(&self) -> Option<&TeamRoster> {
        self.roster.as_ref()
    }

    /// Get the current user from the team roster
    pub fn current_user(&self) -> Option<&TeamMember> {
        self.roster.as_ref().and_then(|r| r.current_user())
    }

    /// Check if a status transition is valid
    pub fn is_valid_transition(&self, from: Status, to: Status) -> bool {
        matches!(
            (from, to),
            // Normal forward transitions
            (Status::Draft, Status::Review)
                | (Status::Review, Status::Approved)
                | (Status::Approved, Status::Released)
                // Rejection (back to draft)
                | (Status::Review, Status::Draft)
                // Obsolete from any released state
                | (Status::Released, Status::Obsolete)
                // Approved can be re-submitted for revision
                | (Status::Approved, Status::Review)
        )
    }

    /// Get allowed transitions from the current status
    pub fn allowed_transitions(&self, current: Status) -> Vec<Status> {
        match current {
            Status::Draft => vec![Status::Review],
            Status::Review => vec![Status::Approved, Status::Draft],
            Status::Approved => vec![Status::Released, Status::Review],
            Status::Released => vec![Status::Obsolete],
            Status::Obsolete => vec![],
        }
    }

    /// Check if a transition is allowed for the given user and entity type
    pub fn can_transition(
        &self,
        from: Status,
        to: Status,
        prefix: EntityPrefix,
        user: Option<&TeamMember>,
    ) -> Result<(), WorkflowError> {
        // Verify valid transition
        if !self.is_valid_transition(from, to) {
            return Err(WorkflowError::InvalidTransition { from, to });
        }

        // Authorization checks only apply when roster is configured
        let Some(roster) = &self.roster else {
            return Ok(()); // No roster = no auth checks
        };

        // For approval transitions, verify authorization
        if to == Status::Approved {
            let Some(member) = user else {
                return Err(WorkflowError::UserNotInRoster);
            };

            if !roster.can_approve(member, prefix) {
                let required_roles = roster
                    .required_roles(prefix)
                    .map(|roles| {
                        roles
                            .iter()
                            .map(|r| r.to_string())
                            .collect::<Vec<_>>()
                            .join(" or ")
                    })
                    .unwrap_or_else(|| "team member".to_string());

                return Err(WorkflowError::Unauthorized {
                    entity_type: prefix.as_str().to_string(),
                    required_role: required_roles,
                });
            }
        }

        // For release transitions, verify release authorization
        if to == Status::Released {
            let Some(member) = user else {
                return Err(WorkflowError::UserNotInRoster);
            };

            if !roster.can_release(member) {
                return Err(WorkflowError::Unauthorized {
                    entity_type: "release".to_string(),
                    required_role: "management".to_string(),
                });
            }
        }

        Ok(())
    }
}

/// Approval record stored in entity YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub approver: String,
    pub role: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub comment: Option<String>,
}

/// Rejection record stored in entity YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectionRecord {
    pub rejector: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

/// Update an entity's status in its YAML file
pub fn update_entity_status(file_path: &Path, new_status: Status) -> Result<(), WorkflowError> {
    let contents = std::fs::read_to_string(file_path)?;

    // Parse as YAML value to preserve formatting
    let mut doc: serde_yml::Value =
        serde_yml::from_str(&contents).map_err(|e| WorkflowError::YamlError {
            message: e.to_string(),
        })?;

    // Update status field
    if let Some(map) = doc.as_mapping_mut() {
        map.insert(
            serde_yml::Value::String("status".to_string()),
            serde_yml::Value::String(new_status.to_string()),
        );
    }

    // Serialize back
    let new_contents =
        serde_yml::to_string(&doc).map_err(|e| WorkflowError::YamlError {
            message: e.to_string(),
        })?;

    std::fs::write(file_path, new_contents)?;
    Ok(())
}

/// Record an approval in an entity's YAML file
pub fn record_approval(
    file_path: &Path,
    approver: &str,
    role: Option<Role>,
    comment: Option<&str>,
) -> Result<(), WorkflowError> {
    let contents = std::fs::read_to_string(file_path)?;

    let mut doc: serde_yml::Value =
        serde_yml::from_str(&contents).map_err(|e| WorkflowError::YamlError {
            message: e.to_string(),
        })?;

    if let Some(map) = doc.as_mapping_mut() {
        // Update status to approved
        map.insert(
            serde_yml::Value::String("status".to_string()),
            serde_yml::Value::String("approved".to_string()),
        );

        // Create approval record
        let mut approval = serde_yml::Mapping::new();
        approval.insert(
            serde_yml::Value::String("approver".to_string()),
            serde_yml::Value::String(approver.to_string()),
        );
        if let Some(r) = role {
            approval.insert(
                serde_yml::Value::String("role".to_string()),
                serde_yml::Value::String(r.to_string()),
            );
        }
        approval.insert(
            serde_yml::Value::String("timestamp".to_string()),
            serde_yml::Value::String(Utc::now().to_rfc3339()),
        );
        if let Some(c) = comment {
            approval.insert(
                serde_yml::Value::String("comment".to_string()),
                serde_yml::Value::String(c.to_string()),
            );
        }

        // Add to approvals list (create if doesn't exist)
        let approvals_key = serde_yml::Value::String("approvals".to_string());
        let approvals = map
            .entry(approvals_key)
            .or_insert_with(|| serde_yml::Value::Sequence(Vec::new()));

        if let Some(seq) = approvals.as_sequence_mut() {
            seq.push(serde_yml::Value::Mapping(approval));
        }
    }

    let new_contents =
        serde_yml::to_string(&doc).map_err(|e| WorkflowError::YamlError {
            message: e.to_string(),
        })?;

    std::fs::write(file_path, new_contents)?;
    Ok(())
}

/// Record a rejection in an entity's YAML file
pub fn record_rejection(
    file_path: &Path,
    rejector: &str,
    reason: &str,
) -> Result<(), WorkflowError> {
    let contents = std::fs::read_to_string(file_path)?;

    let mut doc: serde_yml::Value =
        serde_yml::from_str(&contents).map_err(|e| WorkflowError::YamlError {
            message: e.to_string(),
        })?;

    if let Some(map) = doc.as_mapping_mut() {
        // Update status back to draft
        map.insert(
            serde_yml::Value::String("status".to_string()),
            serde_yml::Value::String("draft".to_string()),
        );

        // Create rejection record
        let mut rejection = serde_yml::Mapping::new();
        rejection.insert(
            serde_yml::Value::String("rejector".to_string()),
            serde_yml::Value::String(rejector.to_string()),
        );
        rejection.insert(
            serde_yml::Value::String("reason".to_string()),
            serde_yml::Value::String(reason.to_string()),
        );
        rejection.insert(
            serde_yml::Value::String("timestamp".to_string()),
            serde_yml::Value::String(Utc::now().to_rfc3339()),
        );

        // Add to rejections list
        let rejections_key = serde_yml::Value::String("rejections".to_string());
        let rejections = map
            .entry(rejections_key)
            .or_insert_with(|| serde_yml::Value::Sequence(Vec::new()));

        if let Some(seq) = rejections.as_sequence_mut() {
            seq.push(serde_yml::Value::Mapping(rejection));
        }
    }

    let new_contents =
        serde_yml::to_string(&doc).map_err(|e| WorkflowError::YamlError {
            message: e.to_string(),
        })?;

    std::fs::write(file_path, new_contents)?;
    Ok(())
}

/// Record a release in an entity's YAML file
pub fn record_release(file_path: &Path, releaser: &str) -> Result<(), WorkflowError> {
    let contents = std::fs::read_to_string(file_path)?;

    let mut doc: serde_yml::Value =
        serde_yml::from_str(&contents).map_err(|e| WorkflowError::YamlError {
            message: e.to_string(),
        })?;

    if let Some(map) = doc.as_mapping_mut() {
        // Update status to released
        map.insert(
            serde_yml::Value::String("status".to_string()),
            serde_yml::Value::String("released".to_string()),
        );

        // Add release metadata
        map.insert(
            serde_yml::Value::String("released_by".to_string()),
            serde_yml::Value::String(releaser.to_string()),
        );
        map.insert(
            serde_yml::Value::String("released_at".to_string()),
            serde_yml::Value::String(Utc::now().to_rfc3339()),
        );
    }

    let new_contents =
        serde_yml::to_string(&doc).map_err(|e| WorkflowError::YamlError {
            message: e.to_string(),
        })?;

    std::fs::write(file_path, new_contents)?;
    Ok(())
}

/// Get entity info from a YAML file (id, title, status)
pub fn get_entity_info(file_path: &Path) -> Result<(String, String, Status), WorkflowError> {
    let contents = std::fs::read_to_string(file_path)?;

    let doc: serde_yml::Value =
        serde_yml::from_str(&contents).map_err(|e| WorkflowError::YamlError {
            message: e.to_string(),
        })?;

    let id = doc
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WorkflowError::YamlError {
            message: "Missing 'id' field".to_string(),
        })?
        .to_string();

    let title = doc
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();

    let status_str = doc
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("draft");

    let status = status_str.parse().unwrap_or(Status::Draft);

    Ok((id, title, status))
}

/// Extract truncated ID (8 chars after prefix) for git artifacts
pub fn truncate_id(full_id: &str) -> String {
    // Format: PREFIX-ULID (e.g., "REQ-01KCWY20F01B21V0G4E835NW3J")
    // Returns: PREFIX-01KCWY20 (first 8 chars of ULID)
    if let Some(dash_pos) = full_id.find('-') {
        let prefix = &full_id[..dash_pos];
        let ulid_part = &full_id[dash_pos + 1..];
        let short_ulid = if ulid_part.len() > 8 {
            &ulid_part[..8]
        } else {
            ulid_part
        };
        format!("{}-{}", prefix, short_ulid)
    } else {
        full_id.to_string()
    }
}

/// Get the entity prefix from a full ID
pub fn get_prefix_from_id(id: &str) -> Option<EntityPrefix> {
    let prefix_str = id.split('-').next()?;
    prefix_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_valid_transitions() {
        let engine = WorkflowEngine::new(None, WorkflowConfig::with_defaults());

        // Valid forward transitions
        assert!(engine.is_valid_transition(Status::Draft, Status::Review));
        assert!(engine.is_valid_transition(Status::Review, Status::Approved));
        assert!(engine.is_valid_transition(Status::Approved, Status::Released));
        assert!(engine.is_valid_transition(Status::Released, Status::Obsolete));

        // Valid backward transitions
        assert!(engine.is_valid_transition(Status::Review, Status::Draft));
        assert!(engine.is_valid_transition(Status::Approved, Status::Review));

        // Invalid transitions
        assert!(!engine.is_valid_transition(Status::Draft, Status::Approved));
        assert!(!engine.is_valid_transition(Status::Draft, Status::Released));
        assert!(!engine.is_valid_transition(Status::Released, Status::Draft));
    }

    #[test]
    fn test_allowed_transitions() {
        let engine = WorkflowEngine::new(None, WorkflowConfig::with_defaults());

        assert_eq!(engine.allowed_transitions(Status::Draft), vec![Status::Review]);
        assert_eq!(
            engine.allowed_transitions(Status::Review),
            vec![Status::Approved, Status::Draft]
        );
        assert_eq!(
            engine.allowed_transitions(Status::Approved),
            vec![Status::Released, Status::Review]
        );
        assert_eq!(
            engine.allowed_transitions(Status::Released),
            vec![Status::Obsolete]
        );
        assert!(engine.allowed_transitions(Status::Obsolete).is_empty());
    }

    #[test]
    fn test_truncate_id() {
        assert_eq!(
            truncate_id("REQ-01KCWY20F01B21V0G4E835NW3J"),
            "REQ-01KCWY20"
        );
        assert_eq!(
            truncate_id("RISK-01KCWY20F01B21V0G4E835NW3J"),
            "RISK-01KCWY20"
        );
        assert_eq!(truncate_id("REQ-SHORT"), "REQ-SHORT");
        assert_eq!(truncate_id("NOPREFIXID"), "NOPREFIXID");
    }

    #[test]
    fn test_format_branch() {
        let config = WorkflowConfig::with_defaults();
        assert_eq!(
            config.format_branch("REQ", "01KCWY20"),
            "review/REQ-01KCWY20"
        );
    }

    #[test]
    fn test_format_messages() {
        let config = WorkflowConfig::with_defaults();
        assert_eq!(
            config.format_submit_message("REQ-01KCWY20", "Test requirement"),
            "Submit REQ-01KCWY20: Test requirement"
        );
        assert_eq!(
            config.format_approve_message("REQ-01KCWY20", "Test requirement"),
            "Approve REQ-01KCWY20: Test requirement"
        );
    }

    #[test]
    fn test_update_entity_status() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("test.yaml");

        std::fs::write(
            &file,
            r#"id: REQ-TEST
title: Test Requirement
status: draft
"#,
        )
        .unwrap();

        update_entity_status(&file, Status::Review).unwrap();

        let contents = std::fs::read_to_string(&file).unwrap();
        assert!(contents.contains("status: review"));
    }

    #[test]
    fn test_get_entity_info() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("test.yaml");

        std::fs::write(
            &file,
            r#"id: REQ-TEST123
title: Test Requirement
status: review
"#,
        )
        .unwrap();

        let (id, title, status) = get_entity_info(&file).unwrap();
        assert_eq!(id, "REQ-TEST123");
        assert_eq!(title, "Test Requirement");
        assert_eq!(status, Status::Review);
    }

    #[test]
    fn test_record_approval() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("test.yaml");

        std::fs::write(
            &file,
            r#"id: REQ-TEST
title: Test Requirement
status: review
"#,
        )
        .unwrap();

        record_approval(&file, "jsmith", Some(Role::Engineering), Some("LGTM")).unwrap();

        let contents = std::fs::read_to_string(&file).unwrap();
        assert!(contents.contains("status: approved"));
        assert!(contents.contains("approver: jsmith"));
        assert!(contents.contains("role: engineering"));
        assert!(contents.contains("comment: LGTM"));
    }

    #[test]
    fn test_record_rejection() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("test.yaml");

        std::fs::write(
            &file,
            r#"id: REQ-TEST
title: Test Requirement
status: review
"#,
        )
        .unwrap();

        record_rejection(&file, "jsmith", "Needs more detail").unwrap();

        let contents = std::fs::read_to_string(&file).unwrap();
        assert!(contents.contains("status: draft"));
        assert!(contents.contains("rejector: jsmith"));
        assert!(contents.contains("reason: Needs more detail"));
    }

    #[test]
    fn test_get_prefix_from_id() {
        assert_eq!(get_prefix_from_id("REQ-01KCWY20"), Some(EntityPrefix::Req));
        assert_eq!(get_prefix_from_id("RISK-01KCWY20"), Some(EntityPrefix::Risk));
        assert_eq!(get_prefix_from_id("CMP-01KCWY20"), Some(EntityPrefix::Cmp));
        assert_eq!(get_prefix_from_id("INVALID-01KCWY20"), None);
    }
}
