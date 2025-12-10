//! Short ID system for easier entity selection
//!
//! Provides session-local numeric aliases like `@1`, `@2` that map to full entity IDs.
//! These are not persisted - they're regenerated each time entities are listed.

use std::collections::HashMap;
use std::fs;

use crate::core::identity::EntityId;
use crate::core::project::Project;

/// Index file location within a project
const INDEX_FILE: &str = ".pdt/shortids.json";

/// A mapping of short IDs (@N) to full entity IDs
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ShortIdIndex {
    /// Maps short number to full entity ID string
    entries: HashMap<u32, String>,
    /// Maps full entity ID to short number (reverse lookup)
    #[serde(skip)]
    reverse: HashMap<String, u32>,
    /// Next available short ID
    next_id: u32,
}

impl ShortIdIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            reverse: HashMap::new(),
            next_id: 1,
        }
    }

    /// Load the index from a project, or create empty if not found
    pub fn load(project: &Project) -> Self {
        let path = project.root().join(INDEX_FILE);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut index) = serde_json::from_str::<ShortIdIndex>(&content) {
                    // Rebuild reverse lookup
                    index.reverse = index.entries.iter()
                        .map(|(k, v)| (v.clone(), *k))
                        .collect();
                    return index;
                }
            }
        }
        Self::new()
    }

    /// Save the index to a project
    pub fn save(&self, project: &Project) -> std::io::Result<()> {
        let path = project.root().join(INDEX_FILE);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)
    }

    /// Clear and rebuild the index with new entity IDs
    pub fn rebuild(&mut self, entity_ids: impl IntoIterator<Item = String>) {
        self.entries.clear();
        self.reverse.clear();
        self.next_id = 1;

        for id in entity_ids {
            self.add(id);
        }
    }

    /// Add an entity ID and return its short ID
    pub fn add(&mut self, entity_id: String) -> u32 {
        if let Some(&short_id) = self.reverse.get(&entity_id) {
            return short_id;
        }

        let short_id = self.next_id;
        self.next_id += 1;
        self.entries.insert(short_id, entity_id.clone());
        self.reverse.insert(entity_id, short_id);
        short_id
    }

    /// Resolve a short ID reference to a full entity ID
    ///
    /// Accepts:
    /// - `@N` format (e.g., `@1`, `@42`)
    /// - Plain number (e.g., `1`, `42`)
    /// - Full or partial entity ID (passed through)
    pub fn resolve(&self, reference: &str) -> Option<String> {
        // Check if it's a short ID reference
        let num_str = if reference.starts_with('@') {
            &reference[1..]
        } else if reference.chars().all(|c| c.is_ascii_digit()) {
            reference
        } else {
            // Not a short ID, return as-is for partial matching
            return Some(reference.to_string());
        };

        // Parse the number and look up
        num_str.parse::<u32>().ok()
            .and_then(|n| self.entries.get(&n).cloned())
    }

    /// Get the short ID for a full entity ID
    pub fn get_short_id(&self, entity_id: &str) -> Option<u32> {
        self.reverse.get(entity_id).copied()
    }

    /// Format an entity ID with its short ID prefix
    pub fn format_with_short_id(&self, entity_id: &EntityId) -> String {
        let id_str = entity_id.to_string();
        if let Some(short_id) = self.reverse.get(&id_str) {
            format!("@{:<3} {}", short_id, id_str)
        } else {
            format!("     {}", id_str)
        }
    }

    /// Get all entries as (short_id, full_id) pairs
    pub fn iter(&self) -> impl Iterator<Item = (u32, &str)> {
        self.entries.iter().map(|(k, v)| (*k, v.as_str()))
    }

    /// Number of entries in the index
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Parse a reference that might be a short ID or a full/partial entity ID
pub fn parse_entity_reference(reference: &str, project: &Project) -> String {
    let index = ShortIdIndex::load(project);
    index.resolve(reference).unwrap_or_else(|| reference.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_id_add_and_resolve() {
        let mut index = ShortIdIndex::new();

        let short1 = index.add("REQ-01ABC".to_string());
        let short2 = index.add("REQ-02DEF".to_string());

        assert_eq!(short1, 1);
        assert_eq!(short2, 2);

        assert_eq!(index.resolve("@1"), Some("REQ-01ABC".to_string()));
        assert_eq!(index.resolve("@2"), Some("REQ-02DEF".to_string()));
        assert_eq!(index.resolve("1"), Some("REQ-01ABC".to_string()));
        assert_eq!(index.resolve("@99"), None);
    }

    #[test]
    fn test_short_id_passthrough() {
        let index = ShortIdIndex::new();

        // Non-numeric references should pass through
        assert_eq!(index.resolve("REQ-01ABC"), Some("REQ-01ABC".to_string()));
        assert_eq!(index.resolve("temperature"), Some("temperature".to_string()));
    }

    #[test]
    fn test_short_id_rebuild() {
        let mut index = ShortIdIndex::new();
        index.add("OLD-001".to_string());
        index.add("OLD-002".to_string());

        assert_eq!(index.len(), 2);

        index.rebuild(vec!["NEW-001".to_string(), "NEW-002".to_string(), "NEW-003".to_string()]);

        assert_eq!(index.len(), 3);
        assert_eq!(index.resolve("@1"), Some("NEW-001".to_string()));
        assert_eq!(index.resolve("@3"), Some("NEW-003".to_string()));
    }

    #[test]
    fn test_short_id_no_duplicates() {
        let mut index = ShortIdIndex::new();

        let short1 = index.add("REQ-001".to_string());
        let short2 = index.add("REQ-001".to_string()); // Same ID

        assert_eq!(short1, short2);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_get_short_id() {
        let mut index = ShortIdIndex::new();
        index.add("REQ-001".to_string());
        index.add("REQ-002".to_string());

        assert_eq!(index.get_short_id("REQ-001"), Some(1));
        assert_eq!(index.get_short_id("REQ-002"), Some(2));
        assert_eq!(index.get_short_id("REQ-003"), None);
    }
}
