# Workstream D

Detail extracted from CLAUDE.md section 5 per the size discipline rule in
CLAUDE.md section 8.5. Read CLAUDE.md first, then this file. Sections 1-4 and
6-11 of CLAUDE.md apply to this workstream in full.

### WS-D: AWS infrastructure and evaluation

Owns `infra/`, `eval/`, `.github/workflows/`. Budget ceiling: 50 USD total.

**Phase D1: AWS bootstrap and cost guardrails** (do the guardrails first)

1. AWS account, then immediately: enable Cost Explorer, create a Budget with
   alerts at 10, 25, and 40 USD to the Berkeley email address.
   **Do this before creating a single billable resource.** A misconfigured Batch
   job that retries in a loop can spend the entire budget in an afternoon.
2. IAM: an admin user for yourself with MFA, an OIDC role for GitHub Actions.
   Never long-lived access keys in CI. Never the root account for daily work.
3. Terraform remote state: S3 bucket with versioning and encryption, plus
   DynamoDB state locking. Bootstrap this by hand or in a separate root module;
   it is the one chicken-and-egg case.
4. Pin the AWS provider version. Pin Terraform. Commit `.terraform.lock.hcl`.
5. Single region, `us-east-1` or `us-west-2`. Check which one hosts the AWS Open
   Data genomics buckets you might use later and match it; cross-region S3
   transfer is a silent cost.
6. Write `docs/aws_teardown.md`: the exact commands to destroy everything.
   Write it now, while the infrastructure is small enough to enumerate.

Done when: `terraform plan` runs against remote state and a budget alert has
been tested.

**Phase D2: Core infrastructure**

1. `modules/storage`: S3 buckets for artifacts and ONNX models. Versioning on,
   public access blocked, SSE-S3, lifecycle rule expiring old versions after 30
   days.
2. `modules/network`: VPC, two public and two private subnets across AZs, one
   NAT gateway. **NAT gateway is roughly 32 USD/month, the largest line item in
   this budget.** Either run Fargate in public subnets with a security group and
   no NAT, or destroy the NAT between sessions. Document the choice.
3. ECR repositories for the server and the embed job, with lifecycle policies
   keeping the last 5 images.
4. IAM task roles, least privilege: the service reads exactly two S3 prefixes.
5. `terraform fmt`, `validate`, and `tflint` clean. Add them to CI.

Done when: `terraform apply` in `envs/dev` creates storage, network, and
registry, and the teardown doc actually works.

**Phase D3: Fargate service**

1. ECS cluster, task definition with the WS-C image. Start at 1 vCPU / 4 GB and
   size down from the C4 load test results.
2. ALB with an HTTPS listener, ACM certificate, health check on `/v1/health`
   with a startup grace period long enough for index load.
3. CloudWatch log group with a 7-day retention. Default retention is forever and
   it costs money.
4. CloudWatch alarms: 5xx rate, p99 latency, task restart count.
5. Deploy circuit breaker with automatic rollback enabled.
6. Scale to zero when not demoing. Write `scripts/demo_up.sh` and
   `scripts/demo_down.sh` that set desired count to 1 and 0. This is what keeps
   the project inside 50 USD.

Done when: a public HTTPS endpoint serves real queries and can be torn down and
brought back with one script each.

**Phase D4: Batch GPU embedding job**

1. Batch compute environment, SPOT, `g5.xlarge` or `g4dn.xlarge`, max vCPUs
   capped so a runaway cannot scale out. Min vCPUs zero.
2. Job definition using WS-A's `Dockerfile.gpu`, with S3 input and output paths.
3. Retry policy: 2 attempts, and handle spot interruption by resuming from
   shards rather than restarting the corpus.
4. **Test with a 1,000-protein job first.** Confirm it completes and the output
   lands in S3 before launching the full run.
5. Run the full corpus once. Expect 30 minutes to 2 hours on an A10G, 5 to 15
   USD. Verify the output matches the local artifact: same shape, same ids, and
   cosine similarity above 0.999 per row against the MPS-produced embeddings.
   A float16 accumulation difference across devices is expected and fine; a
   correlation below 0.99 means a real bug.
6. Set the compute environment to zero and confirm no instances persist.

Done when: the full artifact was produced on Batch, verified against local, and
the environment is scaled to zero.

**Phase D5: Evaluation harness** (independent, can start immediately)

1. Download SCOPe (ASTRAL 40 percent identity subset). Build remote-homology
   pairs: same superfamily, different family. That distinction is the entire
   point of the benchmark, so get the parsing right and test it.
2. Install BLAST+ and MMseqs2 locally. Build both databases over the same
   corpus. Same inputs for every method or the comparison is meaningless.
3. Runners for three methods: BLAST, MMseqs2, and esm2-protein-search via the HTTP API.
4. Metrics: recall@{1,10,100}, mean average precision, ROC AUC for the
   same-superfamily decision, and wall-clock latency per query.
5. **Fix the evaluation protocol in writing before looking at any results.**
   Query set, candidate pool, positive definition, and the exact metric. Then
   run it. Choosing a metric after seeing results is how honest people produce
   dishonest benchmarks.
6. Sensitivity analysis: does the ranking of methods hold across sequence length
   bins and superfamily sizes?
7. Report per-method compute cost: index build time, query latency, memory.

Done when: one command produces a metrics table for all three methods.

**Phase D6: CI/CD**

1. GitHub Actions, path-filtered so a Python change does not run the Rust suite.
2. Rust job: `fmt --check`, `clippy -D warnings`, `test`, build.
3. Python job: `ruff`, `mypy`, `pytest`.
4. Terraform job: `fmt --check`, `validate`, `tflint`, and `plan` on PRs with
   the plan posted as a PR comment.
5. Contract job: validate the OpenAPI spec, run the Rust-Python embedding parity
   test, run `validate_artifact.py` against the dev fixture.
6. On merge to `main`: build and push the image to ECR via OIDC. Do not
   auto-deploy; deployment stays manual to protect the budget.
7. Cache aggressively. `Swatinem/rust-cache`, uv cache. Unoptimized Rust CI will
   burn your Actions minutes.

**Phase D7: Benchmark report** (the deliverable that matters)

1. `docs/benchmark.md`: method, corpus, hardware, protocol, results, honest
   limitations.
2. The two plots that carry the project: recall-versus-latency across methods,
   and recall by superfamily size.
3. A limitations section that a skeptical reviewer would write about your work.
   Include it even when it weakens the headline. Especially then.
4. Fold the headline number into the repository README, with the methodology
   linked directly beside it.
5. Rewrite the README as the front door: what it is, the number, a live demo
   link, an architecture diagram, and a reproduction guide.

