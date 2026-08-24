# Workstream C

Detail extracted from CLAUDE.md section 5 per the size discipline rule in
CLAUDE.md section 8.5. Read CLAUDE.md first, then this file. Sections 1-4 and
6-11 of CLAUDE.md apply to this workstream in full.

### WS-C: Search service (Rust binary)

Owns `crates/esm2-search-server/`, `web/`.

**Phase C1: API skeleton with stubs** (unblocks web and eval)

1. axum app with every route in `contracts/openapi.yaml`: `/v1/health`,
   `POST /v1/search`, `GET /v1/proteins/{accession}`.
2. Return hardcoded fixture responses matching the schema exactly. No index yet.
3. RFC 9457 problem-details error type as an axum `IntoResponse`. Every error
   path in the service routes through this one type.
4. `tower-http` middleware: request id (accept `X-Request-Id` or generate a
   ULID), tracing, timeout, CORS for the web origin, body size limit.
5. Structured JSON logging via `tracing-subscriber`, request id on every line.
6. Config from environment with a typed struct and validation at startup. Fail
   fast on missing config; never default a required value silently.
7. Integration tests asserting responses validate against the OpenAPI schema.

Done when: `curl localhost:8080/v1/search` returns a schema-valid response.

**Phase C2: Query encoder** (needs A3)

1. Load `model.onnx` via the `ort` crate. Start with a fake encoder returning a
   fixed vector so this phase is not blocked on A3 landing.
2. Port the ESM alphabet tokenizer to Rust: BOS, EOS, the 20 standard residues,
   padding, unknown. Test it against the Python tokenizer on 100 sequences;
   token id sequences must match exactly.
3. Input validation: strip FASTA headers, uppercase, strip whitespace, reject
   non-standard residues with a 422 naming the offending character and its
   position, truncate over 1022 with `truncated=true`.
4. Masked mean pool and L2-normalize. **This must match `pipeline/pool.py`
   exactly.** Cross-language parity test: 100 sequences, cosine similarity
   between the Rust and Python embeddings above 0.9999, run in CI.
   Any divergence here corrupts every search result in a way that looks like
   mediocre quality rather than a bug.
5. Startup assertion: the encoder's model id equals the index's `model_id()`.
   Refuse to start on mismatch. This is the guard that makes a whole class of
   silent-garbage failures impossible.
6. Benchmark CPU encode latency. If p95 exceeds 2 s, escalate to WS-A for int8
   quantization.

Done when: a real sequence produces a real embedding matching Python, verified
in CI.

**Phase C3: Wire the index** (needs B1)

1. Depend on `esm2-search-index`. Load at startup into `Arc<dyn VectorIndex>` in
   axum state. Selectable brute force or HNSW by config.
2. Readiness: `/v1/health` returns 503 until the index is loaded, 200 after.
   ALB health checks depend on this being honest.
3. Map `Hit.row` to accession and metadata for the response.
4. Populate `took_ms` with real embed, search, and total timings.
5. Fetch artifacts from S3 at startup if a URI is configured, with a local cache.
6. Apply `min_score` and `k`, clamp `k` to 100.

Done when: an end-to-end query against the dev subset returns real ranked hits.

**Phase C4: Production hardening**

1. Multi-stage Dockerfile: `cargo chef` for dependency caching, distroless or
   `debian:bookworm-slim` runtime. Target under 150 MB.
2. Non-root user. Read-only root filesystem.
3. Graceful shutdown on SIGTERM, draining in-flight requests. ECS sends SIGTERM
   before it kills the task; without this you drop requests on every deploy.
4. Per-IP rate limiting via `tower-governor`. This endpoint runs an ML model per
   request; without a limit, one script makes your bill someone else's decision.
5. Prometheus metrics at `/metrics`: request count by status, latency histogram,
   encode and search latency separately, index size.
6. Load test with `oha` or `k6`. Record sustained RPS at acceptable p99 in
   `docs/`. This number sizes your Fargate task in D3.

Done when: the image runs locally with the real index, survives a load test, and
shuts down cleanly.

**Phase C5: Web UI**

1. Vite + React + TypeScript in `web/`. Generate the client from
   `contracts/openapi.yaml`; do not hand-write request types.
2. One page: a textarea for the sequence, a k selector, a results table with
   rank, accession linked to UniProt, score, name, organism, EC numbers.
3. Three example sequences as one-click buttons. Nobody visiting your demo has a
   protein sequence in their clipboard, and a demo that requires the visitor to
   supply input gets no engagement.
4. Loading, error, and empty states. Surface the problem-details `title` on
   error; never a bare "Something went wrong".
5. Show `took_ms` in the UI. It is a search engine; the speed is the point.
6. Deploy to Cloudflare Pages. Configure CORS on the API for that origin.
7. Accessibility: labeled inputs, keyboard submit, visible focus states.

Done when: a public URL takes a pasted sequence and returns ranked hits.

