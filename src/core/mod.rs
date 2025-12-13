//! Core module - fundamental types and utilities

pub mod config;
pub mod entity;
pub mod identity;
pub mod loader;
pub mod project;
pub mod shortid;

pub use config::Config;
pub use entity::Entity;
pub use identity::{EntityId, EntityPrefix, IdParseError};
pub use project::{Project, ProjectError};
pub use shortid::ShortIdIndex;
