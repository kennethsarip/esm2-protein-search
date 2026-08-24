# AWS teardown

Exact commands to destroy everything this project has provisioned. Updated in
the same PR that adds a new resource, per CLAUDE.md section 8.1. If this
document cannot destroy everything in one documented sequence, we do not know
what we are paying for (CLAUDE.md section 6.4, guardrail 6).

Run this whenever you finish a working session that provisioned anything
beyond the D1 bootstrap. `scripts/demo_down.sh` (added in D3) is the fast path
for the Fargate/ALB pair specifically; this document is the complete sequence.

## Current scope (as of D1)

Only the bootstrap resources exist: the Terraform remote state bucket, the
GitHub Actions OIDC provider and role, and the account budget. None of these
carry a meaningful ongoing cost. This section will grow
as D2 (storage/network/ECR), D3 (Fargate/ALB), and D4 (Batch) land — each
phase's PR adds its own teardown steps below, in dependency order, before the
bootstrap step.

## Teardown order

Destroy in the reverse of the dependency order in CLAUDE.md section 5.1: envs
before bootstrap, because envs' remote state lives in the bucket bootstrap
created.

### 1. Dev environment (D2 onward)

```
cd infra/envs/dev
terraform destroy
```

Confirm in the console that nothing remains: no ECS services with a desired
count above zero, no ALB, no NAT gateway, no running Batch compute
environment. `terraform destroy` should catch all of these since they are all
Terraform-managed, but confirm directly against the console once — a manual
`aws` console change that drifted from state will not be destroyed by
Terraform.

### 2. Bootstrap (remote state, OIDC role, budget)

Only tear this down if abandoning the project entirely. It has no ongoing
cost worth reclaiming (an S3 bucket holding a few state files, an IAM role,
and a budget are all effectively free), and losing it means losing the state
history for everything above.

```
cd infra/bootstrap
```

The state bucket is versioned and `force_destroy` defaults to `false`, so a
plain `terraform destroy` will fail on it with objects still present. To
actually remove it:

```
terraform apply -var-file=example.tfvars -var="state_bucket_force_destroy=true"
terraform destroy -var-file=example.tfvars
```

This deletes the bucket, all its object versions (the entire state history),
the GitHub Actions OIDC provider and role, and the budget. There is no undo.

### 3. Things Terraform does not manage

- The personal admin IAM user and its MFA device (created by hand in D1, see
  `infra/bootstrap/README.md`). Delete manually in the IAM console if
  abandoning the account.
- CloudWatch log groups sometimes outlive their creating resource if a
  destroy was interrupted. Check the CloudWatch console for any group
  prefixed `esm2-protein-search` after step 1.
- ECR images do not block bucket/role deletion but do carry their own small
  storage cost (CLAUDE.md section 6.1); the ECR repos themselves are
  Terraform-managed and go with step 1.

## After teardown

Check AWS Cost Explorer once, a day or two later, to confirm charges have
stopped accruing (billing has latency; a same-day check can look clean while
a charge is still in flight). Log the reading in `MEMORY.md`'s cost table,
per CLAUDE.md section 6.4 guardrail 5.
