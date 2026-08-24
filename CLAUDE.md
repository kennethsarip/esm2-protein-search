# esm2-protein-search

Remote-homology protein search over Swiss-Prot using ESM-2 embeddings, served
from a Rust ANN index on AWS.

**Starting a session:** read this file, then `docs/workstreams/WS-<X>.md` for the
workstream you own, then the "Shared" and your own section of `MEMORY.md`. That
is the full context. Do not read the other workstreams' files.

**Finishing a phase:** log it in `MEMORY.md` (section 9), then run the
consistency check in section 8.4 and fix anything this document now gets wrong.
This document is maintained continuously, not at the end.

## 1. What this project is

BLAST and other alignment-based tools find proteins with similar *sequences*.
They miss remote homologs: proteins that do the same job but whose sequences
have diverged past the point where alignment detects them. Protein language
models encode function and structure into a vector, so two remote homologs land
near each other in embedding space even when their sequences barely align.

esm2-protein-search builds that search:

1. Embed all of Swiss-Prot (~570k curated proteins) with ESM-2 650M.
2. Index the 1280-dim vectors in a Rust HNSW index that answers in under 100 ms.
3. Serve it as an HTTP API on ECS Fargate, with a minimal web UI.
4. **Benchmark recall against BLAST and MMseqs2 on SCOPe remote-homology pairs.**

Step 4 is not optional and not a stretch goal. A protein search engine with no
recall numbers is a demo. One with a published recall-vs-latency curve against
the incumbent tools is a contribution. Every phase below is in service of being
able to make a defensible quantitative claim at the end.

### Prior art, and where our contribution sits

Know these before writing code. Do not reinvent them, and do not claim novelty
that belongs to them.

| Work | What it does | How we differ |
|---|---|---|
| ESM Metagenomic Atlas | Massive-scale ESM embeddings and structures | We are Swiss-Prot scale with curated function labels and an open recall benchmark |
| PLMSearch | Protein language model similarity search, published method | We are an engineering artifact: a deployed, benchmarked, low-latency service |
| Foldseek | Extremely fast *structure* search | We search sequence embeddings; no structure required at query time |
| MMseqs2 | Fast sequence search, the practical incumbent | It is our baseline, not our competitor. We must beat it on remote homology recall or report honestly that we did not |
| faiss / hnswlib | Mature ANN libraries | We build in Rust for a single-binary, no-Python-runtime service |

Our defensible claims, if the numbers support them, are: (a) measured recall
gain over MMseqs2 on remote-homology pairs at matched or better latency,
(b) a reproducible end-to-end pipeline anyone can rerun, (c) a single-binary
Rust service with no Python at serving time.

If the benchmark shows we lose to MMseqs2, we report that. An honest negative
result with a clean methodology is a far better artifact than a fudged win.

## 2. Repository structure

Monorepo. Each workstream owns directories exclusively. Do not edit outside
your workstream's owned paths without a note in the PR description.

```
.
├── CLAUDE.md                    shared, all agents read this. See section 8.
├── MEMORY.md                    shared progress log, append-only. See section 9.
├── contracts/                   FROZEN interfaces, changes need cross-WS review
│   ├── embeddings.md            WS-A -> WS-B artifact format
│   ├── index-api.md             WS-B -> WS-C Rust trait
│   ├── openapi.yaml             WS-C -> web + eval HTTP contract
│   └── fixtures/                golden neighbor sets for recall tests
├── pipeline/                    [WS-A] Python: ingest + embed
│   ├── pyproject.toml
│   ├── src/esm2_search_pipeline/
│   │   ├── ingest.py            UniProt fetch, parse, filter
│   │   ├── embed.py             ESM-2 on MPS/CUDA
│   │   ├── pool.py              masked mean pooling
│   │   ├── bucket.py            length bucketing
│   │   └── manifest.py          provenance + checksums
│   └── tests/
├── crates/
│   ├── esm2-search-index/          [WS-B] Rust lib: HNSW, quantization
│   │   ├── src/
│   │   ├── tests/               recall vs brute force
│   │   └── benches/             criterion latency benches
│   └── esm2-search-server/         [WS-C] Rust bin: axum API
│       ├── src/
│       └── tests/               integration tests against the OpenAPI spec
├── infra/                       [WS-D] Terraform
│   ├── modules/{network,storage,service,batch}/
│   └── envs/dev/
├── eval/                        [WS-D] benchmark harness
│   ├── src/                     SCOPe loader, BLAST/MMseqs2 runners, scoring
│   └── data/                    downloaded, gitignored
├── web/                         [WS-C phase C5] Vite + React demo UI
├── data/                        gitignored, local artifacts only
│   ├── raw/ interim/ embeddings/ index/
├── scripts/                     shared dev scripts, any WS may add
├── docs/
│   ├── workstreams/WS-{A,B,C,D}.md   per-phase task detail, read yours only
│   ├── adr/                     architecture decision records
│   └── benchmark.md             the D7 deliverable
└── .github/workflows/           [WS-D] CI
```

Nothing in `data/` is ever committed. The dev 10k subset is the one exception
and lives in `contracts/fixtures/` if it fits, otherwise in S3.

## 3. Languages and tools

| Layer | Choice | Why |
|---|---|---|
| Ingest + embedding | Python 3.12 | PyTorch and the bio ecosystem live here |
| Python env | uv | fast, lockfile-based, pins the interpreter |
| Model | ESM-2 `esm2_t33_650M_UR50D` | 1280-dim, the standard embedding workhorse |
| Local accel | PyTorch MPS, fp16 | Apple M3 Pro, 36 GB unified memory |
| Index + service | Rust 1.92 | single binary, no GIL, no Python at serve time |
| HTTP | axum + tokio | mainstream, good tracing integration |
| Query encoder in Rust | ONNX Runtime via `ort` | avoids a Python sidecar in prod |
| Index storage | memory-mapped, custom serde | full control over layout and quantization |
| IaC | Terraform | portable, the transferable skill |
| Compute | ECS Fargate, AWS Batch (GPU spot) | long-lived service; one-shot embed job |
| Storage | S3 | artifacts, Terraform remote state |
| Registry | ECR | Rust service image, embed job image |
| Frontend | Vite + React on Cloudflare Pages | you already know this stack; keep it cheap |
| CI | GitHub Actions | |
| Baselines | BLAST+, MMseqs2 | benchmark comparators |

Hard constraint: **Python 3.12, not 3.14.** Your system Python is 3.14, which is
ahead of stable PyTorch wheel support and MPS support lags further. Pin it:

```bash
uv init --python 3.12
```

Do not spend time fighting this. If `torch` fails to install, the interpreter
version is the first thing to check.

## 4. Engineering standards

### 4.1 Output and documentation style

- **No emojis anywhere.** Not in code, commit messages, PR descriptions, log
  output, CLI output, docs, or agent replies. They cost tokens and add nothing.
- No decorative ASCII art, banners, or box-drawing in program output.
- Prose over bullet soup in design docs. Bullets for genuine lists only.
- Write the number, not the adjective. "recall@10 = 0.71" beats "great recall".
- Comments explain *why*, never *what*. Delete any comment that restates code.
- No "Generated by" or "Co-authored-by" trailers unless the user asks for them.

### 4.2 Code size limits

Enforced in review, and by lint where the tooling allows.

| Unit | Soft limit | Hard limit |
|---|---|---|
| Function / method | 40 lines | 60 lines |
| File | 300 lines | 500 lines |
| Function parameters | 4 | 6, else pass a struct |
| Nesting depth | 3 | 4 |
| Cyclomatic complexity | 10 | 15 |

Over the hard limit, split it. A 200-line function is not "cohesive", it is
unreviewed and untestable. The one legitimate exception is a flat match or
dispatch table with no branching logic; note it in the PR if so.

### 4.3 API standards

- REST over HTTP/1.1, JSON only. Version prefix `/v1` on every route.
- JSON keys are `snake_case`. Always. Both directions.
- No trailing slashes. `/v1/search`, never `/v1/search/`.
- Every error response is RFC 9457 `application/problem+json`. No bare strings,
  no `{"error": "..."}`. See `contracts/openapi.yaml` for the shape.
- Every request gets an `X-Request-Id` (accept the client's, else generate a
  ULID) and it is echoed in the response and in every log line for that request.
- Status codes: 400 malformed input, 404 unknown accession, 413 payload too
  large, 422 semantically invalid sequence, 429 rate limited, 503 index not
  loaded. Never 200 with an error body.
- Every endpoint has a timeout. Default 10 s, search 5 s.
- Timings are milliseconds as numbers, in a `took_ms` object. Never strings.
- Breaking changes to `contracts/openapi.yaml` require a version bump and a PR
  that touches both the server and every consumer in the same commit.

### 4.4 Rust standards

- `#![deny(warnings)]` in CI, `clippy::pedantic` on, individually justified
  `allow`s only.
- No `unwrap()` or `expect()` in library code or request handlers. Permitted in
  tests, benches, and `main` startup where a panic is the correct response.
- `thiserror` for library error enums, `anyhow` for binaries. Never `Box<dyn Error>`.
- Public items in `esm2-search-index` need doc comments. `#![warn(missing_docs)]`.
- No `unsafe` without a `// SAFETY:` comment stating the invariant. For MVP,
  prefer no `unsafe` at all.
- `rustfmt` default config, no custom settings.
- Fallible operations return `Result`. Do not encode failure as a sentinel value.

### 4.5 Python standards

- `ruff` for lint and format, `mypy` in non-strict mode with explicit signatures
  on every public function.
- Type hints on all function signatures. `from __future__ import annotations`.
- No bare `except:`. Catch the specific exception.
- No mutable default arguments.
- `pathlib.Path`, never string paths or `os.path`.
- Long-running loops print progress to stderr at most once per 5 seconds. Do not
  emit a line per protein; 570k log lines helps nobody.
- Any script that processes the full corpus must be resumable. Write shards,
  checkpoint, and skip completed work on restart. A 20-hour job that cannot
  resume will be lost to a closed laptop lid.

### 4.6 Test-driven development

Mandatory for all deterministic code in this project. The cycle is:

1. **RED.** Write one test for one behavior that does not exist yet. Run it.
   **Watch it fail, and confirm it fails for the right reason.**
2. **GREEN.** Write the least code that makes it pass. Not the elegant version,
   not the general version. The least.
3. **REFACTOR.** Clean it up with the test still passing. Now apply the size
   limits in 4.2.

Repeat. Small cycles. If a cycle takes more than about twenty minutes, the
behavior under test was too large; split it.

**The step everyone skips is confirming *why* the test failed.** A test that
fails with `ImportError`, `NameError`, a typo in a fixture path, or a Rust
compile error has demonstrated nothing except that you have not written the
function yet. You already knew that. The failure must be an assertion failure
showing the expected value against the actual one. Until you have seen that
specific failure, you do not know the test can detect the bug it exists to
catch, and a test that cannot fail is worse than no test: it is a false
assurance that survives into the benchmark.

Rules:

- One behavior per cycle. Not one function, not one file. One behavior.
- **Assert the value, not the shape.** `assert result.shape == (2, 1280)` passes
  against an array of zeros. `assert_allclose(result, expected)` against
  hand-computed numbers does not. Shape assertions are a supplement, never the
  substance.
- Never write production code with no failing test demanding it. If you cannot
  think of a test that would fail without the code, you do not need the code.
- **Never weaken a test to make it pass.** Loosening a tolerance, deleting an
  assertion, or adding a special case for the failing input inverts the entire
  point of the practice. If a test is genuinely wrong, fix it in its own commit
  with the reasoning stated, so the change is visible rather than buried.
- Every bug fix starts with a failing test reproducing the bug. No exceptions.
  This is the one case where TDD is non-negotiable even for code you would
  otherwise have exempted.
- Proving it happened: paste the RED output into the PR description, or commit
  the failing test separately before the implementation. In agent sessions,
  quote the assertion failure. "I wrote tests" after the fact is not TDD, and
  the difference is not cosmetic: a test written after the implementation tends
  to assert what the code does rather than what it should do.

#### Where TDD applies, and where it does not

Applying it everywhere is as unserious as applying it nowhere. This project has
a lot of deterministic transformation code and some genuinely exploratory
numerical work, and they need different treatment.

| Kind of code | Approach |
|---|---|
| Pure deterministic functions: pooling, tokenizing, `.npy` parsing, filter rules, top-k selection, metric computation, sequence validation | **Strict TDD.** Hand-compute expected values first. |
| HTTP handlers, error mapping, config validation | **Strict TDD** against `contracts/openapi.yaml`. |
| ANN index behavior | TDD the invariants: self-query returns self at rank 1, brute force and HNSW agree at high `ef_search`. Recall thresholds come later, from A4 fixtures. |
| Model output quality | Not TDD-able; you cannot hand-compute an ESM-2 embedding. Use **characterization tests**: verify a small output by an independent route once, pin it as a golden fixture, then assert against it forever. That is what the A3 parity check and A4 fixtures are. |
| Throughput and latency | Benchmarks, not tests. Record numbers in MEMORY.md; do not assert on wall-clock time in CI, it will flake. |
| Terraform | `validate`, `tflint`, and `plan` review. Do not attempt TDD here. |
| Web UI | Test the API client and data transforms. Do not chase coverage on layout. |

#### The first RED test in each workstream

Concretely, before any implementation:

- **WS-A**, `pool.py`. This is risk R1, the highest-impact silent failure in the
  project, and TDD is its mitigation. Write this test before the function:

  ```python
  # two sequences of different real lengths, padded to the same width
  # seq A: 2 residues [[1,1],[3,3]]          -> mean [2.0, 2.0]
  # seq B: 3 residues [[0,0],[3,3],[6,6]]    -> mean [3.0, 3.0]
  # If padding leaks into the mean, A becomes [1.333, 1.333] and this fails.
  ```

  Hand-compute the numbers, watch the assertion fail against the unwritten
  function, then implement. The pad-leak case must be *distinguishable* by the
  test, which is why the two sequences have different lengths.
- **WS-B**, the `.npy` header parser. Byte-literal header in the test, expected
  dtype and shape asserted, before any parsing code.
- **WS-C**, sequence validation. `"MKTZAYIA"` returns 422 with a problem-details
  body naming character `Z` at position 4, before the validator exists.
- **WS-D**, recall@k. A five-item toy ranking where you know the answer by
  inspection, before the metric function exists. A metric implemented without a
  test is a benchmark result nobody should believe, including you.

#### Anti-patterns

- Writing the implementation, then generating tests that pass against it.
- Asserting `is not None`, `len() > 0`, or `assert True`. These pass against
  broken code.
- One giant test exercising five behaviors, so a failure does not localize.
- Mocking the thing under test.
- Skipping TDD on "obvious" code. R1 and R2 are both obvious code.

### 4.7 Testing

Practices that sit alongside the TDD cycle in 4.6 rather than inside it.

- Phase task lists write tests last only because prose is linear. They are
  written **first**, per 4.6. A phase whose tests all landed in the final commit
  did not follow this standard.
- WS-B recall tests run against `contracts/fixtures/golden_neighbors.json`.
  Brute force is the oracle; HNSW is measured against it, never the reverse.
- WS-C integration tests assert conformance to `contracts/openapi.yaml`.
- Tests use the 10k dev subset. No test may require the full corpus.
- No network calls in unit tests. Fixtures or fakes.
- Determinism: any index build takes a seed and produces byte-identical output.

### 4.8 Git and GitHub

- Branches: `ws-a/short-description`, `ws-b/...`, one per phase or smaller.
- Conventional commits: `feat(ws-b): add hnsw layer construction`. Scope is the
  workstream. Subject under 72 chars, imperative mood, no trailing period.
- Commit bodies explain *why*, and only when the why is not obvious. Most
  commits need no body.
- Small PRs. Over ~400 changed lines, justify it in the description.
- PR description: what changed, why, how it was verified. Three short sections.
  No screenshots unless the change is visual. No emojis.
- "How it was verified" includes the **RED assertion output** for new behavior,
  per 4.6. One quoted failure is enough; it is the evidence the test can fail.
- Never force-push a branch someone else may have checked out.
- Never commit: `data/`, `.env`, `*.npy`, `*.parquet`, model weights, AWS
  credentials, `.terraform/`, `terraform.tfstate`.
- Rebase your workstream branch on `main` before opening a PR.

### 4.9 Token discipline for agent sessions

These exist because four parallel sessions burn context fast.

- Never `cat` a file over 200 lines. Use `rg` with context, or `sed -n 'X,Yp'`.
- Never print an entire `.npy`, `.parquet`, or `.json` data file. Print shape,
  dtype, and 3 rows.
- Pipe verbose commands through filters. `cargo test 2>&1 | tail -30`,
  `terraform plan | grep -E '^  [+~-]'`. Do not paste 500 lines of green output.
- Do not re-read a file you just wrote to confirm the write succeeded.
- Do not restate the plan before each step. Do the step.
- When a command fails, read the error, fix it, move on. Do not narrate.
- Keep replies short. The user is running four terminals; they are skimming.

## 5. Build plan

### 5.1 How the parallelism works

Four workstreams run in four terminals, in four git worktrees, each on its own
branch. They do not block each other because every cross-stream interface was
frozen in `contracts/` before any implementation started, and because each
stream ships a **stub or exact-but-slow implementation first**, then optimizes.

That is the core trick. WS-B's Phase B1 is a brute-force index that is correct
and slow. It exists so WS-C can integrate a real, working index on day one
instead of waiting two weeks for HNSW. WS-C's Phase C1 is a stubbed API that
returns fixture data, so the web UI and eval harness have something to call.

```
                    PHASE 0  contracts frozen  [DONE]
                              |
        +---------------+-----+-----------+---------------+
        |               |                 |               |
      WS-A            WS-B              WS-C            WS-D
   data+embed      index core        search svc      infra+eval
        |               |                 |               |
       A1 ingest       B1 brute force    C1 api stub     D1 aws bootstrap
        |               |    \            |    /          |
       A2 embed        B2 hnsw  \        C2 encoder      D2 core infra
        |               |        \        |   /           |
       A3 onnx  -------------------------->  /            D3 fargate+alb
        |               B3 quantize     C3 wire index     |
       A4 fixtures ---->|                 |               D4 batch gpu
        |               B4 bench          C4 harden       |
       A5 full run      |                 |               D5 eval harness
        |               B5 publish        C5 web ui       |
        |                                                 D6 ci/cd
        +--------------------- SYNC ---------------------+
                              |
                       D7 benchmark report
                              |
                          MVP DONE
```

Hard dependencies, the only four that exist:

| Blocker | Blocks | Mitigation so nobody waits |
|---|---|---|
| A3 ONNX export | C2 real query encoder | C2 starts with a fake encoder returning a fixed vector |
| A4 golden fixtures | B4 recall benchmarks | B builds and tunes HNSW on synthetic vectors first |
| B1 brute force | C3 index wiring | C1 stub returns fixture hits until B1 lands |
| D2 ECR | C4 image push | C4 builds and runs the image locally first |

Everything else is genuinely independent. If you find yourself blocked on
something not in this table, that is a signal the contracts were wrong. Fix the
contract in a PR rather than working around it.

### 5.2 Running the four sessions

```bash
for w in a b c d; do
  git worktree add -b ws-$w/main ../esm2-search-$w main
done
```

The `-b` is required: these branches do not exist yet, and without it git
rejects `ws-a/main` as an invalid reference.

Open a terminal per worktree and start a Claude Code session in each with:
"Read CLAUDE.md, docs/workstreams/WS-A.md, and MEMORY.md. You own WS-A. Start at
phase A1."

Rules for parallel sessions:

- Only touch paths your workstream owns. `contracts/` is read-only for everyone
  except in an explicit contract-change PR.
- Append to your own section of `MEMORY.md` only. Never edit another stream's
  section; that is what makes this file merge cleanly across four branches.
- Update CLAUDE.md in the same PR that makes it wrong, per section 8.1. Never
  work around a stale instruction silently: three other sessions are reading it.
- Rebase on `main` before every PR. Merge to `main` often; long-lived branches
  are where parallel work goes to die.
- If you need something from another stream that does not exist yet, write the
  stub yourself against the contract, in your own directory. Do not reach into
  their tree.
- Crate-specific Rust dependencies go in that crate's own `Cargo.toml`, never in
  the root `[workspace.dependencies]` table. WS-B and WS-C both add dependencies
  and appending to the same table conflicts on every merge. Only genuinely
  shared deps live at the root, and adding one is called out in the PR.
- `Cargo.lock` will conflict whenever two streams add dependencies. It is
  generated, so do not hand-merge it: `git checkout --ours Cargo.lock`, then
  `cargo check --workspace`, then commit the regenerated lock.
- `data/` is per-worktree and gitignored. Each session embeds its own dev
  subset, or symlink one shared copy: `ln -s ~/esm2-search-data data`.

---

### 5.3 Phase index

Full task detail for each phase lives in `docs/workstreams/WS-<X>.md`. Each
session reads CLAUDE.md plus its own workstream file, and nothing else. This
split exists because four sessions each carrying every other stream's task list
is pure context cost with no benefit.

**WS-A: Data and embedding (Python).** Owns `pipeline/`, `contracts/fixtures/`.
Runs on the M3 Pro.

| Phase | Deliverable | Unblocks |
|---|---|---|
| A1 | Swiss-Prot ingest, filtering, `meta.parquet`, stratified 10k dev subset | A2 |
| A2 | ESM-2 embedding on MPS, length bucketing, masked mean pooling, sharded resume, measured throughput | A3, A4 |
| A3 | ONNX export of the query encoder, parity verified above 0.9999 | **C2** |
| A4 | Brute-force validation, EC-agreement quality gate, golden fixtures | **B4** |
| A5 | Full 571k corpus run, CUDA container for Batch, artifact in S3 | D4 |

**WS-B: Index core (Rust library).** Owns `crates/esm2-search-index/`. No HTTP, no AWS.

| Phase | Deliverable | Unblocks |
|---|---|---|
| B1 | done, see MEMORY.md | **C3, critical path** |
| B2 | HNSW construction, versioned serialization, mmap load | B3 |
| B3 | int8 quantization, memory-versus-recall table | B4 |
| B4 | Recall and latency benchmarks, the recall-versus-latency curve | D7 |
| B5 | Rustdoc, README, optional crates.io publish | |

**WS-C: Search service (Rust binary).** Owns `crates/esm2-search-server/`, `web/`.

| Phase | Deliverable | Unblocks |
|---|---|---|
| C1 | axum app, all OpenAPI routes stubbed, problem-details errors, middleware | **C5, D5** |
| C2 | ONNX query encoder, Rust tokenizer, cross-language parity test | C3 |
| C3 | Index wired, real hits, honest readiness endpoint, S3 artifact fetch | C4 |
| C4 | Hardened image, graceful shutdown, rate limiting, metrics, load test | D3 |
| C5 | Vite + React demo UI on Cloudflare Pages | |

**WS-D: AWS infrastructure and evaluation.** Owns `infra/`, `eval/`,
`.github/workflows/`. Budget ceiling 50 USD, see section 6.

| Phase | Deliverable | Unblocks |
|---|---|---|
| D1 | Budget alarms **first**, IAM, OIDC, Terraform remote state, teardown doc | D2 |
| D2 | S3, ECR, VPC, IAM task roles. NAT decision recorded | D3, D4 |
| D3 | ECS Fargate, ALB, CloudWatch, scale-to-zero demo scripts | D7 |
| D4 | Batch GPU spot embed job, verified against the local artifact | |
| D5 | SCOPe or Pfam benchmark harness, BLAST and MMseqs2 baselines | D7 |
| D6 | GitHub Actions CI, path-filtered, contract conformance jobs | |
| D7 | Benchmark report, plots, README rewrite | **MVP** |

D5 has no upstream dependency inside this project and can start on day one. It
is also the phase most likely to be underestimated, because of open question Q2
in section 7.2. Start it early.

## 6. Cost allocation

Ceiling: **50 USD total**, not per month. Everything below is approximate,
region-dependent, and changes without notice. Verify against the AWS pricing
calculator before provisioning anything; do not treat these numbers as current.

### 6.1 Where the money goes

| Item | Rate (approx) | Planned usage | Est. total | Notes |
|---|---|---|---|---|
| ECS Fargate | ~0.04 USD/hr for 1 vCPU + 4 GB | demo sessions only, ~150 hr | 6 USD | 24/7 would be ~29 USD/mo. Scale to zero. |
| Application Load Balancer | ~0.023 USD/hr + LCU | only while demoing | 4-16 USD | **Charges hourly even at zero traffic.** See 6.3. |
| AWS Batch, GPU spot | ~0.30-0.50 USD/hr, g5.xlarge spot | 1 test + 1 full run + 1 retry, ~5 hr | 3-8 USD | Interruptible; shard resume makes this safe |
| S3 storage | ~0.023 USD/GB/mo | ~8 GB (embeddings, index, ONNX) | under 1 USD | Negligible. Lifecycle-expire old versions. |
| ECR | ~0.10 USD/GB/mo | ~1 GB of images | under 1 USD | Lifecycle policy keeps last 5 |
| CloudWatch Logs | ~0.50 USD/GB ingested | low traffic, 7-day retention | 1-2 USD | A debug log loop can spike this. See 6.4. |
| ~~DynamoDB (TF lock)~~ | ~~on-demand~~ | none | 0 USD | Struck: Terraform 1.15 deprecates `dynamodb_table`; state locks natively in S3 via `use_lockfile`. No table exists. |
| Data transfer out | first 100 GB/mo free | demo traffic | 0 USD | |
| ACM certificate | free | 1 cert | 0 USD | |
| NAT Gateway | ~0.045 USD/hr + per-GB | **none** | 0 USD | ~32 USD/mo if provisioned. See 6.2. |
| Route 53 hosted zone | 0.50 USD/mo | optional | 0-1 USD | Use Cloudflare DNS instead, free |
| **Total** | | | **15-35 USD** | Leaves headroom for mistakes |

Non-AWS costs are zero: Cloudflare Pages free tier hosts the UI, and GitHub
Actions is free for public repositories. **Make the repo public.** It is free CI
and it is the entire point of building a portfolio project.

Local embedding on the M3 Pro costs nothing but electricity and two nights.
That is deliberate: the expensive path is the one we use once, to prove the
pipeline is portable, not the one we depend on.

### 6.2 The two things that will blow the budget

Everything else on that table is rounding error. These are not.

**NAT Gateway.** Roughly 32 USD/month, billed hourly whether or not a byte
flows through it. The default "VPC with private subnets" Terraform pattern
provisions one automatically and it is the single most common way a student AWS
project quietly spends 100 USD. For this project the Fargate task needs outbound
access to pull from ECR and S3, which you can get three ways without a NAT:
run the task in a public subnet with a public IP and a restrictive security
group; or use VPC gateway and interface endpoints for S3 and ECR; or accept the
NAT only while it is running and destroy it after. WS-D picks one in D2 and
records the decision. Default if undecided: public subnet, no NAT.

**Leaving Fargate and the ALB running.** Together roughly 45 USD/month at idle.
`scripts/demo_down.sh` is not a nice-to-have; it is what keeps this project
inside its ceiling. Run it every time you finish working. Consider making it
part of the phase-completion checklist in section 9.

### 6.3 A cheaper endpoint than the ALB

The ALB exists to terminate TLS and provide a stable hostname. You already know
Cloudflare, and a Cloudflare Tunnel from the Fargate task gives you both for
free, dropping ~16 USD/month and removing the ALB from the architecture
entirely. The tradeoffs: you lose native ALB health checks and CloudWatch target
metrics, and "ECS behind an ALB" is the more conventional thing to be able to
discuss in an interview.

Recommendation: build the ALB path first because it is the standard pattern and
the resume value is real, then switch to a tunnel if the budget tightens. WS-D
records the decision in `docs/adr/`. This is open question Q5 in section 7.

### 6.4 Guardrails

1. AWS Budget with alerts at 10, 25, and 40 USD, created **before** the first
   billable resource. Phase D1, task 1. Non-negotiable.
2. Batch compute environment has a hard max vCPU cap. An unbounded compute
   environment plus a retrying job is how an afternoon costs 200 USD.
3. Every CloudWatch log group has explicit retention. The default is "never
   expire" and it bills forever.
4. Log at INFO in production. A per-request DEBUG log at any real traffic level
   turns CloudWatch into a line item you will notice.
5. Check Cost Explorer at every sync point. Log the running total in MEMORY.md.
   A surprise at 45 USD is a project-ending event; a surprise at 12 USD is a
   Tuesday.
6. `docs/aws_teardown.md` stays current. If you cannot destroy it in one
   documented sequence, you do not know what you are paying for.

---

## 7. Risks and open questions

Living section. Every entry is either closed with a decision or carried forward.
Review at every sync point. An entry that has sat unchanged for two weeks is
either not a real risk or is being avoided; decide which.

### 7.1 Risk register

Impact and likelihood are High / Medium / Low. Owner is the workstream that
resolves it, not the one that discovers it.

| # | Risk | L | I | Owner | Mitigation |
|---|---|---|---|---|---|
| R1 | Masked mean pooling implemented incorrectly; embeddings silently degraded | M | **H** | WS-A | Test-first per 4.6, hand-computed, in A2; EC-agreement gate in A4 catches it downstream |
| R2 | Rust and Python encoders diverge; every result subtly wrong | M | **H** | WS-C | Cross-language parity test in CI, cosine > 0.9999, phase C2 |
| R3 | ONNX export of ESM-2 fails or produces wrong outputs | **M** | M | WS-A | Parity check gates it; FastAPI sidecar fallback documented in A3. Timebox to 3 days. |
| R4 | ESM-2 fp16 on MPS produces numerically wrong results | L | **H** | WS-A | Validate a 500-protein sample against CPU fp32 before the full run |
| R5 | MPS throughput far below estimate; local full-corpus run infeasible | M | M | WS-A | A2 task 7 measures it first; fall back to Batch GPU entirely |
| R6 | Fargate task cannot hold index + ONNX model in memory | **M** | M | WS-B | int8 quantization in B3; mmap the graph; size the task from real RSS |
| R7 | Self-implemented HNSW takes far longer than budgeted | **M** | M | WS-B | Hard decision point, see Q1. Wrap `hnsw_rs` and move on. |
| R8 | Embeddings do not beat MMseqs2 on the benchmark | M | M | WS-D | Report honestly. A clean negative result is still a real result, and is stated as acceptable in section 1. |
| R9 | SCOPe domains do not map cleanly onto Swiss-Prot entries | **H** | **H** | WS-D | See Q2. This is the largest open methodological hole in the project. |
| R10 | Spot interruption mid-embed loses hours of GPU work | M | L | WS-A | Shard checkpointing; resume skips completed shards |
| R11 | Budget overrun | L | **H** | WS-D | Section 6.4 guardrails |
| R12 | Parallel workstreams drift from the frozen contracts | M | M | all | Contract conformance tests in CI; sync points |
| R13 | Truncation at 1022 residues degrades long proteins | **H** | L | WS-A | Measured, not fixed: report the affected fraction and their recall separately |
| R14 | UniProt release changes mid-project; artifacts not comparable | L | M | WS-A | Release string pinned in the manifest; never mix releases in one benchmark |

R1, R2, and R9 are the ones to actually worry about. R1 and R2 are silent
correctness failures that present as mediocre quality rather than as bugs, which
means they can survive to the end of the project and invalidate the benchmark.
R9 is a design hole that could invalidate the benchmark's premise.

### 7.2 Open questions

Each carries a decision deadline and a default. **If the deadline passes without
a decision, the default takes effect automatically and is recorded in
MEMORY.md.** Undecided questions are more expensive than wrong decisions here.

**Q1. Implement HNSW or wrap an existing crate?**
Implementing it is the stronger portfolio signal and is what an interviewer will
actually probe. Wrapping is faster and lower risk. Decide by: end of B2 week 1.
Owner: WS-B. Default: implement, but hard-stop and wrap `hnsw_rs` if recall@10
against synthetic ground truth is not above 0.95 after five working days.

**Q2. What is the benchmark corpus, exactly?**
SCOPe domains are structural domains extracted from PDB entries. They are not
full-length Swiss-Prot proteins. Three options: (a) embed SCOPe ASTRAL sequences
as their own separate corpus and benchmark within it, (b) map SCOPe domains to
their parent Swiss-Prot entries and accept the resulting noise, (c) use a
different remote-homology benchmark such as Pfam clan membership over Swiss-Prot
directly. Option (a) is cleanest methodologically and keeps the benchmark
self-contained; option (c) reuses the corpus we already embedded.
**This blocks D5 and must be settled before any evaluation code is written.**
Decide by: start of D5. Owner: WS-D. Default: (a), a separate SCOPe corpus,
with Pfam-over-Swiss-Prot as a secondary check.

**Q3. Scalar int8 quantization or product quantization?**
Decide by: end of B3. Owner: WS-B. Default: scalar int8. Only escalate to PQ if
scalar costs more than 2 points of recall@10.

**Q4. Truncate or chunk proteins over 1022 residues?**
Truncation is simple and loses the C-terminus. Chunking with averaged embeddings
preserves more but changes the pooling semantics and breaks parity with the
query encoder unless both do it. Decide by: end of A2. Owner: WS-A.
Default: truncate, and report the affected fraction (R13).

**Q5. ALB or Cloudflare Tunnel for the public endpoint?**
See section 6.3. Decide by: start of D3. Owner: WS-D. Default: ALB, with a
documented tunnel fallback if spend exceeds 30 USD.

**Q6. Public subnet, VPC endpoints, or NAT?**
See section 6.2. Decide by: start of D2. Owner: WS-D. Default: public subnet
with a restrictive security group, no NAT.

**Q7. Publish `esm2-search-index` to crates.io?**
A published crate is a materially stronger artifact than a directory in a
monorepo, but it implies some ongoing maintenance. Decide by: B5.
Owner: WS-B. Default: publish.

**Q9. Does search need metadata filtering (by organism, EC class, length)?**
Real users want it. It complicates the index and the API. Decide by: end of C3.
Owner: WS-C. Default: no filtering in MVP; it is listed in section 11 non-goals
and moves to post-MVP.

### 7.3 Closed questions

Move entries here with the decision, the date, and the reasoning. Never delete
them. A record of why something was decided is worth more than the tidiness of
a short document, and it stops the same debate from recurring in a later session.

| Date | Question | Decision | Why |
|---|---|---|---|
| 2026-08-24 | Corpus for MVP | Swiss-Prot, reviewed only | Curated EC and GO labels give a free evaluation set |
| 2026-08-24 | Serving platform | ECS Fargate, not Lambda | Index must stay resident; Lambda cold starts would reload it |
| 2026-08-24 | IaC tool | Terraform, not CDK | Portable skill, transfers beyond AWS |
| 2026-08-24 | Frontend scope | Minimal single-page demo | A clickable link matters; a full dashboard competes with the engineering |
| 2026-08-24 | Q8, AWS region | us-west-2 | The Open Data genomics tiebreaker in the original question does not apply: every corpus we consume (UniProt, SCOPe, ESM-2 weights) comes from its own host, not an AWS Open Data bucket, so there is no cross-region transfer to match. Decided instead on g5 spot availability for D4 and proximity to Berkeley for demo latency |
| 2026-08-24 | Terraform state locking | S3 native `use_lockfile`, no DynamoDB table | Terraform 1.15 deprecates the S3 backend's `dynamodb_table`; native locking removes a resource, an IAM permission, a teardown step, and a cost line |

---

## 8. Maintaining this document

CLAUDE.md is the single source of truth for how this project is built. It is
read at the start of every session in all four worktrees, which means a stale
instruction here propagates into four parallel workstreams before anyone
notices. Treat it as production configuration, not as notes.

### 8.1 The core rule

**When reality and this document disagree, one of them changes in the same pull
request that created the disagreement.** Not later, not in a cleanup pass. If
you discover mid-phase that a task listed here is wrong, impossible, or
unnecessary, fix the text here as part of the work. An agent that silently works
around a stale instruction has created a bug that will surface in a different
terminal, days later, with no trail back to its cause.

### 8.2 Precedence

When sources conflict, resolve in this order:

1. `contracts/` — frozen interfaces win over everything
2. `CLAUDE.md` — this document
3. `MEMORY.md` — a historical record, not an instruction
4. Anything an agent remembers from earlier in a session

If 1 and 2 conflict, that is a bug in this document and it is fixed immediately,
not worked around.

### 8.3 What triggers an update

- A phase completes. Run `scripts/phase_done.py`, per section 9.5. It retires the
  phase from 5.3 and from your workstream file and writes the MEMORY.md entry.
  Then check that the phases after it still describe the work that remains.
- An open question in 7.2 is decided. Move it to 7.3 with the reasoning.
- A new risk becomes apparent. Add it to 7.1. Do not wait for it to materialize.
- A measured number arrives (throughput, recall, memory, cost). Estimates in
  this document are replaced with measurements, and the estimate is struck
  rather than deleted so the gap between plan and reality stays visible.
- A contract changes. The contract PR must also update every reference here.
- A standard proves wrong or unenforceable. Amend it rather than routinely
  violating it. A standard nobody follows is worse than no standard.

### 8.4 Consistency checks

Run at every sync point. Cheap, and catches most drift:

1. Does every phase referenced in the dependency table in 5.1 still exist with
   the same identifier?
2. Do the paths in the repository tree in section 2 match `git ls-files`?
3. Does every dependency in the section 5.1 table still have a stated
   mitigation, and is that mitigation still true?
4. Does `contracts/openapi.yaml` match what the server actually serves?
5. Do the cost estimates in section 6 match the last Cost Explorer reading
   logged in MEMORY.md?
6. Are there open questions in 7.2 past their decision deadline? Apply the
   default and record it.
7. Is the definition of done in section 10 still an accurate description of
   finished?

### 8.5 Size discipline

This file is loaded into context in four sessions continuously, so its length is
a recurring cost paid on every turn. Ceiling: **900 lines.** Past that, move the
per-phase task detail in section 5 into `docs/workstreams/WS-A.md` and friends,
leave a phase list plus the dependency table here, and have each session read
only its own workstream file. Sections 1 through 4 and 6 through 10 stay here
permanently; they are what every session needs.

Prefer editing existing text over appending. This document grows by accretion if
nobody prunes it, and a 2,000-line CLAUDE.md is one that agents skim rather than
follow.

---

## 9. Progress log (MEMORY.md)

`MEMORY.md` at the repository root is the shared, committed record of what has
actually been completed. It exists so that a session starting cold in any of the
four worktrees can learn the current state of the project in a few hundred
tokens rather than by reading four branches of git history.

### 9.1 When to write an entry

Write one when:

- A **phase** completes. Always. One entry.
- A **task** produces a durable number (throughput, recall, latency, memory,
  cost) or a decision that outlives the task. These are exactly what a future
  session needs and cannot recover cheaply.
- An **open question** from 7.2 is decided, or a default takes effect.
- A **risk** materializes, or is closed out.
- A **sync point** is reached, including the running AWS spend.

Do not write an entry for: routine commits, work in progress, refactors with no
external effect, or anything already obvious from the file tree. This log is
read by every future session; noise in it is a tax on all of them.

### 9.2 Format

One or two lines per entry. Never a paragraph.

```
- [WS-B / B2] 2026-09-14 - HNSW build lands, recall@10 0.97 at ef_search=100
  on synthetic ground truth. Self-implemented, Q1 closed. `crates/esm2-search-index/src/hnsw.rs`
```

Prefix is `[WS-X / PhaseId]`, or `[SYNC]`, or `[DECISION]`, or `[RISK]`. Then
the ISO date, then what happened, then the number or the path if there is one.
Include the number. "Embedding pipeline works" is worth nothing to a future
session; "8.4 proteins/sec on M3 Pro fp16, 19h projected for 571k" is worth a
great deal.

### 9.3 Avoiding merge conflicts

MEMORY.md is sectioned by workstream, and **each session appends only to its own
section.** Four agents appending to one shared list would conflict on every
merge; four agents appending to four disjoint sections almost never will.

If you do hit a conflict in this file, the resolution is always to keep both
sides. Nothing in this log is mutually exclusive.

### 9.4 Relationship to this document

MEMORY.md records what happened. CLAUDE.md describes what to do. They are not
interchangeable and neither replaces the other. Do not put plans in MEMORY.md,
and do not put a running history in CLAUDE.md beyond the closed-questions table
in 7.3.

### 9.5 Retiring a completed phase

The moment a phase is done, before you open the PR, run:

```bash
scripts/phase_done.py B1 "brute force exact index lands, recall@10 1.0 vs the
  numpy oracle on the 10k dev subset, 41 ms p95. crates/esm2-search-index/src/brute.rs"
```

This is not optional and it is not a cleanup pass. Planning text that describes
work already finished is the specific failure mode section 8.1 exists to
prevent: three other sessions read this document and will act on it.

The script makes three edits:

1. Collapses the phase's row in 5.3 to `done, see MEMORY.md`. The identifier and
   the "unblocks" column survive deliberately, because the dependency table in
   5.1 and consistency check 8.4 #1 resolve against them. A phase row deleted
   outright turns "B1 unblocks C3" into a dangling reference in a terminal that
   is not yours.
2. Deletes the phase's task detail from `docs/workstreams/WS-<X>.md`, leaving a
   pointer. That block is the real token cost, and it is dead weight once the
   work exists in git.
3. Appends the summary to your own section of MEMORY.md in the 9.2 format.

It refuses a summary with no number in it, for the reason given in 9.2, and
refuses to retire a phase twice. What it cannot do is judge: moving a decided
question from 7.2 to 7.3, striking a superseded estimate, and updating the risk
register in 7.1 are still yours, and the script prints that reminder.

MEMORY.md is the reference for anything retired this way. It is append-only and
never pruned; CLAUDE.md is what shrinks.


---

## 10. Definition of done for the MVP

- Full Swiss-Prot embedded, artifact in S3, checksummed, contract-validated.
- HNSW index with a published recall-versus-latency curve.
- Public HTTPS API conforming to `contracts/openapi.yaml`.
- Latency, stated separately and honestly, because they differ by two orders of
  magnitude and conflating them would be a false claim:
  - **index search** p95 under 20 ms
  - **end-to-end query** p95 under 3 s, dominated by ESM-2 CPU encoding
  The headline number is the end-to-end one. Encoding, not search, is the
  bottleneck, and the README says so.
- Public demo UI that takes a sequence and returns hits.
- Benchmark report comparing against BLAST and MMseqs2 on remote homology, with
  a protocol fixed in writing before results were seen.
- One `terraform apply` reproduces the infrastructure; one teardown script
  destroys it.
- CI green on every check.
- Every deterministic module was built test-first per 4.6, with the RED output
  recorded in its PR.
- Total AWS spend under 50 USD.
- CLAUDE.md passes the section 8.4 consistency check and MEMORY.md records every
  completed phase.

---

## 11. Non-goals for the MVP

Explicitly out of scope. Do not let these expand the project.

- Structure prediction or structure-based search. Foldseek exists.
- Fine-tuning ESM-2. We use the pretrained checkpoint.
- Corpora beyond Swiss-Prot. UniRef and metagenomic data are post-MVP.
- Metadata filtering by organism, EC, or length. See Q9.
- User accounts, saved searches, persistent history.
- Multi-region, autoscaling, or high availability.
- A general-purpose vector database. This index is purpose-built and stays that way.
