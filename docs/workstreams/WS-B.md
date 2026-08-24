# Workstream B

Detail extracted from CLAUDE.md section 5 per the size discipline rule in
CLAUDE.md section 8.5. Read CLAUDE.md first, then this file. Sections 1-4 and
6-11 of CLAUDE.md apply to this workstream in full.

### WS-B: Index core (Rust library)

Owns `crates/esm2-search-index/`. Publishable crate, zero HTTP, zero AWS.

**Phase B1: Crate skeleton and brute-force index** (critical path, do first)

1. Cargo workspace member, the exact public API from `contracts/index-api.md`.
   Types and signatures first, `todo!()` bodies, so it compiles immediately.
2. `.npy` reader for float32 2-D C-contiguous arrays. Parse the header, validate
   dtype, shape, and fortran_order. Reject anything else with `Corrupt`.
   Do not pull in a heavyweight dependency for this; the format is 60 lines.
3. Load `ids.json`, `meta.parquet` (via `arrow`/`parquet` crates), and
   `manifest.json`. Assert row-count agreement across all four. Assert unit
   norms within 1e-4. Fail loudly on mismatch.
4. `BruteForceIndex`: full scan, dot product, top-k via a bounded binary heap.
   Parallelize across rows with `rayon`.
5. Publish it. **This is what unblocks WS-C**, so land it before anything else.
6. Tests (written first, per CLAUDE.md 4.6): load the dev subset, assert `len()`, assert self-query returns the
   query itself at rank 1 with score 1.0.

Done when: WS-C can add the crate as a dependency and get real search results.

**Phase B2: HNSW**

1. Decide: implement HNSW yourself, or wrap an existing crate (`hnsw_rs`,
   `instant-distance`). Implementing it is the stronger portfolio signal and is
   a well-specified algorithm; wrapping is faster. Recommendation: implement it.
   The graph construction is the interesting part and it is what an interviewer
   will actually ask you about. Write an ADR recording the choice.
2. Graph construction: layer assignment by exponential decay, greedy search per
   layer, the neighbor heuristic from the paper (not naive top-M, which
   produces poorly connected graphs).
3. Seeded RNG. Same seed and same input means byte-identical index.
4. Serialization: a versioned binary format with a magic number, format version,
   and the model id from the manifest. Refuse to load a mismatched version.
5. Memory-map on load so startup does not read the whole graph.
6. Tests (written first, per CLAUDE.md 4.6): build on synthetic clustered vectors with known ground truth, assert
   recall@10 above 0.95 at `ef_search=100`. Synthetic data means you are not
   blocked on A4.

Done when: HNSW builds on the dev subset and agrees with brute force at high
`ef_search`.

**Phase B3: Quantization**

1. Scalar quantization first: float32 to int8 per-dimension, with stored scale
   and offset. 4x memory reduction. Measure recall loss on the fixtures.
2. Only if scalar quantization costs more than 2 points of recall@10, implement
   product quantization. Do not start with PQ; it is more code and more risk for
   a corpus this size.
3. 571k x 1280 x 4 bytes is roughly 2.9 GB unquantized, 730 MB at int8. The
   int8 version fits comfortably in a 2 GB Fargate task, which is the actual
   goal here and directly determines your hosting cost.
4. Benchmark: memory footprint, build time, and recall at each setting.

Done when: a documented memory-versus-recall table exists and the int8 index
loads in under 2 GB RSS.

**Phase B4: Recall and latency benchmarks** (needs A4)

1. Criterion benches for p50, p95, p99 single-query latency at `ef_search` in
   {32, 64, 100, 200, 400}.
2. Recall@{1,10,100} against `contracts/fixtures/golden_neighbors.json`.
3. Produce the recall-versus-latency curve. This plot is the single most
   valuable artifact this workstream generates. It goes in the README.
4. Pick and document the default `ef_search` from that curve.
5. Concurrency: throughput at 1, 4, 8, 16 concurrent queries. Confirm the
   `Arc`-shared index does not serialize.

Done when: the curve is committed to `docs/` and a default is chosen with a
stated justification.

**Phase B5: Crate polish**

1. Full rustdoc on every public item, with runnable examples.
2. README with a usage example.
3. `cargo publish --dry-run` clean.
4. Optional but recommended: actually publish to crates.io. A published crate is
   a materially stronger resume artifact than a directory in a monorepo.

