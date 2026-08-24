//! Integration test for `BruteForceIndex` against a synthetic corpus directory
//! shaped like `contracts/embeddings.md`. The real dev10k subset is WS-A's
//! deliverable and isn't built yet; per CLAUDE.md 4.6, B1 is not blocked on it.

#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // fixture sizes are tiny

use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use esm2_search_index::{BruteForceIndex, SearchParams, VectorIndex, EMBEDDING_DIM};
use parquet::data_type::{ByteArray, ByteArrayType};
use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::schema::parser::parse_message_type;
use tempfile::TempDir;

fn write_npy_f32_2d(path: &Path, n: usize, dim: usize, rows: &[Vec<f32>]) {
    let dict = format!("{{'descr': '<f4', 'fortran_order': False, 'shape': ({n}, {dim}), }}");
    let prefix_len = 6 + 2 + 2;
    let unpadded_len = dict.len() + 1;
    let pad = (64 - (prefix_len + unpadded_len) % 64) % 64;
    let mut dict_padded = dict.into_bytes();
    dict_padded.extend(std::iter::repeat_n(b' ', pad));
    dict_padded.push(b'\n');

    let mut out = Vec::new();
    out.extend_from_slice(b"\x93NUMPY\x01\x00");
    out.extend_from_slice(&(dict_padded.len() as u16).to_le_bytes());
    out.extend_from_slice(&dict_padded);
    for row in rows {
        for v in row {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    fs::File::create(path).unwrap().write_all(&out).unwrap();
}

fn write_ids_json(path: &Path, ids: &[String]) {
    let body = serde_json::json!({ "ids": ids });
    fs::write(path, serde_json::to_vec(&body).unwrap()).unwrap();
}

fn write_manifest_json(path: &Path, n: usize, dim: usize) {
    let body = serde_json::json!({
        "corpus": "swissprot-dev10k",
        "corpus_release": "2026_03",
        "n_proteins": n,
        "model": "esm2_t33_650M_UR50D",
        "model_sha256": "deadbeef",
        "embedding_dim": dim,
        "pooling": "masked_mean",
        "max_residues": 1022,
        "dtype": "float32",
        "l2_normalized": true,
        "built_at": "2026-08-24T12:00:00Z",
        "builder_version": "0.1.0",
        "embeddings_sha256": "deadbeef",
        "ids_sha256": "deadbeef",
    });
    fs::write(path, serde_json::to_vec(&body).unwrap()).unwrap();
}

fn write_meta_parquet(path: &Path, n: usize) {
    let message_type = "message schema { REQUIRED BYTE_ARRAY accession (UTF8); }";
    let schema = Arc::new(parse_message_type(message_type).unwrap());
    let props = Arc::new(WriterProperties::builder().build());
    let file = fs::File::create(path).unwrap();
    let mut writer = SerializedFileWriter::new(file, schema, props).unwrap();
    let mut row_group_writer = writer.next_row_group().unwrap();
    let mut col_writer = row_group_writer.next_column().unwrap().unwrap();
    let values: Vec<ByteArray> = (0..n)
        .map(|i| ByteArray::from(format!("P{i:05}").as_str()))
        .collect();
    col_writer
        .typed::<ByteArrayType>()
        .write_batch(&values, None, None)
        .unwrap();
    col_writer.close().unwrap();
    row_group_writer.close().unwrap();
    writer.close().unwrap();
}

/// A deterministic, distinct unit vector per row: a rotation in the first two
/// dimensions by a row-dependent angle, zero elsewhere. Distinct angles mean
/// distinct rows never collide on cosine similarity by construction.
fn synthetic_unit_vector(dim: usize, row: usize, n: usize) -> Vec<f32> {
    let angle = (row as f32) / (n as f32) * std::f32::consts::PI;
    let mut v = vec![0.0; dim];
    v[0] = angle.cos();
    v[1] = angle.sin();
    v
}

fn write_synthetic_corpus(dir: &Path, n: usize, dim: usize) -> Vec<String> {
    let rows: Vec<Vec<f32>> = (0..n)
        .map(|row| synthetic_unit_vector(dim, row, n))
        .collect();
    write_npy_f32_2d(&dir.join("embeddings.npy"), n, dim, &rows);
    let ids: Vec<String> = (0..n).map(|i| format!("P{i:05}")).collect();
    write_ids_json(&dir.join("ids.json"), &ids);
    write_manifest_json(&dir.join("manifest.json"), n, dim);
    write_meta_parquet(&dir.join("meta.parquet"), n);
    ids
}

#[test]
fn len_matches_corpus_row_count() {
    let tmp = TempDir::new().unwrap();
    write_synthetic_corpus(tmp.path(), 50, EMBEDDING_DIM);

    let index = BruteForceIndex::load(tmp.path()).expect("synthetic corpus should load");

    assert_eq!(index.len(), 50);
}

#[test]
fn self_query_returns_itself_at_rank_1_with_score_1() {
    let tmp = TempDir::new().unwrap();
    let n = 50;
    write_synthetic_corpus(tmp.path(), n, EMBEDDING_DIM);
    let index = BruteForceIndex::load(tmp.path()).expect("synthetic corpus should load");

    for query_row in [0usize, 17, 49] {
        let query = synthetic_unit_vector(EMBEDDING_DIM, query_row, n);
        let hits = index
            .search(
                &query,
                &SearchParams {
                    k: 3,
                    ef_search: None,
                },
            )
            .expect("search on a well-formed query must not fail");

        assert_eq!(hits[0].row, query_row as u32);
        assert!(
            (hits[0].score - 1.0).abs() < 1e-4,
            "expected self-similarity ~1.0, got {}",
            hits[0].score
        );
    }
}

#[test]
fn model_id_matches_manifest() {
    let tmp = TempDir::new().unwrap();
    write_synthetic_corpus(tmp.path(), 5, EMBEDDING_DIM);

    let index = BruteForceIndex::load(tmp.path()).expect("synthetic corpus should load");

    assert_eq!(index.model_id(), "esm2_t33_650M_UR50D");
}
