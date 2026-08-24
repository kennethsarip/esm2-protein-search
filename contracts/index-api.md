# Contract B->C: Index Crate Public API

FROZEN as of Phase 0. WS-C codes against this from day one. WS-B ships a
brute-force implementation in Phase B1 so WS-C is never blocked.

```rust
// crates/esm2-search-index/src/lib.rs

use std::path::Path;

pub const EMBEDDING_DIM: usize = 1280;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub row: u32,      // index into the corpus, maps to ids.json
    pub score: f32,    // cosine similarity in [-1, 1], higher is better
}

#[derive(Debug, Clone)]
pub struct SearchParams {
    pub k: usize,
    pub ef_search: Option<usize>,  // None = implementation default
}

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("dimension mismatch: index has {expected}, query has {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    #[error("query vector is not L2-normalized (norm = {norm})")]
    NotNormalized { norm: f32 },
    #[error("index artifact is corrupt or from an incompatible version: {0}")]
    Corrupt(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub trait VectorIndex: Send + Sync {
    /// Search for the k nearest neighbors of `query`.
    /// `query` MUST be L2-normalized and of length EMBEDDING_DIM.
    /// Returns hits sorted by descending score. May return fewer than k.
    fn search(&self, query: &[f32], params: &SearchParams)
        -> Result<Vec<Hit>, IndexError>;

    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }

    /// Model id the index was built from, checked against the query encoder.
    fn model_id(&self) -> &str;
}

/// Exact search. Correct by definition, O(N) per query. The recall oracle.
pub struct BruteForceIndex { /* ... */ }

/// Approximate search via HNSW.
pub struct HnswIndex { /* ... */ }

impl BruteForceIndex {
    pub fn load(dir: &Path) -> Result<Self, IndexError>;
}

impl HnswIndex {
    pub fn load(dir: &Path) -> Result<Self, IndexError>;
    pub fn build(dir: &Path, cfg: &BuildConfig) -> Result<Self, IndexError>;
    pub fn save(&self, path: &Path) -> Result<(), IndexError>;
}

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub m: usize,                 // default 32
    pub ef_construction: usize,   // default 200
    pub seed: u64,                // default 42, builds must be reproducible
}
```

Rules:

1. `search` never panics. All failure paths return `IndexError`.
2. `search` is `&self` and thread-safe. WS-C shares one index across handlers
   behind an `Arc` with no lock.
3. Loading is eager. Once `load` returns, no further disk I/O on the hot path.
4. Score is always cosine similarity, never a distance. Higher is better,
   always. Do not leak an internal L2 or dot-product convention to WS-C.
5. `BruteForceIndex` and `HnswIndex` must return identical results when
   `ef_search` is set high enough. This is a test, not an aspiration.
