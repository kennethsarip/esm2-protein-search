# Contract A->B: Embedding Artifact Format

FROZEN as of Phase 0. Changing this breaks WS-B and WS-D. Any change requires a
PR touching this file, reviewed by whoever owns WS-A and WS-B.

## Artifact set

A corpus build produces exactly four files, written to the same directory:

```
embeddings.npy      float32, shape (N, 1280), C-contiguous, L2-normalized
ids.json            {"ids": ["P00761", ...]}  len == N, row-aligned to .npy
meta.parquet        per-protein metadata, row-aligned to .npy
manifest.json       provenance + checksums
```

## embeddings.npy

- Format: NumPy .npy v1.0, no pickle.
- dtype: `float32` (little-endian). Not float16 on disk; quantization is WS-B's job.
- Shape: `(N, 1280)`. 1280 is the ESM-2 650M hidden size and is fixed for MVP.
- Row `i` corresponds to `ids.json.ids[i]` and row `i` of `meta.parquet`.
- Every row is L2-normalized to unit length. Consumers may therefore treat
  inner product and cosine similarity as equivalent. WS-B must assert this on
  load (tolerance 1e-4) and fail loudly if violated.

## ids.json

UniProt accessions, uppercase, no version suffix. Unique. Order is significant.

## meta.parquet

| column       | type          | null? | notes                                  |
|--------------|---------------|-------|----------------------------------------|
| accession    | string        | no    | matches ids.json                       |
| name         | string        | no    | UniProt protein name                   |
| organism     | string        | no    | scientific name                        |
| length       | int32         | no    | residues in the ORIGINAL sequence      |
| truncated    | bool          | no    | true if length > 1022 and was cut      |
| ec_numbers   | list<string>  | yes   | empty list if none, never null         |
| go_terms     | list<string>  | yes   | GO IDs, e.g. "GO:0004252"              |
| pfam         | list<string>  | yes   | Pfam accessions                        |

## manifest.json

```json
{
  "corpus": "swissprot",
  "corpus_release": "2026_03",
  "n_proteins": 571282,
  "model": "esm2_t33_650M_UR50D",
  "model_sha256": "<hex>",
  "embedding_dim": 1280,
  "pooling": "masked_mean",
  "max_residues": 1022,
  "dtype": "float32",
  "l2_normalized": true,
  "built_at": "2026-08-24T12:00:00Z",
  "builder_version": "0.1.0",
  "embeddings_sha256": "<hex>",
  "ids_sha256": "<hex>"
}
```

WS-B refuses to build an index if `embedding_dim` or `pooling` disagrees with
what the index was configured for. WS-C refuses to serve if the query encoder's
model id differs from `manifest.model`. This check is mandatory, not advisory:
mixing encoders silently produces plausible-looking garbage rankings.

## Dev subset

WS-A also publishes a 10k-row subset under the same schema with
`"corpus": "swissprot-dev10k"`. WS-B and WS-C develop against this. It is
committed to the repo via Git LFS if under 60 MB, otherwise fetched from S3.

## Golden fixtures

WS-A publishes `contracts/fixtures/golden_neighbors.json`:

```json
{
  "k": 10,
  "metric": "cosine",
  "corpus": "swissprot-dev10k",
  "queries": [
    {"id": "P00761", "neighbors": ["P00760", ...], "scores": [0.94, ...]}
  ]
}
```

Computed by exact brute force in NumPy. WS-B's recall tests measure against
this. 200 queries is enough.
