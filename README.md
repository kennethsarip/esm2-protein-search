# esm2-protein-search

Remote-homology protein search over Swiss-Prot using ESM-2 embeddings, served
from a Rust HNSW index on AWS.

Status: pre-MVP. See `CLAUDE.md` for the build plan.

## The idea

Alignment-based search (BLAST, MMseqs2) misses remote homologs: proteins with
shared function whose sequences have diverged past detection. Protein language
model embeddings place those proteins near each other in vector space.

esm2-protein-search embeds ~570k curated Swiss-Prot proteins with ESM-2 650M, indexes the
1280-dimensional vectors, and serves nearest-neighbor search over HTTP.

## The claim we intend to make

Recall on SCOPe remote-homology pairs versus BLAST and MMseqs2, at measured
latency. Numbers land in `docs/benchmark.md` when Phase D7 completes. Until
then this README makes no performance claims.

## Layout

| Path | Contents |
|---|---|
| `contracts/` | Frozen interfaces between workstreams |
| `pipeline/` | Python: UniProt ingest, ESM-2 embedding |
| `crates/esm2-search-index/` | Rust: vector index |
| `crates/esm2-search-server/` | Rust: HTTP service |
| `infra/` | Terraform: S3, ECR, ECS Fargate, Batch |
| `eval/` | Benchmark harness |
| `web/` | Demo UI |

## Development

Requires Rust 1.92+, Python 3.12 (not 3.14), uv, Terraform, Docker.

```bash
cargo test --workspace
cd pipeline && uv run pytest
```

## License

MIT
