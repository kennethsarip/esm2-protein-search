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
  to keep CLAUDE.md under the 900-line ceiling from section 8.5. CLAUDE.md 717 lines.

## WS-A: Data and embedding

_No entries yet. First entry expected at A1 completion._

## WS-B: Index core

_No entries yet. First entry expected at B1 completion. B1 is the critical path._

## WS-C: Search service

_No entries yet. First entry expected at C1 completion._

## WS-D: Infrastructure and evaluation

_No entries yet. First entry expected at D1 completion, which must include the
budget alarm confirmation._

## Cost tracking

Log the Cost Explorer running total at every sync point. CLAUDE.md section 6.4.

| Date | Running total | Note |
|---|---|---|
| 2026-08-24 | 0.00 USD | No billable resources provisioned yet |
