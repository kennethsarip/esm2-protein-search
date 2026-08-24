# Bootstrap

Creates the two things every other root module needs before it can exist: the
Terraform remote state bucket and the GitHub Actions OIDC role. Also creates
the account-wide budget guardrail. Applied once, by hand, by whoever holds
admin credentials. Not run by CI.

State locks natively in S3 via the backend's `use_lockfile`, so there is no
DynamoDB table; Terraform 1.15 deprecates the `dynamodb_table` parameter that
would have created one.

## Manual prerequisites (cannot be done from Terraform or by an agent)

1. Create the AWS account, or confirm one exists.
2. In the IAM console, create a personal admin IAM user (not the root
   account for daily work) and enroll an MFA device on it. This is an
   interactive TOTP enrollment; there is no API for it that would make
   sense to run from code you also control.
3. Configure local credentials for that user, e.g. `aws configure` or
   `aws configure sso`. Confirm with:
   ```
   aws sts get-caller-identity
   ```
4. Confirm Cost Explorer is enabled in the Billing console (Billing
   Preferences -> Cost Explorer). It can take up to 24 hours to populate
   after first enabling it.

Only after those four are done does `terraform apply` in this directory make
sense.

## Applying

```
cd infra/bootstrap
terraform init
terraform plan  -var-file=example.tfvars
terraform apply -var-file=example.tfvars
```

State for this module stays local (`terraform.tfstate` here, gitignored).
That is deliberate, not an oversight: this module creates the remote
backend, so it cannot use it.

## After applying

1. Note the `github_actions_role_arn` output. It goes into the GitHub repo
   as the role GitHub Actions assumes via OIDC (CLAUDE.md D6).
2. Test the budget alert actually reaches the inbox: in the Billing
   console, open the budget and use "Send test notification", or wait for
   real spend to cross the first threshold. D1 is not done until this has
   been verified once, per CLAUDE.md.
3. Every other root module (`infra/envs/dev`, and any future env) points
   its S3 backend at the `state_bucket_name` output here. Backend blocks
   cannot interpolate, so that value is duplicated literally in each env's
   `versions.tf`; if you ever change the bucket name, grep for it.

## Changing the GitHub Actions IAM policy

The role created here can only touch the state bucket and lock table,
intentionally. As D2 and D6 create S3 artifact buckets, ECR repos, and ECS
resources, attach additional scoped `aws_iam_role_policy` resources to
`aws_iam_role.github_actions` in the *env* modules that own those
resources, not by widening the policy here. Keeps blast radius per-policy
reviewable.
