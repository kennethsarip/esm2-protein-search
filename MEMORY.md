# Progress log

What has actually been completed. Append-only. Read this to learn the current
state of the project without reading four branches of git history.

Rules are in CLAUDE.md section 9. In short: one or two lines per entry, include
the measured number, append only to your own workstream's section, and on a
merge conflict keep both sides.

Prefixes: `[WS-X / PhaseId]`, `[SYNC]`, `[DECISION]`, `[RISK]`, `[COST]`.

---

## Shared

- [SYNC] 2026-08-24 - Phase 0 complete. Repo scaffolded, Cargo workspace builds,
  contracts frozen in `contracts/{embeddings.md,index-api.md,openapi.yaml}`.
- [DECISION] 2026-08-24 - Four parallel workstreams, monorepo with per-stream
  directory ownership, git worktrees. Rationale in CLAUDE.md section 5.1.
- [DECISION] 2026-08-24 - Swiss-Prot corpus, ESM-2 650M, Terraform + ECS Fargate,
  minimal demo UI, 50 USD budget ceiling. See CLAUDE.md section 7.3.
- [DECISION] 2026-08-24 - TDD is mandatory for deterministic code (CLAUDE.md 4.6).
  RED output required in every PR description. Characterization tests, not TDD,
  for model-output quality; benchmarks, not assertions, for latency.
- [DECISION] 2026-08-24 - Per-phase task detail moved to `docs/workstreams/WS-*.md`
  to keep CLAUDE.md under the 900-line ceiling from section 8.5. CLAUDE.md 881 lines.
- [DECISION] 2026-08-24 - Completed phases are retired from CLAUDE.md 5.3 and from
  the workstream file by `scripts/phase_done.py`, summarized here instead. See 9.5.

## WS-A: Data and embedding

_No entries yet. First entry expected at A1 completion._

## WS-B: Index core

- [WS-B / B1] 2026-08-24 - 17/17 tests pass across npy parsing, corpus
  validation, and brute-force top-k. Self-query returns rank 1 at score 1.0 (tol
  1e-4) on a synthetic 50-row corpus; real dev10k pending WS-A.
  crates/esm2-search-index/src/{npy,corpus,brute}.rs

## WS-C: Search service

_No entries yet. First entry expected at C1 completion._

## WS-D: Infrastructure and evaluation

- [WS-D / D1] 2026-08-24 - D1 written but NOT complete. `infra/bootstrap` and
  `infra/envs/dev` pass `validate` and `fmt` on Terraform 1.15.8 / AWS provider
  6.61.0. Blocked on an AWS account: no `apply`, no tested budget alert.
- [DECISION] 2026-08-24 - Q8 closed, region us-west-2. The Open Data genomics
  tiebreaker did not apply; no corpus we use (UniProt, SCOPe, ESM-2 weights)
  lives in an AWS Open Data bucket. Decided on g5 spot availability and demo
  latency from Berkeley instead. CLAUDE.md 7.3.
- [DECISION] 2026-08-24 - Terraform state locks via S3 `use_lockfile`, not a
  DynamoDB table: Terraform 1.15 deprecates the backend's `dynamodb_table`.
  Removes 1 resource, 3 IAM actions, and a teardown step. WS-D.md D1 task 3
  and CLAUDE.md 6.1 amended to match.
- [RISK] 2026-08-24 - The GitHub OIDC provider pins no `thumbprint_list`. AWS
  trusts this issuer via its own trust store, and a pinned thumbprint breaks
  on GitHub's next cert rotation. Revisit only if `AssumeRoleWithWebIdentity`
  starts failing on trust, not on the role policy.
- [DECISION] 2026-08-24 - Q2 closed, benchmark corpus is SCOPe ASTRAL 40 as its
  own self-contained corpus; a positive is same superfamily and different
  family, same-family candidates are dropped from the pool. Pfam clans over
  Swiss-Prot as the secondary check. `eval/PROTOCOL.md`.
- [RISK] 2026-08-24 - R9 downgraded from H/H to L/H by that decision. No SCOPe
  domain to Swiss-Prot mapping happens any more, so the hole it described
  cannot open. Residual risk lives only in the Pfam secondary check.
- [WS-D / D5] 2026-08-24 - Protocol fixed in writing before any results exist,
  per D5 task 5, headline metric pre-registered as recall@10. Metrics built
  test-first: 7/7 pass on hand-computed values, recall@1 = 1/3 and AP = 0.7556
  on a 5-item toy ranking. `eval/`
- [RISK] 2026-08-24 - Cross-stream dependency not in the CLAUDE.md 5.1 table:
  D5's esm2 arm needs SCOPe embeddings and so waits on WS-A's embed pipeline.
  The BLAST and MMseqs2 arms and all scoring are unblocked and proceed now.

- [WS-D / D5] 2026-08-25 - SCOPe 2.08 ASTRAL-40 pinned and loaded: 15177
  domains, 13275 queryable, 2065 superfamilies, 4703 families, median 23
  positives per query, max 371. sha256 e6d3213b. Scoring the full protocol is
  13275 x 15177 = 201M pairs per method, which sizes the runners.
  `eval/src/esm2_search_eval/{scope,corpus}.py`
- [WS-D / D5] 2026-08-25 - ASTRAL ships entirely lowercase. The loader
  uppercases every sequence: BLAST treats lowercase as soft-masked, so passing
  it through would have handed BLAST a masked database while the other methods
  saw the real one, violating PROTOCOL.md section 5.
- [RISK] 2026-08-25 - R13 measured on the benchmark corpus: 11 of 15177 SCOPe
  domains (0.07%) exceed 1022 residues, so truncation is negligible here. This
  says nothing about Swiss-Prot; WS-A still measures that separately.
- [WS-D / D5] 2026-08-25 - 1909 of 13275 queries have more than 100 positives,
  so recall@100 is capped below 1.0 for 14% of the query set. Anticipated by
  PROTOCOL.md section 4 and identical for every method; noted so a reader of
  the D7 table does not read the ceiling as a method failure.

## Cost tracking

Log the Cost Explorer running total at every sync point. CLAUDE.md section 6.4.

| Date | Running total | Note |
|---|---|---|
| 2026-08-24 | 0.00 USD | No billable resources provisioned yet |
