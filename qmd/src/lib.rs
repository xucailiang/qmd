//! QMD — Query Markdown Documents.
//!
//! Local search engine for markdown files backed by SQLite FTS5 (BM25).
//!
//! # Quick start
//!
//! ```rust,no_run
//! use qmd::{Qmd, Collection};
//!
//! let mut qmd = Qmd::open("./index.sqlite")?;
//! qmd.register_collection(&Collection::new("docs", "/path/to/docs"))?;
//! qmd.update(None)?;
//!
//! // BM25 full-text search
//! let results = qmd.search("how does auth work?", 10)?;
//!
//! // Or just BM25
//! let fts_results = qmd.search_fts("rust ownership", 10)?;
//! # Ok::<(), qmd::Error>(())
//! ```

pub mod db;
pub mod error;
pub mod qmd;
pub mod search;

pub use db::{
    Collection, CollectionInfo, Document, IndexStatus, SearchResult, SearchSource, hash_content,
};
pub use error::{Error, Result};
pub use qmd::{IndexResult, Qmd, UpdateResult};
