//! Git workflow commands for collaborative review and approval
//!
//! These commands provide guided git workflows for non-git-savvy users:
//! - `submit` - Submit entities for review (creates PR)
//! - `approve` - Approve reviewed entities (approves PR)
//! - `reject` - Reject entities back to draft
//! - `release` - Release approved entities
//! - `review` - View pending reviews
//! - `team` - Team roster management

pub mod approve;
pub mod reject;
pub mod release;
pub mod review;
pub mod submit;
pub mod team;

pub use approve::ApproveArgs;
pub use reject::RejectArgs;
pub use release::ReleaseArgs;
pub use review::{ReviewCommands, ReviewListArgs};
pub use submit::SubmitArgs;
pub use team::TeamCommands;
