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

_No entries yet. First entry expected at B1 completion. B1 is the critical path._

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

## Cost tracking

Log the Cost Explorer running total at every sync point. CLAUDE.md section 6.4.

| Date | Running total | Note |
|---|---|---|
| 2026-08-24 | 0.00 USD | No billable resources provisioned yet |
