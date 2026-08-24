# Workstream A

Detail extracted from CLAUDE.md section 5 per the size discipline rule in
CLAUDE.md section 8.5. Read CLAUDE.md first, then this file. Sections 1-4 and
6-11 of CLAUDE.md apply to this workstream in full.

### WS-A: Data and embedding (Python)

Owns `pipeline/`, `contracts/fixtures/`. Runs on the M3 Pro.

**Phase A1: Corpus ingest**

1. `uv init --python 3.12` in `pipeline/`, add `torch`, `fair-esm`, `biopython`,
   `polars`, `pyarrow`, `numpy`, `httpx`, `tqdm`; dev: `pytest`, `ruff`, `mypy`.
2. Write `ingest.py` fetching reviewed Swiss-Prot from the UniProt REST API
   (`/uniprotkb/stream` with `reviewed:true`, TSV + FASTA). Stream to disk with
   resume; the download is ~100 MB compressed and will fail at least once.
3. Record the UniProt release string. It goes in `manifest.json` and it is the
   difference between a reproducible artifact and an anecdote.
4. Filter: drop sequences under 20 residues, drop any containing non-standard
   residues (B, J, O, U, X, Z), deduplicate identical sequences keeping the
   lowest accession. Log counts dropped per rule.
5. Parse EC numbers, GO terms, Pfam accessions, protein name, organism.
6. Write `meta.parquet` per `contracts/embeddings.md`.
7. Emit a dev subset of 10,000 proteins, stratified so EC class distribution
   matches the full corpus. Random 10k will over-represent uncharacterized
   proteins and make your dev-time quality checks lie to you.
8. Tests (written first, per CLAUDE.md 4.6): filter rules, dedup, EC parsing, subset stratification.

Done when: `meta.parquet` for the full corpus and the 10k subset both exist and
match the contract schema, with a printed summary of counts dropped per rule.

**Phase A2: ESM-2 embedding on MPS**

1. Load `esm2_t33_650M_UR50D`, `torch.float16`, `device="mps"`, `model.eval()`,
   inside `torch.inference_mode()`.
2. Implement length bucketing in `bucket.py`: sort by length, form batches with
   a token budget (start at 16,384 tokens per batch) rather than a fixed count.
   Attention is O(L^2); a fixed batch size will either OOM on long proteins or
   waste the GPU on short ones.
3. Truncate at 1022 residues, set `truncated=true` in metadata.
4. Implement `pool.py`: masked mean pooling over residue representations from
   the final layer, excluding BOS, EOS, and padding.
   **This is the single highest-risk function in the project.** Pooling over pad
   tokens silently produces plausible-looking but degraded embeddings, and you
   will not notice until recall numbers are inexplicably bad.
   **Write the test first.** CLAUDE.md 4.6 gives the exact worked example: two
   sequences of different real lengths, hand-computed expected means, chosen so
   that a pad leak changes the answer. Watch it fail with an assertion error
   against the unwritten function before you implement anything.
5. L2-normalize every embedding before writing.
6. Shard output: write every 25,000 proteins to `shard_%05d.npy` plus a
   completed-ids file. On restart, skip completed shards.
7. Write `scripts/bench_throughput.py`: embed 1,000 proteins, report
   proteins/sec, tokens/sec, peak memory, and the extrapolated wall time for
   571k. **Run this before planning anything else in this workstream.** Every
   scheduling assumption downstream depends on the real number.
8. Merge shards into `embeddings.npy` + `ids.json` + `manifest.json` with
   SHA-256 checksums.

Done when: 10k dev subset is fully embedded, `bench_throughput.py` output is
recorded in `docs/throughput.md`, and the merged artifact validates against the
contract (shape, dtype, unit norms within 1e-4).

**Phase A3: ONNX export for the query encoder** (unblocks C2)

1. Export ESM-2 650M to ONNX with dynamic axes for batch and sequence length.
2. Verify parity: for 100 held-out sequences, cosine similarity between the
   PyTorch embedding and the ONNX embedding must exceed 0.9999. Any lower and
   the export is wrong; do not proceed.
3. Quantize to int8 dynamic if p50 CPU latency exceeds 1.5 s per sequence, and
   re-verify parity at a 0.999 threshold. Record the tradeoff in `docs/`.
4. Publish `model.onnx` plus a `tokenizer.json` (the ESM alphabet as a plain
   vocab mapping) to S3, and document the exact URI in `contracts/`.
5. If ONNX export fights back (ESM-2 has export quirks around attention masks),
   fall back to a FastAPI sidecar in the task definition and open an issue.
   Do not lose a week to this. Note the fallback in `docs/adr/`.

Done when: WS-C can load the artifact in Rust via `ort` and reproduce a known
embedding to within 1e-3.

**Phase A4: Validation and golden fixtures** (unblocks B4)

1. Brute-force cosine top-k over the 10k dev subset in NumPy.
2. Quality gate: for the 200 sampled query proteins that have EC numbers, what
   fraction of top-10 neighbors share the query's EC class at each of the four
   levels? Write it to `docs/quality_dev10k.md`.
   **This is the go/no-go for the whole project.** If top-10 neighbors do not
   preferentially share EC class, something upstream is broken. In order of
   likelihood: pooling mask, missing L2 normalization, row misalignment between
   the npy and ids.json, wrong model checkpoint. Debug there before continuing.
3. Manually inspect 20 queries. Do the hits look like real homologs?
4. Write `contracts/fixtures/golden_neighbors.json` per the contract.
5. Ship a `validate_artifact.py` that any stream can run against any artifact
   directory to check contract conformance. Wire it into CI.

Done when: the quality gate passes and is written up, and the fixtures are
committed.

**Phase A5: Full corpus run and Batch containerization**

1. Run the full 571k embed locally. Expect roughly 20 hours at 8 proteins/sec;
   use your A2 measurement, not this estimate. Run it overnight across two
   nights, relying on shard resume. Disable App Nap and keep the lid open or
   use `caffeinate -i`.
2. In parallel, write `pipeline/Dockerfile.gpu` for CUDA so the same code runs
   on AWS Batch. Device selection is a flag, not a branch scattered through the
   code: `--device {mps,cuda,cpu}`.
3. Coordinate with WS-D on D4 to run the same job once on Batch GPU spot. The
   point is the pipeline works on both, and you have the resume line.
4. Upload the full artifact to S3, record checksums in the manifest.
5. Rerun the A4 quality gate on the full corpus.

Done when: full-corpus artifact is in S3, validated, and the same container has
run successfully on both MPS and Batch.

