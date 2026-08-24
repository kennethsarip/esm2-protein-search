//! Loads and validates the four-artifact corpus format from `contracts/embeddings.md`:
//! `embeddings.npy`, `ids.json`, `meta.parquet`, `manifest.json`.

use std::fs;
use std::path::Path;

use parquet::file::reader::{FileReader, SerializedFileReader};
use serde::Deserialize;

use crate::npy::parse_npy_f32_2d;
use crate::{IndexError, EMBEDDING_DIM};

const NORM_TOLERANCE: f32 = 1e-4;

#[derive(Debug)]
pub struct CorpusArtifacts {
    pub ids: Vec<String>,
    pub vectors: Vec<f32>,
    pub dim: usize,
    pub model_id: String,
}

#[derive(Deserialize)]
struct IdsFile {
    ids: Vec<String>,
}

#[derive(Deserialize)]
struct Manifest {
    model: String,
    n_proteins: usize,
    embedding_dim: usize,
    pooling: String,
    l2_normalized: bool,
}

pub fn load(dir: &Path) -> Result<CorpusArtifacts, IndexError> {
    let array = parse_npy_f32_2d(&fs::read(dir.join("embeddings.npy"))?)?;
    let (n, dim) = array.shape;
    let ids_file: IdsFile = read_json(&dir.join("ids.json"))?;
    let manifest: Manifest = read_json(&dir.join("manifest.json"))?;
    let meta_rows = read_parquet_row_count(&dir.join("meta.parquet"))?;

    validate_manifest(dim, &manifest)?;
    validate_row_counts(n, ids_file.ids.len(), meta_rows, manifest.n_proteins)?;
    validate_unit_norms(&array.data, dim)?;

    Ok(CorpusArtifacts {
        ids: ids_file.ids,
        vectors: array.data,
        dim,
        model_id: manifest.model,
    })
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, IndexError> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes)
        .map_err(|e| IndexError::Corrupt(format!("{}: {e}", path.display())))
}

fn read_parquet_row_count(path: &Path) -> Result<usize, IndexError> {
    let file = fs::File::open(path)?;
    let reader = SerializedFileReader::new(file)
        .map_err(|e| IndexError::Corrupt(format!("{}: {e}", path.display())))?;
    usize::try_from(reader.metadata().file_metadata().num_rows())
        .map_err(|_| IndexError::Corrupt("meta.parquet reports a negative row count".into()))
}

fn validate_manifest(dim: usize, manifest: &Manifest) -> Result<(), IndexError> {
    if dim != EMBEDDING_DIM {
        return Err(IndexError::Corrupt(format!(
            "embeddings.npy has dim {dim}, expected {EMBEDDING_DIM}"
        )));
    }
    if manifest.embedding_dim != EMBEDDING_DIM {
        return Err(IndexError::Corrupt(format!(
            "manifest embedding_dim {} disagrees with the compiled-in {EMBEDDING_DIM}",
            manifest.embedding_dim
        )));
    }
    if manifest.pooling != "masked_mean" {
        return Err(IndexError::Corrupt(format!(
            "unsupported pooling strategy in manifest: {}",
            manifest.pooling
        )));
    }
    if !manifest.l2_normalized {
        return Err(IndexError::Corrupt(
            "manifest declares l2_normalized = false".into(),
        ));
    }
    Ok(())
}

fn validate_row_counts(
    n: usize,
    ids_len: usize,
    meta_rows: usize,
    manifest_n: usize,
) -> Result<(), IndexError> {
    if [ids_len, meta_rows, manifest_n].iter().any(|&c| c != n) {
        return Err(IndexError::Corrupt(format!(
            "row-count mismatch across corpus artifacts: embeddings={n}, ids={ids_len}, meta={meta_rows}, manifest={manifest_n}"
        )));
    }
    Ok(())
}

fn validate_unit_norms(data: &[f32], dim: usize) -> Result<(), IndexError> {
    for (row, vector) in data.chunks_exact(dim).enumerate() {
        let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if (norm - 1.0).abs() > NORM_TOLERANCE {
            return Err(IndexError::Corrupt(format!(
                "row {row} is not unit-normalized: norm = {norm}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)] // hand-built byte-literal fixtures, sizes are tiny
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Arc;

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

    fn write_ids_json(path: &Path, ids: &[&str]) {
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

    /// A unit vector in `dim` dimensions with all mass on axis `axis`.
    fn unit_vector(dim: usize, axis: usize) -> Vec<f32> {
        let mut v = vec![0.0; dim];
        v[axis] = 1.0;
        v
    }

    fn write_valid_corpus(dir: &Path, n: usize, dim: usize) {
        let rows: Vec<Vec<f32>> = (0..n).map(|i| unit_vector(dim, i % dim)).collect();
        write_npy_f32_2d(&dir.join("embeddings.npy"), n, dim, &rows);
        let ids: Vec<String> = (0..n).map(|i| format!("P{i:05}")).collect();
        let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        write_ids_json(&dir.join("ids.json"), &id_refs);
        write_manifest_json(&dir.join("manifest.json"), n, dim);
        write_meta_parquet(&dir.join("meta.parquet"), n);
    }

    #[test]
    fn loads_a_valid_corpus() {
        let tmp = TempDir::new().unwrap();
        write_valid_corpus(tmp.path(), 3, EMBEDDING_DIM);

        let corpus = load(tmp.path()).expect("valid corpus should load");

        assert_eq!(corpus.ids, vec!["P00000", "P00001", "P00002"]);
        assert_eq!(corpus.dim, EMBEDDING_DIM);
        assert_eq!(corpus.model_id, "esm2_t33_650M_UR50D");
        assert_eq!(
            &corpus.vectors[0..EMBEDDING_DIM],
            unit_vector(EMBEDDING_DIM, 0).as_slice()
        );
    }

    #[test]
    fn rejects_ids_row_count_mismatch() {
        let tmp = TempDir::new().unwrap();
        write_valid_corpus(tmp.path(), 3, EMBEDDING_DIM);
        // Overwrite ids.json with only 2 ids, disagreeing with the 3-row npy.
        write_ids_json(&tmp.path().join("ids.json"), &["P00000", "P00001"]);

        let err = load(tmp.path()).expect_err("row-count mismatch must be rejected");
        assert!(matches!(err, IndexError::Corrupt(_)));
    }

    #[test]
    fn rejects_non_unit_norm_vectors() {
        let tmp = TempDir::new().unwrap();
        let dim = EMBEDDING_DIM;
        let mut rows: Vec<Vec<f32>> = (0..3).map(|i| unit_vector(dim, i % dim)).collect();
        rows[1][0] = 5.0; // norm now sqrt(1 + 25) != 1, well outside 1e-4 tolerance
        write_npy_f32_2d(&tmp.path().join("embeddings.npy"), 3, dim, &rows);
        let ids: Vec<String> = (0..3).map(|i| format!("P{i:05}")).collect();
        let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        write_ids_json(&tmp.path().join("ids.json"), &id_refs);
        write_manifest_json(&tmp.path().join("manifest.json"), 3, dim);
        write_meta_parquet(&tmp.path().join("meta.parquet"), 3);

        let err = load(tmp.path()).expect_err("non-unit-norm row must be rejected");
        assert!(matches!(err, IndexError::Corrupt(_)));
    }

    #[test]
    fn rejects_embedding_dim_mismatch_in_manifest() {
        let tmp = TempDir::new().unwrap();
        let dim = EMBEDDING_DIM;
        write_valid_corpus(tmp.path(), 3, dim);
        // Manifest claims a different embedding_dim than the crate is compiled for.
        write_manifest_json(&tmp.path().join("manifest.json"), 3, dim - 1);

        let err = load(tmp.path()).expect_err("embedding_dim disagreement must be rejected");
        assert!(matches!(err, IndexError::Corrupt(_)));
    }
}
