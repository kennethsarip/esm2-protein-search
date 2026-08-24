//! Vector index for protein embedding similarity search.
//!
//! Public API is frozen in `contracts/index-api.md`. See CLAUDE.md phase B1.

#![warn(missing_docs)]

mod brute;
mod corpus;
mod npy;

use std::path::Path;

pub use brute::BruteForceIndex;

/// Dimensionality of ESM-2 650M embeddings.
pub const EMBEDDING_DIM: usize = 1280;

/// A single search result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// Index into the corpus, maps to `ids.json`.
    pub row: u32,
    /// Cosine similarity in `[-1, 1]`, higher is better.
    pub score: f32,
}

/// Parameters controlling a single search call.
#[derive(Debug, Clone)]
pub struct SearchParams {
    /// Number of neighbors to return.
    pub k: usize,
    /// Approximate-search effort. `None` means the implementation default.
    /// Ignored by [`BruteForceIndex`], which is always exact.
    pub ef_search: Option<usize>,
}

/// Errors returned by index construction, loading, and search.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// The query vector's length does not match the index's embedding dimension.
    #[error("dimension mismatch: index has {expected}, query has {actual}")]
    DimensionMismatch {
        /// Dimension the index was built with.
        expected: usize,
        /// Dimension of the provided query.
        actual: usize,
    },
    /// The query vector is not L2-normalized.
    #[error("query vector is not L2-normalized (norm = {norm})")]
    NotNormalized {
        /// The measured L2 norm of the offending vector.
        norm: f32,
    },
    /// The on-disk index artifact is corrupt or from an incompatible version.
    #[error("index artifact is corrupt or from an incompatible version: {0}")]
    Corrupt(String),
    /// An I/O error occurred while loading an artifact.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A queryable nearest-neighbor index over L2-normalized embedding vectors.
pub trait VectorIndex: Send + Sync {
    /// Search for the k nearest neighbors of `query`.
    ///
    /// `query` MUST be L2-normalized and of length [`EMBEDDING_DIM`].
    /// Returns hits sorted by descending score. May return fewer than k.
    ///
    /// # Errors
    ///
    /// Returns [`IndexError::DimensionMismatch`] if `query.len() != EMBEDDING_DIM`,
    /// or [`IndexError::NotNormalized`] if `query` is not unit length.
    fn search(&self, query: &[f32], params: &SearchParams) -> Result<Vec<Hit>, IndexError>;

    /// Number of vectors held by the index.
    fn len(&self) -> usize;

    /// True if the index holds no vectors.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Model id the index was built from, checked against the query encoder.
    fn model_id(&self) -> &str;
}

/// Approximate search via HNSW.
///
/// Stub for Phase B2; not yet implemented.
pub struct HnswIndex {
    _private: (),
}

/// Parameters controlling HNSW graph construction.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Max neighbors per node per layer. Default 32.
    pub m: usize,
    /// Candidate list size during construction. Default 200.
    pub ef_construction: usize,
    /// RNG seed. Builds with the same seed and input are byte-identical.
    pub seed: u64,
}

impl HnswIndex {
    /// Load a previously built and serialized HNSW index.
    ///
    /// # Errors
    ///
    /// Returns [`IndexError::Corrupt`] if the artifact is missing, malformed, or
    /// from an incompatible format version.
    pub fn load(_dir: &Path) -> Result<Self, IndexError> {
        todo!("Phase B2")
    }

    /// Build an HNSW index from the corpus artifacts in `dir`.
    ///
    /// # Errors
    ///
    /// Returns [`IndexError::Corrupt`] if the corpus artifacts in `dir` are
    /// missing or fail validation.
    pub fn build(_dir: &Path, _cfg: &BuildConfig) -> Result<Self, IndexError> {
        todo!("Phase B2")
    }

    /// Serialize this index to `path`.
    ///
    /// # Errors
    ///
    /// Returns [`IndexError::Io`] if `path` cannot be written.
    pub fn save(&self, _path: &Path) -> Result<(), IndexError> {
        todo!("Phase B2")
    }
}

impl VectorIndex for HnswIndex {
    fn search(&self, _query: &[f32], _params: &SearchParams) -> Result<Vec<Hit>, IndexError> {
        todo!("Phase B2")
    }

    fn len(&self) -> usize {
        todo!("Phase B2")
    }

    fn model_id(&self) -> &str {
        todo!("Phase B2")
    }
}
