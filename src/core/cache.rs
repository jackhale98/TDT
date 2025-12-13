//! SQLite-backed entity cache for fast lookups
//!
//! This module provides a local SQLite cache that:
//! - Maps short IDs (PREFIX@N) to full entity IDs
//! - Caches entity metadata for fast lookups
//! - Auto-detects file changes and syncs incrementally
//! - Supports direct SQL queries for power users
//!
//! IMPORTANT: The cache is user-local and gitignored.
//! Entity files must NEVER contain short IDs - only full ULIDs.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, TimeZone, Utc};
use miette::{IntoDiagnostic, Result};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::core::identity::EntityPrefix;
use crate::core::project::Project;

/// Cache file location within a project
const CACHE_FILE: &str = ".tdt/cache.db";

/// Current schema version for migrations
const SCHEMA_VERSION: i32 = 6;

/// Link types for entity relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    TracesTo,      // Requirement traces to another requirement
    TracesFrom,    // Reverse of traces_to
    Verifies,      // Test verifies a requirement
    VerifiedBy,    // Reverse of verifies
    Mitigates,     // Control/CAPA mitigates a risk
    MitigatedBy,   // Reverse of mitigates
    References,    // Generic reference to another entity
    ReferencedBy,  // Reverse of references
    Contains,      // Assembly contains component
    ContainedIn,   // Reverse of contains
    QuotesFor,     // Quote is for a component
    QuotedBy,      // Reverse of quotes_for
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::TracesTo => "traces_to",
            LinkType::TracesFrom => "traces_from",
            LinkType::Verifies => "verifies",
            LinkType::VerifiedBy => "verified_by",
            LinkType::Mitigates => "mitigates",
            LinkType::MitigatedBy => "mitigated_by",
            LinkType::References => "references",
            LinkType::ReferencedBy => "referenced_by",
            LinkType::Contains => "contains",
            LinkType::ContainedIn => "contained_in",
            LinkType::QuotesFor => "quotes_for",
            LinkType::QuotedBy => "quoted_by",
        }
    }

    pub fn reverse(&self) -> Self {
        match self {
            LinkType::TracesTo => LinkType::TracesFrom,
            LinkType::TracesFrom => LinkType::TracesTo,
            LinkType::Verifies => LinkType::VerifiedBy,
            LinkType::VerifiedBy => LinkType::Verifies,
            LinkType::Mitigates => LinkType::MitigatedBy,
            LinkType::MitigatedBy => LinkType::Mitigates,
            LinkType::References => LinkType::ReferencedBy,
            LinkType::ReferencedBy => LinkType::References,
            LinkType::Contains => LinkType::ContainedIn,
            LinkType::ContainedIn => LinkType::Contains,
            LinkType::QuotesFor => LinkType::QuotedBy,
            LinkType::QuotedBy => LinkType::QuotesFor,
        }
    }
}

/// A cached link between two entities
#[derive(Debug, Clone)]
pub struct CachedLink {
    pub source_id: String,
    pub target_id: String,
    pub link_type: String,
}

/// The entity cache backed by SQLite
pub struct EntityCache {
    conn: Connection,
    project_root: PathBuf,
}

/// Cached entity metadata (fast access without YAML parsing)
#[derive(Debug, Clone)]
pub struct CachedEntity {
    pub id: String,
    pub prefix: String,
    pub title: String,
    pub status: String,
    pub author: String,
    pub created: DateTime<Utc>,
    pub file_path: PathBuf,
    // Extended fields (v3 schema)
    pub priority: Option<String>,
    pub entity_type: Option<String>,   // subtype (input/output, verification/validation, etc.)
    pub category: Option<String>,
    pub tags: Vec<String>,
}

/// Cached feature with dimension data
#[derive(Debug, Clone)]
pub struct CachedFeature {
    pub id: String,
    pub title: String,
    pub status: String,
    pub component_id: String,
    pub feature_type: String,
    pub dim_name: Option<String>,
    pub dim_nominal: Option<f64>,
    pub dim_plus_tol: Option<f64>,
    pub dim_minus_tol: Option<f64>,
    pub dim_internal: Option<bool>,
    pub author: String,
    pub created: DateTime<Utc>,
    pub file_path: PathBuf,
}

/// Cached supplier data
#[derive(Debug, Clone)]
pub struct CachedSupplier {
    pub id: String,
    pub name: String,
    pub short_name: Option<String>,
    pub status: String,
    pub author: String,
    pub created: DateTime<Utc>,
    pub website: Option<String>,
    pub capabilities: Vec<String>,
    pub lead_time_days: Option<i32>,
    pub file_path: PathBuf,
}

/// Cached requirement data
#[derive(Debug, Clone)]
pub struct CachedRequirement {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub req_type: Option<String>,
    pub category: Option<String>,
    pub author: String,
    pub created: DateTime<Utc>,
    pub tags: Vec<String>,
    pub file_path: PathBuf,
}

/// Cached risk data
#[derive(Debug, Clone)]
pub struct CachedRisk {
    pub id: String,
    pub title: String,
    pub status: String,
    pub risk_type: Option<String>,
    pub severity: Option<i32>,
    pub occurrence: Option<i32>,
    pub detection: Option<i32>,
    pub rpn: Option<i32>,
    pub risk_level: Option<String>,
    pub category: Option<String>,
    pub author: String,
    pub created: DateTime<Utc>,
    pub file_path: PathBuf,
}

/// Cached test protocol data
#[derive(Debug, Clone)]
pub struct CachedTest {
    pub id: String,
    pub title: String,
    pub status: String,
    pub test_type: Option<String>,
    pub level: Option<String>,
    pub method: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub author: String,
    pub created: DateTime<Utc>,
    pub file_path: PathBuf,
}

/// Cached component data
#[derive(Debug, Clone)]
pub struct CachedComponent {
    pub id: String,
    pub title: String,
    pub status: String,
    pub part_number: Option<String>,
    pub revision: Option<String>,
    pub make_buy: Option<String>,
    pub category: Option<String>,
    pub author: String,
    pub created: DateTime<Utc>,
    pub file_path: PathBuf,
}

/// Cached quote data
#[derive(Debug, Clone)]
pub struct CachedQuote {
    pub id: String,
    pub title: String,
    pub status: String,
    pub quote_status: Option<String>,
    pub supplier_id: Option<String>,
    pub component_id: Option<String>,
    pub unit_price: Option<f64>,
    pub quantity: Option<i32>,
    pub lead_time_days: Option<i32>,
    pub currency: Option<String>,
    pub valid_until: Option<String>,
    pub author: String,
    pub created: DateTime<Utc>,
    pub file_path: PathBuf,
}

/// Cached NCR data
#[derive(Debug, Clone)]
pub struct CachedNcr {
    pub id: String,
    pub title: String,
    pub status: String,
    pub ncr_type: Option<String>,
    pub severity: Option<String>,
    pub ncr_status: Option<String>,
    pub category: Option<String>,
    pub author: String,
    pub created: DateTime<Utc>,
    pub file_path: PathBuf,
}

/// Cached CAPA data
#[derive(Debug, Clone)]
pub struct CachedCapa {
    pub id: String,
    pub title: String,
    pub status: String,
    pub capa_type: Option<String>,
    pub capa_status: Option<String>,
    pub author: String,
    pub created: DateTime<Utc>,
    pub file_path: PathBuf,
}

/// Statistics from sync operation
#[derive(Debug, Default)]
pub struct SyncStats {
    pub files_scanned: usize,
    pub entities_added: usize,
    pub entities_updated: usize,
    pub entities_removed: usize,
    pub duration_ms: u64,
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub total_entities: usize,
    pub total_short_ids: usize,
    pub by_prefix: HashMap<String, usize>,
    pub db_size_bytes: u64,
}

/// Filter for listing entities
#[derive(Debug, Default)]
pub struct EntityFilter {
    pub prefix: Option<EntityPrefix>,
    pub status: Option<String>,
    pub author: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    // Extended filters (v3 schema)
    pub priority: Option<String>,
    pub entity_type: Option<String>,   // subtype filter (input/output, verification/validation)
    pub category: Option<String>,
}

impl EntityCache {
    /// Open or create cache for a project
    ///
    /// If the cache doesn't exist, it will be created and populated.
    /// If the cache is stale (files changed), it will be synced automatically.
    pub fn open(project: &Project) -> Result<Self> {
        let cache_path = project.root().join(CACHE_FILE);

        // Ensure .tdt directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).into_diagnostic()?;
        }

        let needs_init = !cache_path.exists();
        let conn = Connection::open(&cache_path).into_diagnostic()?;

        // Enable WAL mode for better concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL;").into_diagnostic()?;

        let mut cache = Self {
            conn,
            project_root: project.root().to_path_buf(),
        };

        if needs_init {
            cache.init_schema()?;
            cache.rebuild()?;
        } else {
            // Check schema version and migrate if needed
            cache.migrate_schema_if_needed()?;
            // Auto-sync to detect file changes
            cache.auto_sync()?;
        }

        Ok(cache)
    }

    /// Check if schema needs migration and perform it
    fn migrate_schema_if_needed(&mut self) -> Result<()> {
        let current_version: i32 = self
            .conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .unwrap_or(1);

        if current_version < SCHEMA_VERSION {
            // Migrate from v1 to v2: add links table
            if current_version < 2 {
                self.conn
                    .execute_batch(
                        r#"
                    CREATE TABLE IF NOT EXISTS links (
                        source_id TEXT NOT NULL,
                        target_id TEXT NOT NULL,
                        link_type TEXT NOT NULL,
                        PRIMARY KEY (source_id, target_id, link_type)
                    );
                    CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_id);
                    CREATE INDEX IF NOT EXISTS idx_links_target ON links(target_id);
                    CREATE INDEX IF NOT EXISTS idx_links_type ON links(link_type);
                    "#,
                    )
                    .into_diagnostic()?;
            }

            // Migrate from v2 to v3: add extended entity fields and type-specific tables
            if current_version < 3 {
                self.conn
                    .execute_batch(
                        r#"
                    -- Add new columns to entities table
                    ALTER TABLE entities ADD COLUMN priority TEXT;
                    ALTER TABLE entities ADD COLUMN entity_type TEXT;
                    ALTER TABLE entities ADD COLUMN category TEXT;
                    ALTER TABLE entities ADD COLUMN tags TEXT;

                    CREATE INDEX IF NOT EXISTS idx_entities_priority ON entities(priority);
                    CREATE INDEX IF NOT EXISTS idx_entities_entity_type ON entities(entity_type);
                    CREATE INDEX IF NOT EXISTS idx_entities_category ON entities(category);

                    -- Add new type-specific tables
                    CREATE TABLE IF NOT EXISTS tests (
                        id TEXT PRIMARY KEY,
                        test_type TEXT,
                        level TEXT,
                        method TEXT,
                        FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
                    );

                    CREATE TABLE IF NOT EXISTS quotes (
                        id TEXT PRIMARY KEY,
                        supplier_id TEXT,
                        component_id TEXT,
                        unit_price REAL,
                        quantity INTEGER,
                        lead_time_days INTEGER,
                        currency TEXT,
                        valid_until TEXT,
                        FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
                    );
                    CREATE INDEX IF NOT EXISTS idx_quotes_supplier ON quotes(supplier_id);
                    CREATE INDEX IF NOT EXISTS idx_quotes_component ON quotes(component_id);

                    CREATE TABLE IF NOT EXISTS suppliers (
                        id TEXT PRIMARY KEY,
                        short_name TEXT,
                        contact_name TEXT,
                        email TEXT,
                        phone TEXT,
                        location TEXT,
                        website TEXT,
                        lead_time_days INTEGER,
                        capabilities TEXT,
                        FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
                    );

                    CREATE TABLE IF NOT EXISTS processes (
                        id TEXT PRIMARY KEY,
                        process_type TEXT,
                        equipment TEXT,
                        FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
                    );

                    CREATE TABLE IF NOT EXISTS controls (
                        id TEXT PRIMARY KEY,
                        control_type TEXT,
                        inspection_method TEXT,
                        frequency TEXT,
                        process_id TEXT,
                        FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
                    );
                    CREATE INDEX IF NOT EXISTS idx_controls_process ON controls(process_id);

                    CREATE TABLE IF NOT EXISTS works (
                        id TEXT PRIMARY KEY,
                        process_id TEXT,
                        FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
                    );
                    CREATE INDEX IF NOT EXISTS idx_works_process ON works(process_id);

                    CREATE TABLE IF NOT EXISTS ncrs (
                        id TEXT PRIMARY KEY,
                        ncr_type TEXT,
                        severity TEXT,
                        ncr_status TEXT,
                        category TEXT,
                        disposition TEXT,
                        component_id TEXT,
                        process_id TEXT,
                        FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
                    );
                    CREATE INDEX IF NOT EXISTS idx_ncrs_ncr_status ON ncrs(ncr_status);
                    CREATE INDEX IF NOT EXISTS idx_ncrs_severity ON ncrs(severity);

                    CREATE TABLE IF NOT EXISTS capas (
                        id TEXT PRIMARY KEY,
                        capa_type TEXT,
                        capa_status TEXT,
                        effectiveness TEXT,
                        FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
                    );
                    CREATE INDEX IF NOT EXISTS idx_capas_capa_status ON capas(capa_status);

                    CREATE TABLE IF NOT EXISTS assemblies (
                        id TEXT PRIMARY KEY,
                        part_number TEXT,
                        revision TEXT,
                        FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
                    );
                    "#,
                    )
                    .into_diagnostic()?;
            }

            // Migrate from v3 to v4: add quote_status to quotes table
            if current_version < 4 {
                self.conn
                    .execute_batch(
                        r#"
                    ALTER TABLE quotes ADD COLUMN quote_status TEXT;
                    CREATE INDEX IF NOT EXISTS idx_quotes_status ON quotes(quote_status);
                    "#,
                    )
                    .into_diagnostic()?;
            }

            // Migrate from v4 to v5: add ncr_status and category to ncrs table
            if current_version < 5 {
                self.conn
                    .execute_batch(
                        r#"
                    ALTER TABLE ncrs ADD COLUMN ncr_status TEXT;
                    ALTER TABLE ncrs ADD COLUMN category TEXT;
                    CREATE INDEX IF NOT EXISTS idx_ncrs_ncr_status ON ncrs(ncr_status);
                    CREATE INDEX IF NOT EXISTS idx_ncrs_severity ON ncrs(severity);
                    "#,
                    )
                    .into_diagnostic()?;
            }

            // Migrate from v5 to v6: add capa_status to capas table
            if current_version < 6 {
                self.conn
                    .execute_batch(
                        r#"
                    ALTER TABLE capas ADD COLUMN capa_status TEXT;
                    CREATE INDEX IF NOT EXISTS idx_capas_capa_status ON capas(capa_status);
                    "#,
                    )
                    .into_diagnostic()?;
            }

            // Update version
            self.conn
                .execute(
                    "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
                    params![SCHEMA_VERSION],
                )
                .into_diagnostic()?;

            // Rebuild to populate new tables
            self.rebuild()?;
        }

        Ok(())
    }

    /// Auto-sync: quickly check if any files changed and sync if needed
    fn auto_sync(&mut self) -> Result<()> {
        // Get the most recent file mtime from cache
        let cached_max_mtime: Option<i64> = self
            .conn
            .query_row("SELECT MAX(file_mtime) FROM entities", [], |row| row.get(0))
            .optional()
            .into_diagnostic()?
            .flatten();

        // Quick check: scan for any file newer than cached max mtime
        let needs_sync = self.has_newer_files(cached_max_mtime.unwrap_or(0))?;

        if needs_sync {
            self.sync()?;
        }

        Ok(())
    }

    /// Check if any entity files are newer than the given mtime
    fn has_newer_files(&self, max_cached_mtime: i64) -> Result<bool> {
        let entity_dirs = Self::entity_directories();

        for dir in entity_dirs {
            let full_path = self.project_root.join(dir);
            if !full_path.exists() {
                continue;
            }

            for entry in WalkDir::new(&full_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if !path.to_string_lossy().ends_with(".tdt.yaml") {
                    continue;
                }

                let mtime = get_file_mtime(path)?;
                if mtime > max_cached_mtime {
                    return Ok(true);
                }
            }
        }

        // Also check if any cached files were deleted
        let cached_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
            .into_diagnostic()?;

        let mut actual_count = 0i64;
        for dir in entity_dirs {
            let full_path = self.project_root.join(dir);
            if full_path.exists() {
                for entry in WalkDir::new(&full_path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    if entry.path().to_string_lossy().ends_with(".tdt.yaml") {
                        actual_count += 1;
                    }
                }
            }
        }

        if actual_count != cached_count {
            return Ok(true);
        }

        Ok(false)
    }

    /// Get the list of entity directories to scan
    fn entity_directories() -> &'static [&'static str] {
        &[
            "requirements/inputs",
            "requirements/outputs",
            "risks/design",
            "risks/process",
            "bom/assemblies",
            "bom/components",
            "bom/quotes",
            "bom/suppliers",
            "tolerances/features",
            "tolerances/mates",
            "tolerances/stackups",
            "verification/protocols",
            "verification/results",
            "validation/protocols",
            "validation/results",
            "manufacturing/processes",
            "manufacturing/controls",
            "manufacturing/work_instructions",
            "manufacturing/ncrs",
            "manufacturing/capas",
        ]
    }

    /// Open cache without auto-sync (for testing)
    pub fn open_without_sync(project: &Project) -> Result<Self> {
        let cache_path = project.root().join(CACHE_FILE);

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).into_diagnostic()?;
        }

        let needs_init = !cache_path.exists();
        let conn = Connection::open(&cache_path).into_diagnostic()?;
        conn.execute_batch("PRAGMA journal_mode=WAL;").into_diagnostic()?;

        let mut cache = Self {
            conn,
            project_root: project.root().to_path_buf(),
        };

        if needs_init {
            cache.init_schema()?;
        }

        Ok(cache)
    }

    /// Initialize the database schema
    fn init_schema(&mut self) -> Result<()> {
        self.conn
            .execute_batch(
                r#"
            -- Schema version for migrations
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY
            );

            -- Short ID mappings
            CREATE TABLE IF NOT EXISTS short_ids (
                short_id TEXT PRIMARY KEY,
                entity_id TEXT NOT NULL UNIQUE,
                prefix TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_short_ids_entity ON short_ids(entity_id);
            CREATE INDEX IF NOT EXISTS idx_short_ids_prefix ON short_ids(prefix);

            -- Next available short ID per prefix
            CREATE TABLE IF NOT EXISTS short_id_counters (
                prefix TEXT PRIMARY KEY,
                next_id INTEGER NOT NULL DEFAULT 1
            );

            -- Entity metadata (common fields for all entity types)
            CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                prefix TEXT NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                author TEXT NOT NULL,
                created TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_mtime INTEGER NOT NULL,
                file_hash TEXT NOT NULL,
                -- Common fields added in v3
                priority TEXT,
                entity_type TEXT,           -- subtype (input/output, verification/validation, etc.)
                category TEXT,
                tags TEXT                   -- comma-separated
            );
            CREATE INDEX IF NOT EXISTS idx_entities_prefix ON entities(prefix);
            CREATE INDEX IF NOT EXISTS idx_entities_status ON entities(status);
            CREATE INDEX IF NOT EXISTS idx_entities_priority ON entities(priority);
            CREATE INDEX IF NOT EXISTS idx_entities_entity_type ON entities(entity_type);
            CREATE INDEX IF NOT EXISTS idx_entities_category ON entities(category);
            CREATE INDEX IF NOT EXISTS idx_entities_file_path ON entities(file_path);

            -- Feature-specific data
            CREATE TABLE IF NOT EXISTS features (
                id TEXT PRIMARY KEY,
                component_id TEXT NOT NULL,
                feature_type TEXT NOT NULL,
                dim_name TEXT,
                dim_nominal REAL,
                dim_plus_tol REAL,
                dim_minus_tol REAL,
                dim_internal INTEGER,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_features_component ON features(component_id);

            -- Component-specific data
            CREATE TABLE IF NOT EXISTS components (
                id TEXT PRIMARY KEY,
                part_number TEXT,
                revision TEXT,
                make_buy TEXT,
                category TEXT,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );

            -- Risk-specific data
            CREATE TABLE IF NOT EXISTS risks (
                id TEXT PRIMARY KEY,
                risk_type TEXT,
                severity INTEGER,
                occurrence INTEGER,
                detection INTEGER,
                rpn INTEGER,
                risk_level TEXT,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );

            -- Test-specific data
            CREATE TABLE IF NOT EXISTS tests (
                id TEXT PRIMARY KEY,
                test_type TEXT,             -- verification/validation
                level TEXT,                 -- unit/integration/system/acceptance
                method TEXT,                -- inspection/analysis/demonstration/test
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );

            -- Quote-specific data
            CREATE TABLE IF NOT EXISTS quotes (
                id TEXT PRIMARY KEY,
                quote_status TEXT,
                supplier_id TEXT,
                component_id TEXT,
                unit_price REAL,
                quantity INTEGER,
                lead_time_days INTEGER,
                currency TEXT,
                valid_until TEXT,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_quotes_supplier ON quotes(supplier_id);
            CREATE INDEX IF NOT EXISTS idx_quotes_component ON quotes(component_id);
            CREATE INDEX IF NOT EXISTS idx_quotes_status ON quotes(quote_status);

            -- Supplier-specific data
            CREATE TABLE IF NOT EXISTS suppliers (
                id TEXT PRIMARY KEY,
                short_name TEXT,
                contact_name TEXT,
                email TEXT,
                phone TEXT,
                location TEXT,
                website TEXT,
                lead_time_days INTEGER,
                capabilities TEXT,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );

            -- Process-specific data
            CREATE TABLE IF NOT EXISTS processes (
                id TEXT PRIMARY KEY,
                process_type TEXT,
                equipment TEXT,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );

            -- Control-specific data
            CREATE TABLE IF NOT EXISTS controls (
                id TEXT PRIMARY KEY,
                control_type TEXT,
                inspection_method TEXT,
                frequency TEXT,
                process_id TEXT,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_controls_process ON controls(process_id);

            -- Work instruction-specific data
            CREATE TABLE IF NOT EXISTS works (
                id TEXT PRIMARY KEY,
                process_id TEXT,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_works_process ON works(process_id);

            -- NCR-specific data
            CREATE TABLE IF NOT EXISTS ncrs (
                id TEXT PRIMARY KEY,
                ncr_type TEXT,
                severity TEXT,
                ncr_status TEXT,
                category TEXT,
                disposition TEXT,
                component_id TEXT,
                process_id TEXT,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_ncrs_ncr_status ON ncrs(ncr_status);
            CREATE INDEX IF NOT EXISTS idx_ncrs_severity ON ncrs(severity);

            -- CAPA-specific data
            CREATE TABLE IF NOT EXISTS capas (
                id TEXT PRIMARY KEY,
                capa_type TEXT,             -- corrective/preventive
                capa_status TEXT,
                effectiveness TEXT,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_capas_capa_status ON capas(capa_status);

            -- Assembly-specific data
            CREATE TABLE IF NOT EXISTS assemblies (
                id TEXT PRIMARY KEY,
                part_number TEXT,
                revision TEXT,
                FOREIGN KEY (id) REFERENCES entities(id) ON DELETE CASCADE
            );

            -- Entity links/relationships
            CREATE TABLE IF NOT EXISTS links (
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                link_type TEXT NOT NULL,
                PRIMARY KEY (source_id, target_id, link_type)
            );
            CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_id);
            CREATE INDEX IF NOT EXISTS idx_links_target ON links(target_id);
            CREATE INDEX IF NOT EXISTS idx_links_type ON links(link_type);

            -- Cache metadata
            CREATE TABLE IF NOT EXISTS cache_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
            )
            .into_diagnostic()?;

        // Set schema version
        self.conn
            .execute(
                "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
                params![SCHEMA_VERSION],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Full rebuild of cache from filesystem
    pub fn rebuild(&mut self) -> Result<SyncStats> {
        let start = std::time::Instant::now();
        let mut stats = SyncStats::default();

        // Clear existing entity data (but preserve short ID mappings)
        self.conn
            .execute_batch(
                r#"
            DELETE FROM entities;
            DELETE FROM features;
            DELETE FROM components;
            DELETE FROM risks;
            DELETE FROM tests;
            DELETE FROM quotes;
            DELETE FROM suppliers;
            DELETE FROM processes;
            DELETE FROM controls;
            DELETE FROM works;
            DELETE FROM ncrs;
            DELETE FROM capas;
            DELETE FROM assemblies;
            DELETE FROM links;
            "#,
            )
            .into_diagnostic()?;

        // Scan all entity directories
        for dir in Self::entity_directories() {
            let full_path = self.project_root.join(dir);
            if full_path.exists() {
                self.scan_directory(&full_path, &mut stats)?;
            }
        }

        stats.duration_ms = start.elapsed().as_millis() as u64;
        Ok(stats)
    }

    /// Scan a directory and cache all entities
    fn scan_directory(&mut self, dir: &Path, stats: &mut SyncStats) -> Result<()> {
        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if !path.to_string_lossy().ends_with(".tdt.yaml") {
                continue;
            }

            stats.files_scanned += 1;

            if let Err(e) = self.cache_entity_file(path) {
                // Log but continue on parse errors
                eprintln!("Warning: Failed to cache {}: {}", path.display(), e);
            } else {
                stats.entities_added += 1;
            }
        }

        Ok(())
    }

    /// Cache a single entity file
    fn cache_entity_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path).into_diagnostic()?;
        let mtime = get_file_mtime(path)?;
        let hash = compute_hash(&content);
        let rel_path = path
            .strip_prefix(&self.project_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Parse fields from YAML
        let value: serde_yml::Value = serde_yml::from_str(&content).into_diagnostic()?;

        let id = value["id"]
            .as_str()
            .ok_or_else(|| miette::miette!("Missing 'id' field"))?;
        // Use title if available, fall back to name (for suppliers)
        let title = value["title"]
            .as_str()
            .or_else(|| value["name"].as_str())
            .unwrap_or("");
        let status = value["status"].as_str().unwrap_or("draft");
        let author = value["author"].as_str().unwrap_or("");
        let created = value["created"].as_str().unwrap_or("");

        // Extract common fields (v3 schema)
        let priority = value["priority"].as_str();
        let entity_type = value["type"].as_str(); // "type" field holds subtype (input/output, verification/validation)
        let category = value["category"].as_str();
        let tags: Option<String> = value["tags"]
            .as_sequence()
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            });

        // Extract prefix from ID
        let prefix = id
            .split('-')
            .next()
            .ok_or_else(|| miette::miette!("Invalid ID format"))?;

        // Insert into entities table with all fields
        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO entities
                   (id, prefix, title, status, author, created, file_path, file_mtime, file_hash,
                    priority, entity_type, category, tags)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"#,
                params![id, prefix, title, status, author, created, rel_path, mtime, hash,
                        priority, entity_type, category, tags],
            )
            .into_diagnostic()?;

        // Ensure short ID exists
        self.ensure_short_id(id)?;

        // Cache type-specific data
        match prefix {
            "FEAT" => self.cache_feature_data(id, &value)?,
            "CMP" => self.cache_component_data(id, &value)?,
            "RISK" => self.cache_risk_data(id, &value)?,
            "TEST" => self.cache_test_data(id, &value)?,
            "QUOT" => self.cache_quote_data(id, &value)?,
            "SUP" => self.cache_supplier_data(id, &value)?,
            "PROC" => self.cache_process_data(id, &value)?,
            "CTRL" => self.cache_control_data(id, &value)?,
            "WORK" => self.cache_work_data(id, &value)?,
            "NCR" => self.cache_ncr_data(id, &value)?,
            "CAPA" => self.cache_capa_data(id, &value)?,
            "ASM" => self.cache_assembly_data(id, &value)?,
            _ => {}
        }

        // Cache links/relationships
        self.cache_entity_links(id, &value)?;

        Ok(())
    }

    /// Extract and cache links from an entity
    fn cache_entity_links(&self, source_id: &str, value: &serde_yml::Value) -> Result<()> {
        // Clear existing links for this entity
        self.conn
            .execute("DELETE FROM links WHERE source_id = ?1", params![source_id])
            .into_diagnostic()?;

        // Extract links from various fields
        let link_fields = [
            ("traces_to", "traces_to"),
            ("traces_from", "traces_from"),
            ("verifies", "verifies"),
            ("verified_by", "verified_by"),
            ("mitigates", "mitigates"),
            ("mitigated_by", "mitigated_by"),
            ("references", "references"),
            ("components", "contains"),        // Assembly contains components
            ("children", "contains"),          // Assembly children
            ("requirements", "verifies"),      // Test verifies requirements
            ("risks", "mitigates"),            // Control/CAPA mitigates risks
            ("parent", "contained_in"),        // Component contained in assembly
            ("component", "quotes_for"),       // Quote quotes for component (special case)
        ];

        for (field, link_type) in link_fields {
            if let Some(targets) = value[field].as_sequence() {
                for target in targets {
                    if let Some(target_id) = target.as_str() {
                        self.insert_link(source_id, target_id, link_type)?;
                    } else if let Some(target_obj) = target.as_mapping() {
                        // Handle nested objects like { id: "XXX", ... }
                        if let Some(target_id) = target_obj
                            .get(serde_yml::Value::String("id".to_string()))
                            .and_then(|v| v.as_str())
                        {
                            self.insert_link(source_id, target_id, link_type)?;
                        }
                    }
                }
            } else if let Some(target_id) = value[field].as_str() {
                // Handle single string value (e.g., "component" field on Feature)
                if field == "component" {
                    // Feature references component
                    self.insert_link(source_id, target_id, "references")?;
                } else if field == "parent" {
                    self.insert_link(source_id, target_id, "contained_in")?;
                } else {
                    self.insert_link(source_id, target_id, link_type)?;
                }
            }
        }

        Ok(())
    }

    /// Insert a link into the cache
    fn insert_link(&self, source_id: &str, target_id: &str, link_type: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO links (source_id, target_id, link_type) VALUES (?1, ?2, ?3)",
                params![source_id, target_id, link_type],
            )
            .into_diagnostic()?;
        Ok(())
    }

    /// Cache feature-specific data
    fn cache_feature_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let component_id = value["component"].as_str().unwrap_or("");
        let feature_type = value["feature_type"].as_str().unwrap_or("internal");

        // Get primary dimension if available
        let dims = value["dimensions"].as_sequence();
        let (dim_name, dim_nominal, dim_plus_tol, dim_minus_tol, dim_internal) =
            if let Some(dims) = dims {
                if let Some(first) = dims.first() {
                    (
                        first["name"].as_str().map(String::from),
                        first["nominal"].as_f64(),
                        first["plus_tol"].as_f64(),
                        first["minus_tol"].as_f64(),
                        first["internal"].as_bool(),
                    )
                } else {
                    (None, None, None, None, None)
                }
            } else {
                (None, None, None, None, None)
            };

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO features
                   (id, component_id, feature_type, dim_name, dim_nominal, dim_plus_tol, dim_minus_tol, dim_internal)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
                params![
                    id,
                    component_id,
                    feature_type,
                    dim_name,
                    dim_nominal,
                    dim_plus_tol,
                    dim_minus_tol,
                    dim_internal.map(|b| if b { 1 } else { 0 })
                ],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache component-specific data
    fn cache_component_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let part_number = value["part_number"].as_str();
        let revision = value["revision"].as_str();
        let make_buy = value["make_buy"].as_str();
        let category = value["category"].as_str();

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO components
                   (id, part_number, revision, make_buy, category)
                   VALUES (?1, ?2, ?3, ?4, ?5)"#,
                params![id, part_number, revision, make_buy, category],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache risk-specific data
    fn cache_risk_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let risk_type = value["risk_type"].as_str();
        let severity = value["severity"].as_i64().map(|v| v as i32);
        let occurrence = value["occurrence"].as_i64().map(|v| v as i32);
        let detection = value["detection"].as_i64().map(|v| v as i32);
        let rpn = value["rpn"].as_i64().map(|v| v as i32);
        let risk_level = value["risk_level"].as_str();

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO risks
                   (id, risk_type, severity, occurrence, detection, rpn, risk_level)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
                params![id, risk_type, severity, occurrence, detection, rpn, risk_level],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache test-specific data
    fn cache_test_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let test_type = value["type"].as_str();
        let level = value["level"].as_str();
        let method = value["method"].as_str();

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO tests (id, test_type, level, method)
                   VALUES (?1, ?2, ?3, ?4)"#,
                params![id, test_type, level, method],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache quote-specific data
    fn cache_quote_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let quote_status = value["quote_status"].as_str();
        let supplier_id = value["supplier"].as_str();
        let component_id = value["component"].as_str();
        let unit_price = value["unit_price"].as_f64();
        let quantity = value["quantity"].as_i64().map(|v| v as i32);
        let lead_time_days = value["lead_time_days"].as_i64().map(|v| v as i32);
        let currency = value["currency"].as_str();
        let valid_until = value["valid_until"].as_str();

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO quotes
                   (id, quote_status, supplier_id, component_id, unit_price, quantity, lead_time_days, currency, valid_until)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
                params![id, quote_status, supplier_id, component_id, unit_price, quantity, lead_time_days, currency, valid_until],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache supplier-specific data
    fn cache_supplier_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let short_name = value["short_name"].as_str();
        // Extract contact info from nested structure (first contact if contacts array)
        let contact = if let Some(contacts) = value["contacts"].as_sequence() {
            contacts.first().map(|c| c.clone()).unwrap_or(serde_yml::Value::Null)
        } else {
            value["contact"].clone()
        };
        let contact_name = contact["name"].as_str();
        let email = contact["email"].as_str();
        let phone = contact["phone"].as_str();
        let location = value["location"].as_str();
        let website = value["website"].as_str();
        let lead_time_days = value["lead_time_days"].as_i64().map(|v| v as i32);

        // Extract capabilities as comma-separated string
        let capabilities: Option<String> = value["capabilities"]
            .as_sequence()
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            });

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO suppliers
                   (id, short_name, contact_name, email, phone, location, website, lead_time_days, capabilities)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
                params![id, short_name, contact_name, email, phone, location, website, lead_time_days, capabilities],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache process-specific data
    fn cache_process_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let process_type = value["process_type"].as_str();
        let equipment = value["equipment"]
            .as_sequence()
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            });

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO processes (id, process_type, equipment)
                   VALUES (?1, ?2, ?3)"#,
                params![id, process_type, equipment],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache control-specific data
    fn cache_control_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let control_type = value["control_type"].as_str();
        let inspection_method = value["inspection_method"].as_str();
        let frequency = value["frequency"].as_str();
        let process_id = value["process"].as_str();

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO controls
                   (id, control_type, inspection_method, frequency, process_id)
                   VALUES (?1, ?2, ?3, ?4, ?5)"#,
                params![id, control_type, inspection_method, frequency, process_id],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache work instruction-specific data
    fn cache_work_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let process_id = value["process"].as_str();

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO works (id, process_id)
                   VALUES (?1, ?2)"#,
                params![id, process_id],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache NCR-specific data
    fn cache_ncr_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let ncr_type = value["ncr_type"].as_str();
        let severity = value["severity"].as_str();
        let ncr_status = value["ncr_status"].as_str();
        let category = value["category"].as_str();
        let disposition = value["disposition"]["decision"].as_str();
        let component_id = value["component"].as_str();
        let process_id = value["process"].as_str();

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO ncrs
                   (id, ncr_type, severity, ncr_status, category, disposition, component_id, process_id)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
                params![id, ncr_type, severity, ncr_status, category, disposition, component_id, process_id],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache CAPA-specific data
    fn cache_capa_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let capa_type = value["capa_type"].as_str();
        let capa_status = value["capa_status"].as_str();
        let effectiveness = value["effectiveness"].as_str();

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO capas (id, capa_type, capa_status, effectiveness)
                   VALUES (?1, ?2, ?3, ?4)"#,
                params![id, capa_type, capa_status, effectiveness],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Cache assembly-specific data
    fn cache_assembly_data(&self, id: &str, value: &serde_yml::Value) -> Result<()> {
        let part_number = value["part_number"].as_str();
        let revision = value["revision"].as_str();

        self.conn
            .execute(
                r#"INSERT OR REPLACE INTO assemblies (id, part_number, revision)
                   VALUES (?1, ?2, ?3)"#,
                params![id, part_number, revision],
            )
            .into_diagnostic()?;

        Ok(())
    }

    /// Incremental sync - only update changed files
    pub fn sync(&mut self) -> Result<SyncStats> {
        let start = std::time::Instant::now();
        let mut stats = SyncStats::default();

        // Get all current files
        let mut current_files: HashMap<String, PathBuf> = HashMap::new();

        for dir in Self::entity_directories() {
            let full_path = self.project_root.join(dir);
            if full_path.exists() {
                for entry in WalkDir::new(&full_path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let path = entry.path();
                    if path.to_string_lossy().ends_with(".tdt.yaml") {
                        let rel_path = path
                            .strip_prefix(&self.project_root)
                            .unwrap_or(path)
                            .to_string_lossy()
                            .to_string();
                        current_files.insert(rel_path, path.to_path_buf());
                        stats.files_scanned += 1;
                    }
                }
            }
        }

        // Get cached files
        let mut cached_files: HashMap<String, (i64, String)> = HashMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT file_path, file_mtime, file_hash FROM entities")
                .into_diagnostic()?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .into_diagnostic()?;

            for row in rows {
                let (path, mtime, hash) = row.into_diagnostic()?;
                cached_files.insert(path, (mtime, hash));
            }
        }

        // Find files to add/update
        for (rel_path, full_path) in &current_files {
            let needs_update = if let Some((cached_mtime, cached_hash)) = cached_files.get(rel_path)
            {
                // Check if file changed
                let current_mtime = get_file_mtime(full_path)?;
                if current_mtime != *cached_mtime {
                    // mtime changed, verify with hash
                    let content = fs::read_to_string(full_path).into_diagnostic()?;
                    let current_hash = compute_hash(&content);
                    current_hash != *cached_hash
                } else {
                    false
                }
            } else {
                true // New file
            };

            if needs_update {
                if cached_files.contains_key(rel_path) {
                    stats.entities_updated += 1;
                } else {
                    stats.entities_added += 1;
                }
                self.cache_entity_file(full_path)?;
            }
        }

        // Find files to remove
        for rel_path in cached_files.keys() {
            if !current_files.contains_key(rel_path) {
                // Get entity ID before removing
                let entity_id: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT id FROM entities WHERE file_path = ?1",
                        params![rel_path],
                        |row| row.get(0),
                    )
                    .optional()
                    .into_diagnostic()?;

                if let Some(id) = entity_id {
                    self.remove_entity(&id)?;
                    stats.entities_removed += 1;
                }
            }
        }

        stats.duration_ms = start.elapsed().as_millis() as u64;
        Ok(stats)
    }

    /// Remove an entity from the cache
    fn remove_entity(&self, id: &str) -> Result<()> {
        // Delete from main entities table
        self.conn
            .execute("DELETE FROM entities WHERE id = ?1", params![id])
            .into_diagnostic()?;
        // Delete from all type-specific tables
        self.conn
            .execute("DELETE FROM features WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM components WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM risks WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM tests WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM quotes WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM suppliers WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM processes WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM controls WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM works WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM ncrs WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM capas WHERE id = ?1", params![id])
            .into_diagnostic()?;
        self.conn
            .execute("DELETE FROM assemblies WHERE id = ?1", params![id])
            .into_diagnostic()?;
        // Delete links
        self.conn
            .execute("DELETE FROM links WHERE source_id = ?1", params![id])
            .into_diagnostic()?;
        // Note: We don't delete short IDs - they remain stable
        Ok(())
    }

    /// Ensure a short ID exists for an entity, creating one if needed
    pub fn ensure_short_id(&mut self, entity_id: &str) -> Result<String> {
        // Check if already exists
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT short_id FROM short_ids WHERE entity_id = ?1",
                params![entity_id],
                |row| row.get(0),
            )
            .optional()
            .into_diagnostic()?;

        if let Some(short_id) = existing {
            return Ok(short_id);
        }

        // Extract prefix
        let prefix = entity_id
            .split('-')
            .next()
            .ok_or_else(|| miette::miette!("Invalid entity ID format"))?;

        // Get next ID for this prefix
        let next_id: i64 = self
            .conn
            .query_row(
                "SELECT next_id FROM short_id_counters WHERE prefix = ?1",
                params![prefix],
                |row| row.get(0),
            )
            .optional()
            .into_diagnostic()?
            .unwrap_or(1);

        let short_id = format!("{}@{}", prefix, next_id);

        // Insert short ID mapping
        self.conn
            .execute(
                "INSERT INTO short_ids (short_id, entity_id, prefix) VALUES (?1, ?2, ?3)",
                params![short_id, entity_id, prefix],
            )
            .into_diagnostic()?;

        // Update counter
        self.conn
            .execute(
                "INSERT OR REPLACE INTO short_id_counters (prefix, next_id) VALUES (?1, ?2)",
                params![prefix, next_id + 1],
            )
            .into_diagnostic()?;

        Ok(short_id)
    }

    /// Resolve a short ID to full entity ID
    pub fn resolve_short_id(&self, short_id: &str) -> Option<String> {
        // Normalize: PREFIX@N format, case-insensitive prefix
        if let Some(at_pos) = short_id.find('@') {
            let prefix = &short_id[..at_pos];
            let num = &short_id[at_pos + 1..];
            let normalized = format!("{}@{}", prefix.to_ascii_uppercase(), num);

            self.conn
                .query_row(
                    "SELECT entity_id FROM short_ids WHERE short_id = ?1",
                    params![normalized],
                    |row| row.get(0),
                )
                .optional()
                .ok()
                .flatten()
        } else {
            None
        }
    }

    /// Get short ID for an entity
    pub fn get_short_id(&self, entity_id: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT short_id FROM short_ids WHERE entity_id = ?1",
                params![entity_id],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()
    }

    /// Get entity by ID (full or partial match)
    pub fn get_entity(&self, id: &str) -> Option<CachedEntity> {
        // Try exact match first
        let result = self.conn.query_row(
            "SELECT id, prefix, title, status, author, created, file_path, priority, entity_type, category, tags FROM entities WHERE id = ?1",
            params![id],
            |row| {
                let tags_str: Option<String> = row.get(10)?;
                let tags = tags_str
                    .map(|s| s.split(',').filter(|t| !t.is_empty()).map(String::from).collect())
                    .unwrap_or_default();
                Ok(CachedEntity {
                    id: row.get(0)?,
                    prefix: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    author: row.get(4)?,
                    created: parse_datetime(row.get::<_, String>(5)?),
                    file_path: PathBuf::from(row.get::<_, String>(6)?),
                    priority: row.get(7)?,
                    entity_type: row.get(8)?,
                    category: row.get(9)?,
                    tags,
                })
            },
        ).optional().ok().flatten();

        if result.is_some() {
            return result;
        }

        // Try partial match
        self.conn.query_row(
            "SELECT id, prefix, title, status, author, created, file_path, priority, entity_type, category, tags FROM entities WHERE id LIKE ?1",
            params![format!("%{}%", id)],
            |row| {
                let tags_str: Option<String> = row.get(10)?;
                let tags = tags_str
                    .map(|s| s.split(',').filter(|t| !t.is_empty()).map(String::from).collect())
                    .unwrap_or_default();
                Ok(CachedEntity {
                    id: row.get(0)?,
                    prefix: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    author: row.get(4)?,
                    created: parse_datetime(row.get::<_, String>(5)?),
                    file_path: PathBuf::from(row.get::<_, String>(6)?),
                    priority: row.get(7)?,
                    entity_type: row.get(8)?,
                    category: row.get(9)?,
                    tags,
                })
            },
        ).optional().ok().flatten()
    }

    /// Get feature by ID with dimension data
    pub fn get_feature(&self, id: &str) -> Option<CachedFeature> {
        self.conn.query_row(
            r#"SELECT e.id, e.title, e.status, f.component_id, f.feature_type,
                      f.dim_name, f.dim_nominal, f.dim_plus_tol, f.dim_minus_tol, f.dim_internal,
                      e.author, e.created, e.file_path
               FROM features f
               JOIN entities e ON f.id = e.id
               WHERE f.id = ?1"#,
            params![id],
            |row| {
                Ok(CachedFeature {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    status: row.get(2)?,
                    component_id: row.get(3)?,
                    feature_type: row.get(4)?,
                    dim_name: row.get(5)?,
                    dim_nominal: row.get(6)?,
                    dim_plus_tol: row.get(7)?,
                    dim_minus_tol: row.get(8)?,
                    dim_internal: row.get::<_, Option<i32>>(9)?.map(|v| v != 0),
                    author: row.get(10)?,
                    created: parse_datetime(row.get::<_, String>(11)?),
                    file_path: PathBuf::from(row.get::<_, String>(12)?),
                })
            },
        ).optional().ok().flatten()
    }

    /// Get all features for a component
    pub fn get_features_for_component(&self, component_id: &str) -> Vec<CachedFeature> {
        let mut stmt = match self.conn.prepare(
            r#"SELECT e.id, e.title, e.status, f.component_id, f.feature_type,
                      f.dim_name, f.dim_nominal, f.dim_plus_tol, f.dim_minus_tol, f.dim_internal,
                      e.author, e.created, e.file_path
               FROM features f
               JOIN entities e ON f.id = e.id
               WHERE f.component_id = ?1"#,
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let rows = match stmt.query_map(params![component_id], |row| {
            Ok(CachedFeature {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                component_id: row.get(3)?,
                feature_type: row.get(4)?,
                dim_name: row.get(5)?,
                dim_nominal: row.get(6)?,
                dim_plus_tol: row.get(7)?,
                dim_minus_tol: row.get(8)?,
                dim_internal: row.get::<_, Option<i32>>(9)?.map(|v| v != 0),
                author: row.get(10)?,
                created: parse_datetime(row.get::<_, String>(11)?),
                file_path: PathBuf::from(row.get::<_, String>(12)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// List entities with filters
    pub fn list_entities(&self, filter: &EntityFilter) -> Vec<CachedEntity> {
        let mut sql = String::from(
            "SELECT id, prefix, title, status, author, created, file_path, priority, entity_type, category, tags FROM entities WHERE 1=1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(ref prefix) = filter.prefix {
            sql.push_str(" AND prefix = ?");
            params_vec.push(Box::new(prefix.as_str().to_string()));
        }

        if let Some(ref status) = filter.status {
            sql.push_str(" AND status = ?");
            params_vec.push(Box::new(status.clone()));
        }

        if let Some(ref author) = filter.author {
            sql.push_str(" AND author = ?");
            params_vec.push(Box::new(author.clone()));
        }

        if let Some(ref priority) = filter.priority {
            sql.push_str(" AND priority = ?");
            params_vec.push(Box::new(priority.clone()));
        }

        if let Some(ref entity_type) = filter.entity_type {
            sql.push_str(" AND entity_type = ?");
            params_vec.push(Box::new(entity_type.clone()));
        }

        if let Some(ref category) = filter.category {
            sql.push_str(" AND category = ?");
            params_vec.push(Box::new(category.clone()));
        }

        if let Some(ref search) = filter.search {
            sql.push_str(" AND (title LIKE ? OR id LIKE ?)");
            let pattern = format!("%{}%", search);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        sql.push_str(" ORDER BY created DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = match stmt.query_map(params_refs.as_slice(), |row| {
            let tags_str: Option<String> = row.get(10)?;
            let tags = tags_str
                .map(|s| s.split(',').filter(|t| !t.is_empty()).map(String::from).collect())
                .unwrap_or_default();
            Ok(CachedEntity {
                id: row.get(0)?,
                prefix: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                author: row.get(4)?,
                created: parse_datetime(row.get::<_, String>(5)?),
                file_path: PathBuf::from(row.get::<_, String>(6)?),
                priority: row.get(7)?,
                entity_type: row.get(8)?,
                category: row.get(9)?,
                tags,
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// List suppliers with filtering
    /// Returns cached supplier data with support for status, capability, author filters
    pub fn list_suppliers(
        &self,
        status: Option<&str>,
        capability: Option<&str>,
        author: Option<&str>,
        search: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<CachedSupplier> {
        let mut sql = String::from(
            r#"SELECT e.id, e.title, s.short_name, e.status, e.author, e.created,
                      s.website, s.capabilities, s.lead_time_days, e.file_path
               FROM entities e
               JOIN suppliers s ON e.id = s.id
               WHERE e.prefix = 'SUP'"#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(status) = status {
            sql.push_str(" AND e.status = ?");
            params_vec.push(Box::new(status.to_string()));
        }

        if let Some(capability) = capability {
            // Match capability in comma-separated list
            sql.push_str(" AND (',' || s.capabilities || ',' LIKE ?)");
            params_vec.push(Box::new(format!("%,{},%", capability)));
        }

        if let Some(author) = author {
            sql.push_str(" AND e.author LIKE ?");
            params_vec.push(Box::new(format!("%{}%", author)));
        }

        if let Some(search) = search {
            sql.push_str(" AND (e.title LIKE ? OR e.id LIKE ?)");
            let pattern = format!("%{}%", search);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        sql.push_str(" ORDER BY e.title ASC");

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = match stmt.query_map(params_refs.as_slice(), |row| {
            let caps_str: Option<String> = row.get(7)?;
            let capabilities = caps_str
                .map(|s| s.split(',').filter(|t| !t.is_empty()).map(String::from).collect())
                .unwrap_or_default();
            Ok(CachedSupplier {
                id: row.get(0)?,
                name: row.get(1)?,
                short_name: row.get(2)?,
                status: row.get(3)?,
                author: row.get(4)?,
                created: parse_datetime(row.get::<_, String>(5)?),
                website: row.get(6)?,
                capabilities,
                lead_time_days: row.get(8)?,
                file_path: PathBuf::from(row.get::<_, String>(9)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// List requirements with filtering
    pub fn list_requirements(
        &self,
        status: Option<&str>,
        priority: Option<&str>,
        req_type: Option<&str>,
        category: Option<&str>,
        author: Option<&str>,
        search: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<CachedRequirement> {
        let mut sql = String::from(
            r#"SELECT e.id, e.title, e.status, e.priority, e.entity_type, e.category,
                      e.author, e.created, e.tags, e.file_path
               FROM entities e
               WHERE e.prefix = 'REQ'"#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(status) = status {
            sql.push_str(" AND e.status = ?");
            params_vec.push(Box::new(status.to_string()));
        }

        if let Some(priority) = priority {
            sql.push_str(" AND e.priority = ?");
            params_vec.push(Box::new(priority.to_string()));
        }

        if let Some(req_type) = req_type {
            sql.push_str(" AND e.entity_type = ?");
            params_vec.push(Box::new(req_type.to_string()));
        }

        if let Some(category) = category {
            sql.push_str(" AND e.category = ?");
            params_vec.push(Box::new(category.to_string()));
        }

        if let Some(author) = author {
            sql.push_str(" AND e.author LIKE ?");
            params_vec.push(Box::new(format!("%{}%", author)));
        }

        if let Some(search) = search {
            sql.push_str(" AND (e.title LIKE ? OR e.id LIKE ?)");
            let pattern = format!("%{}%", search);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        sql.push_str(" ORDER BY e.created DESC");

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = match stmt.query_map(params_refs.as_slice(), |row| {
            let tags_str: Option<String> = row.get(8)?;
            let tags = tags_str
                .map(|s| s.split(',').filter(|t| !t.is_empty()).map(String::from).collect())
                .unwrap_or_default();
            Ok(CachedRequirement {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                priority: row.get(3)?,
                req_type: row.get(4)?,
                category: row.get(5)?,
                author: row.get(6)?,
                created: parse_datetime(row.get::<_, String>(7)?),
                tags,
                file_path: PathBuf::from(row.get::<_, String>(9)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// List components with filtering
    pub fn list_components(
        &self,
        status: Option<&str>,
        make_buy: Option<&str>,
        category: Option<&str>,
        author: Option<&str>,
        search: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<CachedComponent> {
        let mut sql = String::from(
            r#"SELECT e.id, e.title, e.status, c.part_number, c.revision, c.make_buy,
                      c.category, e.author, e.created, e.file_path
               FROM entities e
               JOIN components c ON e.id = c.id
               WHERE e.prefix = 'CMP'"#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(status) = status {
            sql.push_str(" AND e.status = ?");
            params_vec.push(Box::new(status.to_string()));
        }

        if let Some(make_buy) = make_buy {
            sql.push_str(" AND c.make_buy = ?");
            params_vec.push(Box::new(make_buy.to_string()));
        }

        if let Some(category) = category {
            sql.push_str(" AND c.category = ?");
            params_vec.push(Box::new(category.to_string()));
        }

        if let Some(author) = author {
            sql.push_str(" AND e.author LIKE ?");
            params_vec.push(Box::new(format!("%{}%", author)));
        }

        if let Some(search) = search {
            sql.push_str(" AND (e.title LIKE ? OR e.id LIKE ? OR c.part_number LIKE ?)");
            let pattern = format!("%{}%", search);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        sql.push_str(" ORDER BY e.title ASC");

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = match stmt.query_map(params_refs.as_slice(), |row| {
            Ok(CachedComponent {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                part_number: row.get(3)?,
                revision: row.get(4)?,
                make_buy: row.get(5)?,
                category: row.get(6)?,
                author: row.get(7)?,
                created: parse_datetime(row.get::<_, String>(8)?),
                file_path: PathBuf::from(row.get::<_, String>(9)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// List tests with filtering
    pub fn list_tests(
        &self,
        status: Option<&str>,
        test_type: Option<&str>,
        level: Option<&str>,
        method: Option<&str>,
        priority: Option<&str>,
        category: Option<&str>,
        author: Option<&str>,
        search: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<CachedTest> {
        let mut sql = String::from(
            r#"SELECT e.id, e.title, e.status, t.test_type, t.level, t.method,
                      e.priority, e.category, e.author, e.created, e.file_path
               FROM entities e
               JOIN tests t ON e.id = t.id
               WHERE e.prefix = 'TEST'"#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(status) = status {
            sql.push_str(" AND e.status = ?");
            params_vec.push(Box::new(status.to_string()));
        }

        if let Some(test_type) = test_type {
            sql.push_str(" AND t.test_type = ?");
            params_vec.push(Box::new(test_type.to_string()));
        }

        if let Some(level) = level {
            sql.push_str(" AND t.level = ?");
            params_vec.push(Box::new(level.to_string()));
        }

        if let Some(method) = method {
            sql.push_str(" AND t.method = ?");
            params_vec.push(Box::new(method.to_string()));
        }

        if let Some(priority) = priority {
            sql.push_str(" AND e.priority = ?");
            params_vec.push(Box::new(priority.to_string()));
        }

        if let Some(category) = category {
            sql.push_str(" AND e.category = ?");
            params_vec.push(Box::new(category.to_string()));
        }

        if let Some(author) = author {
            sql.push_str(" AND e.author LIKE ?");
            params_vec.push(Box::new(format!("%{}%", author)));
        }

        if let Some(search) = search {
            sql.push_str(" AND (e.title LIKE ? OR e.id LIKE ?)");
            let pattern = format!("%{}%", search);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        sql.push_str(" ORDER BY e.created DESC");

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = match stmt.query_map(params_refs.as_slice(), |row| {
            Ok(CachedTest {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                test_type: row.get(3)?,
                level: row.get(4)?,
                method: row.get(5)?,
                priority: row.get(6)?,
                category: row.get(7)?,
                author: row.get(8)?,
                created: parse_datetime(row.get::<_, String>(9)?),
                file_path: PathBuf::from(row.get::<_, String>(10)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// List quotes with filtering
    pub fn list_quotes(
        &self,
        status: Option<&str>,
        quote_status: Option<&str>,
        supplier_id: Option<&str>,
        component_id: Option<&str>,
        author: Option<&str>,
        search: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<CachedQuote> {
        let mut sql = String::from(
            r#"SELECT e.id, e.title, e.status, q.quote_status, q.supplier_id, q.component_id, q.unit_price,
                      q.quantity, q.lead_time_days, q.currency, q.valid_until, e.author, e.created, e.file_path
               FROM entities e
               JOIN quotes q ON e.id = q.id
               WHERE e.prefix = 'QUOT'"#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(status) = status {
            sql.push_str(" AND e.status = ?");
            params_vec.push(Box::new(status.to_string()));
        }

        if let Some(quote_status) = quote_status {
            sql.push_str(" AND q.quote_status = ?");
            params_vec.push(Box::new(quote_status.to_string()));
        }

        if let Some(supplier_id) = supplier_id {
            sql.push_str(" AND q.supplier_id = ?");
            params_vec.push(Box::new(supplier_id.to_string()));
        }

        if let Some(component_id) = component_id {
            sql.push_str(" AND q.component_id = ?");
            params_vec.push(Box::new(component_id.to_string()));
        }

        if let Some(author) = author {
            sql.push_str(" AND e.author LIKE ?");
            params_vec.push(Box::new(format!("%{}%", author)));
        }

        if let Some(search) = search {
            sql.push_str(" AND (e.title LIKE ? OR e.id LIKE ?)");
            let pattern = format!("%{}%", search);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        sql.push_str(" ORDER BY e.created DESC");

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = match stmt.query_map(params_refs.as_slice(), |row| {
            Ok(CachedQuote {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                quote_status: row.get(3)?,
                supplier_id: row.get(4)?,
                component_id: row.get(5)?,
                unit_price: row.get(6)?,
                quantity: row.get(7)?,
                lead_time_days: row.get(8)?,
                currency: row.get(9)?,
                valid_until: row.get(10)?,
                author: row.get(11)?,
                created: parse_datetime(row.get::<_, String>(12)?),
                file_path: PathBuf::from(row.get::<_, String>(13)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// List NCRs with filtering
    pub fn list_ncrs(
        &self,
        status: Option<&str>,
        ncr_type: Option<&str>,
        severity: Option<&str>,
        ncr_status: Option<&str>,
        category: Option<&str>,
        author: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<CachedNcr> {
        let mut sql = String::from(
            r#"SELECT e.id, e.title, e.status, n.ncr_type, n.severity, n.ncr_status,
                      e.category, e.author, e.created, e.file_path
               FROM entities e
               JOIN ncrs n ON e.id = n.id
               WHERE e.prefix = 'NCR'"#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(status) = status {
            sql.push_str(" AND e.status = ?");
            params_vec.push(Box::new(status.to_string()));
        }

        if let Some(ncr_type) = ncr_type {
            sql.push_str(" AND n.ncr_type = ?");
            params_vec.push(Box::new(ncr_type.to_string()));
        }

        if let Some(severity) = severity {
            sql.push_str(" AND n.severity = ?");
            params_vec.push(Box::new(severity.to_string()));
        }

        if let Some(ncr_status) = ncr_status {
            sql.push_str(" AND n.ncr_status = ?");
            params_vec.push(Box::new(ncr_status.to_string()));
        }

        if let Some(category) = category {
            sql.push_str(" AND e.category = ?");
            params_vec.push(Box::new(category.to_string()));
        }

        if let Some(author) = author {
            sql.push_str(" AND e.author LIKE ?");
            params_vec.push(Box::new(format!("%{}%", author)));
        }

        sql.push_str(" ORDER BY e.created DESC");

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = match stmt.query_map(params_refs.as_slice(), |row| {
            Ok(CachedNcr {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                ncr_type: row.get(3)?,
                severity: row.get(4)?,
                ncr_status: row.get(5)?,
                category: row.get(6)?,
                author: row.get(7)?,
                created: parse_datetime(row.get::<_, String>(8)?),
                file_path: PathBuf::from(row.get::<_, String>(9)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// List CAPAs with filtering
    pub fn list_capas(
        &self,
        status: Option<&str>,
        capa_type: Option<&str>,
        capa_status: Option<&str>,
        author: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<CachedCapa> {
        let mut sql = String::from(
            r#"SELECT e.id, e.title, e.status, c.capa_type, c.capa_status,
                      e.author, e.created, e.file_path
               FROM entities e
               JOIN capas c ON e.id = c.id
               WHERE e.prefix = 'CAPA'"#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(status) = status {
            sql.push_str(" AND e.status = ?");
            params_vec.push(Box::new(status.to_string()));
        }

        if let Some(capa_type) = capa_type {
            sql.push_str(" AND c.capa_type = ?");
            params_vec.push(Box::new(capa_type.to_string()));
        }

        if let Some(capa_status) = capa_status {
            sql.push_str(" AND c.capa_status = ?");
            params_vec.push(Box::new(capa_status.to_string()));
        }

        if let Some(author) = author {
            sql.push_str(" AND e.author LIKE ?");
            params_vec.push(Box::new(format!("%{}%", author)));
        }

        sql.push_str(" ORDER BY e.created DESC");

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = match stmt.query_map(params_refs.as_slice(), |row| {
            Ok(CachedCapa {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                capa_type: row.get(3)?,
                capa_status: row.get(4)?,
                author: row.get(5)?,
                created: parse_datetime(row.get::<_, String>(6)?),
                file_path: PathBuf::from(row.get::<_, String>(7)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// List risks with filtering
    pub fn list_risks(
        &self,
        status: Option<&str>,
        risk_type: Option<&str>,
        risk_level: Option<&str>,
        category: Option<&str>,
        min_rpn: Option<i32>,
        author: Option<&str>,
        search: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<CachedRisk> {
        let mut sql = String::from(
            r#"SELECT e.id, e.title, e.status, r.risk_type, r.severity, r.occurrence, r.detection,
                      r.rpn, r.risk_level, e.category, e.author, e.created, e.file_path
               FROM entities e
               JOIN risks r ON e.id = r.id
               WHERE e.prefix = 'RISK'"#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(status) = status {
            sql.push_str(" AND e.status = ?");
            params_vec.push(Box::new(status.to_string()));
        }

        if let Some(risk_type) = risk_type {
            sql.push_str(" AND r.risk_type = ?");
            params_vec.push(Box::new(risk_type.to_string()));
        }

        if let Some(risk_level) = risk_level {
            sql.push_str(" AND r.risk_level = ?");
            params_vec.push(Box::new(risk_level.to_string()));
        }

        if let Some(category) = category {
            sql.push_str(" AND e.category = ?");
            params_vec.push(Box::new(category.to_string()));
        }

        if let Some(min_rpn) = min_rpn {
            sql.push_str(" AND r.rpn >= ?");
            params_vec.push(Box::new(min_rpn));
        }

        if let Some(author) = author {
            sql.push_str(" AND e.author LIKE ?");
            params_vec.push(Box::new(format!("%{}%", author)));
        }

        if let Some(search) = search {
            sql.push_str(" AND (e.title LIKE ? OR e.id LIKE ?)");
            let pattern = format!("%{}%", search);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        sql.push_str(" ORDER BY r.rpn DESC, e.created DESC");

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = match stmt.query_map(params_refs.as_slice(), |row| {
            Ok(CachedRisk {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                risk_type: row.get(3)?,
                severity: row.get(4)?,
                occurrence: row.get(5)?,
                detection: row.get(6)?,
                rpn: row.get(7)?,
                risk_level: row.get(8)?,
                category: row.get(9)?,
                author: row.get(10)?,
                created: parse_datetime(row.get::<_, String>(11)?),
                file_path: PathBuf::from(row.get::<_, String>(12)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// List features with filtering
    pub fn list_features(
        &self,
        status: Option<&str>,
        feature_type: Option<&str>,
        component_id: Option<&str>,
        author: Option<&str>,
        search: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<CachedFeature> {
        let mut sql = String::from(
            r#"SELECT e.id, e.title, e.status, f.component_id, f.feature_type,
                      f.dim_name, f.dim_nominal, f.dim_plus_tol, f.dim_minus_tol, f.dim_internal,
                      e.author, e.created, e.file_path
               FROM entities e
               JOIN features f ON e.id = f.id
               WHERE e.prefix = 'FEAT'"#,
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(status) = status {
            sql.push_str(" AND e.status = ?");
            params_vec.push(Box::new(status.to_string()));
        }

        if let Some(feature_type) = feature_type {
            sql.push_str(" AND f.feature_type = ?");
            params_vec.push(Box::new(feature_type.to_string()));
        }

        if let Some(component_id) = component_id {
            sql.push_str(" AND f.component_id = ?");
            params_vec.push(Box::new(component_id.to_string()));
        }

        if let Some(author) = author {
            sql.push_str(" AND e.author LIKE ?");
            params_vec.push(Box::new(format!("%{}%", author)));
        }

        if let Some(search) = search {
            sql.push_str(" AND (e.title LIKE ? OR e.id LIKE ?)");
            let pattern = format!("%{}%", search);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        sql.push_str(" ORDER BY e.created DESC");

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = match stmt.query_map(params_refs.as_slice(), |row| {
            Ok(CachedFeature {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                component_id: row.get(3)?,
                feature_type: row.get(4)?,
                dim_name: row.get(5)?,
                dim_nominal: row.get(6)?,
                dim_plus_tol: row.get(7)?,
                dim_minus_tol: row.get(8)?,
                dim_internal: row.get(9)?,
                author: row.get(10)?,
                created: parse_datetime(row.get::<_, String>(11)?),
                file_path: PathBuf::from(row.get::<_, String>(12)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get all cached features (for validation)
    pub fn get_all_features(&self) -> HashMap<String, CachedFeature> {
        let mut result = HashMap::new();

        let mut stmt = match self.conn.prepare(
            r#"SELECT e.id, e.title, e.status, f.component_id, f.feature_type,
                      f.dim_name, f.dim_nominal, f.dim_plus_tol, f.dim_minus_tol, f.dim_internal,
                      e.author, e.created, e.file_path
               FROM features f
               JOIN entities e ON f.id = e.id"#,
        ) {
            Ok(s) => s,
            Err(_) => return result,
        };

        let rows = match stmt.query_map([], |row| {
            Ok(CachedFeature {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                component_id: row.get(3)?,
                feature_type: row.get(4)?,
                dim_name: row.get(5)?,
                dim_nominal: row.get(6)?,
                dim_plus_tol: row.get(7)?,
                dim_minus_tol: row.get(8)?,
                dim_internal: row.get::<_, Option<i32>>(9)?.map(|v| v != 0),
                author: row.get(10)?,
                created: parse_datetime(row.get::<_, String>(11)?),
                file_path: PathBuf::from(row.get::<_, String>(12)?),
            })
        }) {
            Ok(r) => r,
            Err(_) => return result,
        };

        for row in rows.flatten() {
            result.insert(row.id.clone(), row);
        }

        result
    }

    // =========================================================================
    // Link Query Methods (for trace operations)
    // =========================================================================

    /// Get all outgoing links from an entity (what it links TO)
    pub fn get_links_from(&self, source_id: &str) -> Vec<CachedLink> {
        let mut stmt = match self.conn.prepare(
            "SELECT source_id, target_id, link_type FROM links WHERE source_id = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let rows = match stmt.query_map(params![source_id], |row| {
            Ok(CachedLink {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                link_type: row.get(2)?,
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get all incoming links to an entity (what links TO it)
    pub fn get_links_to(&self, target_id: &str) -> Vec<CachedLink> {
        let mut stmt = match self.conn.prepare(
            "SELECT source_id, target_id, link_type FROM links WHERE target_id = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let rows = match stmt.query_map(params![target_id], |row| {
            Ok(CachedLink {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                link_type: row.get(2)?,
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get links of a specific type from an entity
    pub fn get_links_from_of_type(&self, source_id: &str, link_type: &str) -> Vec<String> {
        let mut stmt = match self.conn.prepare(
            "SELECT target_id FROM links WHERE source_id = ?1 AND link_type = ?2",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let rows = match stmt.query_map(params![source_id, link_type], |row| row.get(0)) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get links of a specific type to an entity
    pub fn get_links_to_of_type(&self, target_id: &str, link_type: &str) -> Vec<String> {
        let mut stmt = match self.conn.prepare(
            "SELECT source_id FROM links WHERE target_id = ?1 AND link_type = ?2",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let rows = match stmt.query_map(params![target_id, link_type], |row| row.get(0)) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Trace forward from an entity (recursive)
    /// Returns all entities reachable from source via outgoing links
    pub fn trace_from(&self, source_id: &str, max_depth: usize) -> Vec<(String, String, usize)> {
        let mut results = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back((source_id.to_string(), 0usize));
        visited.insert(source_id.to_string());

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            for link in self.get_links_from(&current_id) {
                if !visited.contains(&link.target_id) {
                    visited.insert(link.target_id.clone());
                    results.push((link.target_id.clone(), link.link_type.clone(), depth + 1));
                    queue.push_back((link.target_id, depth + 1));
                }
            }
        }

        results
    }

    /// Trace backward to an entity (recursive)
    /// Returns all entities that can reach target via outgoing links
    pub fn trace_to(&self, target_id: &str, max_depth: usize) -> Vec<(String, String, usize)> {
        let mut results = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back((target_id.to_string(), 0usize));
        visited.insert(target_id.to_string());

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            for link in self.get_links_to(&current_id) {
                if !visited.contains(&link.source_id) {
                    visited.insert(link.source_id.clone());
                    results.push((link.source_id.clone(), link.link_type.clone(), depth + 1));
                    queue.push_back((link.source_id, depth + 1));
                }
            }
        }

        results
    }

    /// Find orphan entities (no incoming or outgoing links)
    pub fn find_orphans(&self, prefix: Option<&str>) -> Vec<CachedEntity> {
        let sql = if let Some(p) = prefix {
            format!(
                r#"SELECT e.id, e.prefix, e.title, e.status, e.author, e.created, e.file_path,
                          e.priority, e.entity_type, e.category, e.tags
                   FROM entities e
                   WHERE e.prefix = '{}'
                   AND NOT EXISTS (SELECT 1 FROM links WHERE source_id = e.id)
                   AND NOT EXISTS (SELECT 1 FROM links WHERE target_id = e.id)"#,
                p
            )
        } else {
            r#"SELECT e.id, e.prefix, e.title, e.status, e.author, e.created, e.file_path,
                      e.priority, e.entity_type, e.category, e.tags
               FROM entities e
               WHERE NOT EXISTS (SELECT 1 FROM links WHERE source_id = e.id)
               AND NOT EXISTS (SELECT 1 FROM links WHERE target_id = e.id)"#
                .to_string()
        };

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let rows = match stmt.query_map([], |row| {
            let tags_str: Option<String> = row.get(10)?;
            let tags = tags_str
                .map(|s| s.split(',').filter(|t| !t.is_empty()).map(String::from).collect())
                .unwrap_or_default();
            Ok(CachedEntity {
                id: row.get(0)?,
                prefix: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                author: row.get(4)?,
                created: parse_datetime(row.get::<_, String>(5)?),
                file_path: PathBuf::from(row.get::<_, String>(6)?),
                priority: row.get(7)?,
                entity_type: row.get(8)?,
                category: row.get(9)?,
                tags,
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Count links by type (for statistics)
    pub fn count_links_by_type(&self) -> HashMap<String, usize> {
        let mut result = HashMap::new();

        let mut stmt = match self
            .conn
            .prepare("SELECT link_type, COUNT(*) FROM links GROUP BY link_type")
        {
            Ok(s) => s,
            Err(_) => return result,
        };

        let rows = match stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
        }) {
            Ok(r) => r,
            Err(_) => return result,
        };

        for row in rows.flatten() {
            result.insert(row.0, row.1);
        }

        result
    }

    /// Get all entity IDs that have links (either direction)
    pub fn get_linked_entity_ids(&self) -> std::collections::HashSet<String> {
        let mut result = std::collections::HashSet::new();

        if let Ok(mut stmt) = self
            .conn
            .prepare("SELECT DISTINCT source_id FROM links UNION SELECT DISTINCT target_id FROM links")
        {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for row in rows.flatten() {
                    result.insert(row);
                }
            }
        }

        result
    }

    /// Get cache statistics
    pub fn statistics(&self) -> Result<CacheStats> {
        let total_entities: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
            .into_diagnostic()?;

        let total_short_ids: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM short_ids", [], |row| row.get(0))
            .into_diagnostic()?;

        let mut by_prefix = HashMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT prefix, COUNT(*) FROM entities GROUP BY prefix")
                .into_diagnostic()?;
            let rows = stmt
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?)))
                .into_diagnostic()?;

            for row in rows {
                let (prefix, count) = row.into_diagnostic()?;
                by_prefix.insert(prefix, count);
            }
        }

        let db_path = self.project_root.join(CACHE_FILE);
        let db_size_bytes = fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

        Ok(CacheStats {
            total_entities,
            total_short_ids,
            by_prefix,
            db_size_bytes,
        })
    }

    /// Execute raw SQL query (read-only)
    pub fn query_raw(&self, sql: &str) -> Result<Vec<Vec<String>>> {
        let mut stmt = self.conn.prepare(sql).into_diagnostic()?;
        let column_count = stmt.column_count();

        let rows = stmt
            .query_map([], |row| {
                let mut values = Vec::with_capacity(column_count);
                for i in 0..column_count {
                    let value: String = row
                        .get::<_, rusqlite::types::Value>(i)
                        .map(|v| match v {
                            rusqlite::types::Value::Null => "NULL".to_string(),
                            rusqlite::types::Value::Integer(i) => i.to_string(),
                            rusqlite::types::Value::Real(f) => f.to_string(),
                            rusqlite::types::Value::Text(s) => s,
                            rusqlite::types::Value::Blob(_) => "<blob>".to_string(),
                        })
                        .unwrap_or_default();
                    values.push(value);
                }
                Ok(values)
            })
            .into_diagnostic()?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .into_diagnostic()
    }

    /// Get column names for a query
    pub fn query_columns(&self, sql: &str) -> Result<Vec<String>> {
        let stmt = self.conn.prepare(sql).into_diagnostic()?;
        Ok(stmt.column_names().iter().map(|s| s.to_string()).collect())
    }

    /// Clear the entire cache (for testing or reset)
    pub fn clear(&mut self) -> Result<()> {
        self.conn
            .execute_batch(
                r#"
            DELETE FROM entities;
            DELETE FROM features;
            DELETE FROM components;
            DELETE FROM risks;
            DELETE FROM short_ids;
            DELETE FROM short_id_counters;
            "#,
            )
            .into_diagnostic()?;
        Ok(())
    }
}

/// Get file modification time as Unix timestamp
fn get_file_mtime(path: &Path) -> Result<i64> {
    let metadata = fs::metadata(path).into_diagnostic()?;
    let mtime = metadata
        .modified()
        .into_diagnostic()?
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Ok(mtime)
}

/// Compute SHA256 hash of content
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Parse datetime string to DateTime<Utc>
fn parse_datetime(s: String) -> DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_project() -> (tempfile::TempDir, Project) {
        let tmp = tempdir().unwrap();
        let project = Project::init(tmp.path()).unwrap();
        (tmp, project)
    }

    fn write_test_entity(project: &Project, rel_path: &str, content: &str) {
        let full_path = project.root().join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, content).unwrap();
    }

    #[test]
    fn test_cache_creation() {
        let (_tmp, project) = create_test_project();
        let cache = EntityCache::open(&project).unwrap();

        let stats = cache.statistics().unwrap();
        assert_eq!(stats.total_entities, 0);
        assert_eq!(stats.total_short_ids, 0);
    }

    #[test]
    fn test_short_id_assignment() {
        let (_tmp, project) = create_test_project();
        let mut cache = EntityCache::open_without_sync(&project).unwrap();

        let short1 = cache.ensure_short_id("REQ-01ABC123").unwrap();
        let short2 = cache.ensure_short_id("REQ-02DEF456").unwrap();
        let short3 = cache.ensure_short_id("RISK-01GHI789").unwrap();

        assert_eq!(short1, "REQ@1");
        assert_eq!(short2, "REQ@2");
        assert_eq!(short3, "RISK@1");

        // Same ID should return same short ID
        let short1_again = cache.ensure_short_id("REQ-01ABC123").unwrap();
        assert_eq!(short1_again, "REQ@1");
    }

    #[test]
    fn test_short_id_resolution() {
        let (_tmp, project) = create_test_project();
        let mut cache = EntityCache::open_without_sync(&project).unwrap();

        cache.ensure_short_id("REQ-01ABC123").unwrap();

        // Test resolution
        assert_eq!(
            cache.resolve_short_id("REQ@1"),
            Some("REQ-01ABC123".to_string())
        );
        assert_eq!(
            cache.resolve_short_id("req@1"),
            Some("REQ-01ABC123".to_string())
        );
        assert_eq!(
            cache.resolve_short_id("Req@1"),
            Some("REQ-01ABC123".to_string())
        );
        assert_eq!(cache.resolve_short_id("REQ@99"), None);
    }

    #[test]
    fn test_entity_caching() {
        let (_tmp, project) = create_test_project();

        write_test_entity(
            &project,
            "requirements/inputs/REQ-01ABC123.tdt.yaml",
            r#"
id: REQ-01ABC123
title: Test Requirement
status: draft
author: Test Author
created: 2024-01-15T10:30:00Z
"#,
        );

        let mut cache = EntityCache::open_without_sync(&project).unwrap();
        let stats = cache.rebuild().unwrap();

        assert_eq!(stats.entities_added, 1);

        let entity = cache.get_entity("REQ-01ABC123").unwrap();
        assert_eq!(entity.title, "Test Requirement");
        assert_eq!(entity.status, "draft");
        assert_eq!(entity.author, "Test Author");
    }

    #[test]
    fn test_feature_caching() {
        let (_tmp, project) = create_test_project();

        write_test_entity(
            &project,
            "tolerances/features/FEAT-01ABC123.tdt.yaml",
            r#"
id: FEAT-01ABC123
component: CMP-01XYZ789
feature_type: internal
title: Mounting Hole
status: draft
author: Test Author
created: 2024-01-15T10:30:00Z
dimensions:
  - name: diameter
    nominal: 10.0
    plus_tol: 0.1
    minus_tol: 0.05
    internal: true
"#,
        );

        let mut cache = EntityCache::open_without_sync(&project).unwrap();
        cache.rebuild().unwrap();

        let feature = cache.get_feature("FEAT-01ABC123").unwrap();
        assert_eq!(feature.component_id, "CMP-01XYZ789");
        assert_eq!(feature.feature_type, "internal");
        assert_eq!(feature.dim_name, Some("diameter".to_string()));
        assert_eq!(feature.dim_nominal, Some(10.0));
        assert_eq!(feature.dim_plus_tol, Some(0.1));
        assert_eq!(feature.dim_minus_tol, Some(0.05));
        assert_eq!(feature.dim_internal, Some(true));
    }

    #[test]
    fn test_incremental_sync_add() {
        let (_tmp, project) = create_test_project();
        let mut cache = EntityCache::open(&project).unwrap();

        // Initially empty
        let stats = cache.statistics().unwrap();
        assert_eq!(stats.total_entities, 0);

        // Add a file
        write_test_entity(
            &project,
            "requirements/inputs/REQ-01ABC123.tdt.yaml",
            r#"
id: REQ-01ABC123
title: New Requirement
status: draft
author: Test Author
created: 2024-01-15T10:30:00Z
"#,
        );

        // Sync should detect the new file
        let sync_stats = cache.sync().unwrap();
        assert_eq!(sync_stats.entities_added, 1);
        assert_eq!(sync_stats.entities_updated, 0);
        assert_eq!(sync_stats.entities_removed, 0);

        let stats = cache.statistics().unwrap();
        assert_eq!(stats.total_entities, 1);
    }

    #[test]
    fn test_incremental_sync_remove() {
        let (_tmp, project) = create_test_project();

        // Create initial file
        let file_path = project
            .root()
            .join("requirements/inputs/REQ-01ABC123.tdt.yaml");
        write_test_entity(
            &project,
            "requirements/inputs/REQ-01ABC123.tdt.yaml",
            r#"
id: REQ-01ABC123
title: To Be Deleted
status: draft
author: Test Author
created: 2024-01-15T10:30:00Z
"#,
        );

        let mut cache = EntityCache::open(&project).unwrap();
        let stats = cache.statistics().unwrap();
        assert_eq!(stats.total_entities, 1);

        // Delete the file
        fs::remove_file(&file_path).unwrap();

        // Sync should detect removal
        let sync_stats = cache.sync().unwrap();
        assert_eq!(sync_stats.entities_removed, 1);

        let stats = cache.statistics().unwrap();
        assert_eq!(stats.total_entities, 0);
    }

    #[test]
    fn test_list_entities_with_filter() {
        let (_tmp, project) = create_test_project();

        write_test_entity(
            &project,
            "requirements/inputs/REQ-01ABC.tdt.yaml",
            r#"
id: REQ-01ABC
title: Requirement One
status: draft
author: Alice
created: 2024-01-15T10:30:00Z
"#,
        );

        write_test_entity(
            &project,
            "requirements/inputs/REQ-02DEF.tdt.yaml",
            r#"
id: REQ-02DEF
title: Requirement Two
status: approved
author: Bob
created: 2024-01-16T10:30:00Z
"#,
        );

        write_test_entity(
            &project,
            "risks/design/RISK-01GHI.tdt.yaml",
            r#"
id: RISK-01GHI
title: Risk One
status: draft
author: Alice
created: 2024-01-17T10:30:00Z
"#,
        );

        let mut cache = EntityCache::open_without_sync(&project).unwrap();
        cache.rebuild().unwrap();

        // Filter by prefix
        let reqs = cache.list_entities(&EntityFilter {
            prefix: Some(EntityPrefix::Req),
            ..Default::default()
        });
        assert_eq!(reqs.len(), 2);

        // Filter by status
        let approved = cache.list_entities(&EntityFilter {
            status: Some("approved".to_string()),
            ..Default::default()
        });
        assert_eq!(approved.len(), 1);
        assert_eq!(approved[0].title, "Requirement Two");

        // Filter by author
        let alice = cache.list_entities(&EntityFilter {
            author: Some("Alice".to_string()),
            ..Default::default()
        });
        assert_eq!(alice.len(), 2);
    }

    #[test]
    fn test_raw_query() {
        let (_tmp, project) = create_test_project();

        write_test_entity(
            &project,
            "requirements/inputs/REQ-01ABC.tdt.yaml",
            r#"
id: REQ-01ABC
title: Test Req
status: draft
author: Test
created: 2024-01-15T10:30:00Z
"#,
        );

        let mut cache = EntityCache::open_without_sync(&project).unwrap();
        cache.rebuild().unwrap();

        let result = cache
            .query_raw("SELECT id, title FROM entities WHERE prefix = 'REQ'")
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0][0], "REQ-01ABC");
        assert_eq!(result[0][1], "Test Req");
    }

    #[test]
    fn test_short_ids_not_in_files() {
        let (_tmp, project) = create_test_project();

        write_test_entity(
            &project,
            "requirements/inputs/REQ-01ABC123.tdt.yaml",
            r#"
id: REQ-01ABC123
title: Test Requirement
status: draft
author: Test Author
created: 2024-01-15T10:30:00Z
traces_to:
  - REQ-02DEF456
"#,
        );

        // Read the file back
        let content = fs::read_to_string(
            project
                .root()
                .join("requirements/inputs/REQ-01ABC123.tdt.yaml"),
        )
        .unwrap();

        // Verify no short IDs in file content
        assert!(!content.contains('@'), "File should not contain short IDs");
        assert!(
            content.contains("REQ-01ABC123"),
            "File should contain full ULID"
        );
        assert!(
            content.contains("REQ-02DEF456"),
            "References should use full ULIDs"
        );
    }

    #[test]
    fn test_cache_survives_rebuild() {
        let (_tmp, project) = create_test_project();

        write_test_entity(
            &project,
            "requirements/inputs/REQ-01ABC.tdt.yaml",
            r#"
id: REQ-01ABC
title: Persistent Req
status: draft
author: Test
created: 2024-01-15T10:30:00Z
"#,
        );

        // First cache instance
        {
            let mut cache = EntityCache::open(&project).unwrap();
            let short_id = cache.ensure_short_id("REQ-01ABC").unwrap();
            assert_eq!(short_id, "REQ@1");
        }

        // Second cache instance (should load existing data)
        {
            let cache = EntityCache::open(&project).unwrap();
            let resolved = cache.resolve_short_id("REQ@1");
            assert_eq!(resolved, Some("REQ-01ABC".to_string()));
        }
    }

    #[test]
    fn test_features_for_component() {
        let (_tmp, project) = create_test_project();

        write_test_entity(
            &project,
            "tolerances/features/FEAT-01A.tdt.yaml",
            r#"
id: FEAT-01A
component: CMP-001
feature_type: internal
title: Hole A
status: draft
author: Test
created: 2024-01-15T10:30:00Z
dimensions:
  - name: diameter
    nominal: 10.0
    plus_tol: 0.1
    minus_tol: 0.05
    internal: true
"#,
        );

        write_test_entity(
            &project,
            "tolerances/features/FEAT-02B.tdt.yaml",
            r#"
id: FEAT-02B
component: CMP-001
feature_type: external
title: Shaft B
status: draft
author: Test
created: 2024-01-15T10:30:00Z
dimensions:
  - name: diameter
    nominal: 9.9
    plus_tol: 0.05
    minus_tol: 0.1
    internal: false
"#,
        );

        write_test_entity(
            &project,
            "tolerances/features/FEAT-03C.tdt.yaml",
            r#"
id: FEAT-03C
component: CMP-002
feature_type: internal
title: Hole C
status: draft
author: Test
created: 2024-01-15T10:30:00Z
"#,
        );

        let mut cache = EntityCache::open_without_sync(&project).unwrap();
        cache.rebuild().unwrap();

        let features = cache.get_features_for_component("CMP-001");
        assert_eq!(features.len(), 2);

        let features2 = cache.get_features_for_component("CMP-002");
        assert_eq!(features2.len(), 1);
    }
}
