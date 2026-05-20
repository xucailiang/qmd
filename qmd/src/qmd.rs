//! Top-level facade: [`Qmd`] owns the search database.
//!
//! ```rust,no_run
//! use qmd::{Qmd, Collection};
//!
//! let mut qmd = Qmd::open("./index.sqlite")?;
//! qmd.register_collection(&Collection::new("docs", "/path/to/docs"))?;
//! qmd.update(None)?;
//! let results = qmd.search("how does auth work?", 10)?;
//! # Ok::<(), qmd::Error>(())
//! ```

use std::collections::HashSet;
use std::path::Path;

use ignore::WalkBuilder;

use crate::db::{
    Collection, CollectionInfo, Db, Document, IndexStatus, SearchResult, extract_title,
    hash_content,
};
use crate::error::{Error, Result};
use crate::search;

/// The main qmd handle.
///
/// Owns the SQLite database and exposes collection, indexing, and BM25 search
/// operations.
pub struct Qmd {
    /// SQLite database handle.
    db: Db,
}

impl std::fmt::Debug for Qmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Qmd").finish_non_exhaustive()
    }
}

impl Qmd {
    /// Open (or create) a qmd index at the given SQLite path.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            db: Db::open(db_path.as_ref())?,
        })
    }

    /// Open an in-memory index (useful for tests).
    pub fn open_memory() -> Result<Self> {
        Ok(Self {
            db: Db::open_memory()?,
        })
    }

    /// Access the underlying database.
    #[must_use]
    pub const fn db(&self) -> &Db {
        &self.db
    }

    // ── Collection management ───────────────────────────────────────────

    /// Register (or update) a collection. Does NOT index files.
    /// Call [`update`](Self::update) afterwards to scan the filesystem.
    pub fn register_collection(&self, coll: &Collection) -> Result<()> {
        let path = Path::new(&coll.path);
        if !path.is_dir() {
            return Err(Error::Config(format!("not a directory: {}", coll.path)));
        }
        self.db.upsert_collection(coll)
    }

    /// Remove a collection and all its documents.
    pub fn remove_collection(&self, name: &str) -> Result<usize> {
        self.db.delete_collection(name)
    }

    /// Rename a collection.
    pub fn rename_collection(&self, old_name: &str, new_name: &str) -> Result<()> {
        self.db.rename_collection(old_name, new_name)
    }

    /// List all registered collections with stats.
    pub fn list_collections(&self) -> Result<Vec<CollectionInfo>> {
        Ok(self.db.status()?.collections)
    }

    // ── Context management ──────────────────────────────────────────────

    /// Set a path-scoped context for a collection.
    pub fn set_context(&self, collection: &str, path_prefix: &str, text: &str) -> Result<bool> {
        self.db.set_context(collection, path_prefix, text)
    }

    /// Remove a path-scoped context.
    pub fn remove_context(&self, collection: &str, path_prefix: &str) -> Result<bool> {
        self.db.remove_context(collection, path_prefix)
    }

    /// Set the global context (applies to all collections).
    pub fn set_global_context(&self, text: Option<&str>) -> Result<()> {
        self.db.set_global_context(text)
    }

    /// Get the global context.
    pub fn global_context(&self) -> Result<Option<String>> {
        self.db.global_context()
    }

    // ── Indexing ────────────────────────────────────────────────────────

    /// Scan registered collections and incrementally index files.
    ///
    /// If `collections` is `None`, all registered collections are scanned.
    /// Pass a slice of names to limit to specific collections.
    pub fn update(&self, collections: Option<&[&str]>) -> Result<UpdateResult> {
        let all_colls = self.db.list_collections()?;
        let colls: Vec<&Collection> = if let Some(names) = collections {
            let set: HashSet<&str> = names.iter().copied().collect();
            all_colls
                .iter()
                .filter(|c| set.contains(c.name.as_str()))
                .collect()
        } else {
            all_colls.iter().collect()
        };

        let mut total = UpdateResult::default();

        for coll in colls {
            let r = self.index_collection(coll)?;
            total.indexed += r.indexed;
            total.updated += r.updated;
            total.unchanged += r.unchanged;
            total.removed += r.removed;
            total.collections += 1;
        }

        Ok(total)
    }

    /// Index a single collection by scanning the filesystem.
    fn index_collection(&self, coll: &Collection) -> Result<IndexResult> {
        let base = Path::new(&coll.path);
        if !base.is_dir() {
            return Err(Error::Config(format!("not a directory: {}", coll.path)));
        }

        let files = walk_collection(base, &coll.pattern, &coll.ignore)?;

        let existing = self.db.active_paths(&coll.name)?;
        let existing_set: HashSet<&str> = existing.iter().map(String::as_str).collect();

        let mut indexed = 0usize;
        let mut updated = 0usize;
        let mut unchanged = 0usize;

        for file_path in &files {
            let rel = file_path
                .strip_prefix(base)
                .unwrap_or(file_path)
                .to_string_lossy()
                .replace('\\', "/");

            let Ok(content) = std::fs::read_to_string(file_path) else {
                continue;
            };
            if content.trim().is_empty() {
                continue;
            }

            let hash = hash_content(&content);
            let title = extract_title(&content, &rel);

            if let Some(existing_doc) = self.db.get_document(&coll.name, &rel)? {
                if existing_doc.hash == hash {
                    unchanged += 1;
                    continue;
                }
                updated += 1;
            } else {
                indexed += 1;
            }

            self.db.insert_content(&hash, &content)?;
            self.db.upsert_document(&coll.name, &rel, &title, &hash)?;
        }

        let new_paths: HashSet<String> = files
            .iter()
            .filter_map(|p| {
                p.strip_prefix(base)
                    .ok()
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
            })
            .collect();

        let mut removed = 0usize;
        for path in &existing_set {
            if !new_paths.contains(*path) {
                self.db.deactivate(&coll.name, path)?;
                removed += 1;
            }
        }

        Ok(IndexResult {
            indexed,
            updated,
            unchanged,
            removed,
        })
    }

    // ── Search ──────────────────────────────────────────────────────────

    /// Full-text search using BM25.
    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let fts_query = search::build_fts5_query(query).unwrap_or_else(|| query.to_string());
        self.db.search_fts(&fts_query, limit, None)
    }

    /// Search documents using BM25 full-text search.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search_fts(query, limit)
    }

    // ── Document retrieval ──────────────────────────────────────────────

    /// Get a document by `collection/path` or by docid (`#abc123`).
    pub fn get(&self, path_or_docid: &str) -> Result<Document> {
        let clean = path_or_docid.trim_start_matches('#');
        if clean.len() == 6 && clean.chars().all(|c| c.is_ascii_hexdigit()) {
            if let Some((coll, path)) = self.db.find_by_docid(clean)? {
                return self
                    .db
                    .get_document(&coll, &path)?
                    .ok_or_else(|| Error::NotFound(path_or_docid.to_string()));
            }
        } else if let Some((coll, path)) = path_or_docid.split_once('/') {
            return self
                .db
                .get_document(coll, path)?
                .ok_or_else(|| Error::NotFound(path_or_docid.to_string()));
        }
        Err(Error::NotFound(path_or_docid.to_string()))
    }

    // ── Index health ────────────────────────────────────────────────────

    /// Get full index status.
    pub fn status(&self) -> Result<IndexStatus> {
        self.db.status()
    }

    /// Count active documents.
    pub fn doc_count(&self) -> Result<usize> {
        self.db.doc_count()
    }

    // ── Maintenance ─────────────────────────────────────────────────────

    /// Delete inactive documents and orphaned data.
    pub fn cleanup(&self) -> Result<usize> {
        self.db.cleanup()
    }

    /// Vacuum the database.
    pub fn vacuum(&self) -> Result<()> {
        self.db.vacuum()
    }
}

/// Walk a collection directory using the `ignore` crate (gitignore-aware).
fn walk_collection(
    base: &Path,
    pattern: &str,
    ignore_patterns: &[String],
) -> Result<Vec<std::path::PathBuf>> {
    let mut builder = ignore::overrides::OverrideBuilder::new(base);
    builder
        .add(pattern)
        .map_err(|e| Error::Config(e.to_string()))?;

    for pat in ignore_patterns {
        let negated = format!("!{pat}");
        builder
            .add(&negated)
            .map_err(|e| Error::Config(e.to_string()))?;
    }

    for dir in EXCLUDE_DIRS {
        let neg = format!("!{dir}/");
        builder
            .add(&neg)
            .map_err(|e| Error::Config(e.to_string()))?;
    }

    let overrides = builder.build().map_err(|e| Error::Config(e.to_string()))?;

    let mut files = Vec::new();
    let walker = WalkBuilder::new(base)
        .overrides(overrides)
        .hidden(true)
        .git_ignore(true)
        .build();

    for dir_entry in walker {
        let entry = dir_entry.map_err(|e| Error::Config(e.to_string()))?;
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            files.push(entry.into_path());
        }
    }

    Ok(files)
}

/// Result of an [`update`](Qmd::update) operation across collections.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct UpdateResult {
    /// Number of collections processed.
    pub collections: usize,
    /// Newly indexed documents.
    pub indexed: usize,
    /// Updated documents (content changed).
    pub updated: usize,
    /// Unchanged documents.
    pub unchanged: usize,
    /// Removed (deactivated) documents.
    pub removed: usize,
}

/// Result of indexing a single collection.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct IndexResult {
    /// Newly indexed documents.
    pub indexed: usize,
    /// Updated documents (content changed).
    pub updated: usize,
    /// Unchanged documents.
    pub unchanged: usize,
    /// Removed (deactivated) documents.
    pub removed: usize,
}

/// Directories excluded from indexing.
const EXCLUDE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".cache",
    "vendor",
    "dist",
    "build",
    "target",
];
