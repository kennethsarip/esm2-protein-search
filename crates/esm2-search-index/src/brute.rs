//! Exact brute-force search: the recall oracle everything else is measured against.

use std::path::Path;

use rayon::prelude::*;

use crate::corpus::{self, CorpusArtifacts};
use crate::{Hit, IndexError, SearchParams, VectorIndex};

/// Exact search. Correct by definition, O(N) per query.
pub struct BruteForceIndex {
    corpus: CorpusArtifacts,
}

impl BruteForceIndex {
    /// Load corpus artifacts from `dir` per `contracts/embeddings.md`.
    ///
    /// # Errors
    ///
    /// Returns [`IndexError::Corrupt`] if the artifacts are missing, disagree
    /// on row count, or contain non-unit-normalized vectors.
    pub fn load(dir: &Path) -> Result<Self, IndexError> {
        let corpus = corpus::load(dir)?;
        Ok(Self { corpus })
    }
}

/// Dot product of two equal-length, unit-normalized vectors, i.e. cosine similarity.
fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Exact top-k by score, descending. Ties broken by ascending row index for determinism.
fn top_k(query: &[f32], vectors: &[f32], dim: usize, k: usize) -> Vec<Hit> {
    let mut hits: Vec<Hit> = vectors
        .par_chunks_exact(dim)
        .enumerate()
        .map(|(row, vector)| Hit {
            // Corpus row counts are bounded by u32::MAX (Swiss-Prot is ~571k
            // rows); `Hit::row` is u32 per the frozen contract in
            // contracts/index-api.md.
            #[allow(clippy::cast_possible_truncation)]
            row: row as u32,
            score: dot(query, vector),
        })
        .collect();
    hits.sort_unstable_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap()
            .then(a.row.cmp(&b.row))
    });
    hits.truncate(k);
    hits
}

impl VectorIndex for BruteForceIndex {
    fn search(&self, query: &[f32], params: &SearchParams) -> Result<Vec<Hit>, IndexError> {
        if query.len() != self.corpus.dim {
            return Err(IndexError::DimensionMismatch {
                expected: self.corpus.dim,
                actual: query.len(),
            });
        }
        let norm = query.iter().map(|v| v * v).sum::<f32>().sqrt();
        if (norm - 1.0).abs() > 1e-3 {
            return Err(IndexError::NotNormalized { norm });
        }
        Ok(top_k(
            query,
            &self.corpus.vectors,
            self.corpus.dim,
            params.k,
        ))
    }

    fn len(&self) -> usize {
        self.corpus.ids.len()
    }

    fn model_id(&self) -> &str {
        &self.corpus.model_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(values: &[f32]) -> Vec<f32> {
        values.to_vec()
    }

    #[test]
    fn top_k_orders_by_descending_cosine_similarity() {
        // 4 vectors in R^2, already unit-normalized.
        // row 0: [1, 0], row 1: [0, 1], row 2: [-1, 0], row 3: [0.6, 0.8]
        // query = [1, 0] (row 0 itself).
        // Hand-computed dot products (= cosine sim, since all unit norm):
        //   row 0: 1*1 + 0*0 = 1.0
        //   row 1: 1*0 + 0*1 = 0.0
        //   row 2: 1*-1 + 0*0 = -1.0
        //   row 3: 1*0.6 + 0*0.8 = 0.6
        // Expected descending order: row 0 (1.0), row 3 (0.6), row 1 (0.0), row 2 (-1.0)
        let vectors: Vec<f32> = [
            v(&[1.0, 0.0]),
            v(&[0.0, 1.0]),
            v(&[-1.0, 0.0]),
            v(&[0.6, 0.8]),
        ]
        .concat();
        let query = [1.0, 0.0];

        let hits = top_k(&query, &vectors, 2, 4);

        assert_eq!(
            hits,
            vec![
                Hit { row: 0, score: 1.0 },
                Hit { row: 3, score: 0.6 },
                Hit { row: 1, score: 0.0 },
                Hit {
                    row: 2,
                    score: -1.0
                },
            ]
        );
    }

    #[test]
    fn top_k_truncates_to_k() {
        let vectors: Vec<f32> = [v(&[1.0, 0.0]), v(&[0.0, 1.0]), v(&[-1.0, 0.0])].concat();
        let query = [1.0, 0.0];

        let hits = top_k(&query, &vectors, 2, 1);

        assert_eq!(hits, vec![Hit { row: 0, score: 1.0 }]);
    }

    fn tiny_index() -> BruteForceIndex {
        use crate::corpus::CorpusArtifacts;
        BruteForceIndex {
            corpus: CorpusArtifacts {
                ids: vec!["P1".into(), "P2".into()],
                vectors: [v(&[1.0, 0.0]), v(&[0.0, 1.0])].concat(),
                dim: 2,
                model_id: "test-model".into(),
            },
        }
    }

    #[test]
    fn search_rejects_wrong_dimension_query() {
        let index = tiny_index();
        let params = SearchParams {
            k: 1,
            ef_search: None,
        };

        let err = index
            .search(&[1.0, 0.0, 0.0], &params)
            .expect_err("3-dim query against a 2-dim index must be rejected");

        assert!(matches!(
            err,
            IndexError::DimensionMismatch {
                expected: 2,
                actual: 3
            }
        ));
    }

    #[test]
    fn search_rejects_non_normalized_query() {
        let index = tiny_index();
        let params = SearchParams {
            k: 1,
            ef_search: None,
        };

        let err = index
            .search(&[3.0, 4.0], &params)
            .expect_err("norm-5 query must be rejected as not normalized");

        assert!(matches!(err, IndexError::NotNormalized { .. }));
    }
}
