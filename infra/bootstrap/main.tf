# Terraform remote state. CLAUDE.md D1 task 3. Chicken-and-egg case: this
# module's own state is local.
#
# No DynamoDB lock table. Terraform 1.15 deprecates the S3 backend's
# dynamodb_table parameter in favour of native locking via S3 conditional
# writes (use_lockfile), so consumers lock against an object in this bucket.
# Versioning below is what makes that safe, and is required either way.

resource "aws_s3_bucket" "tfstate" {
  bucket        = var.state_bucket_name
  force_destroy = var.state_bucket_force_destroy
}

resource "aws_s3_bucket_versioning" "tfstate" {
  bucket = aws_s3_bucket.tfstate.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "tfstate" {
  bucket = aws_s3_bucket.tfstate.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "tfstate" {
  bucket                  = aws_s3_bucket.tfstate.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Cost guardrail: CLAUDE.md D1 task 1, non-negotiable, before any other
# billable resource. Alerts fire on ACTUAL spend at each absolute USD
# threshold, not on a percentage of the monthly limit.

resource "aws_budgets_budget" "project_ceiling" {
  name         = "esm2-protein-search-monthly"
  budget_type  = "COST"
  limit_amount = tostring(var.budget_limit_usd)
  limit_unit   = "USD"
  time_unit    = "MONTHLY"

  dynamic "notification" {
    for_each = var.budget_alert_thresholds_usd
    content {
      comparison_operator        = "GREATER_THAN"
      threshold                  = notification.value
      threshold_type             = "ABSOLUTE_VALUE"
      notification_type          = "ACTUAL"
      subscriber_email_addresses = [var.budget_alert_email]
    }
  }

  # Forecast alert on the full limit so an overrun is visible before it happens,
  # not just after each absolute threshold is crossed.
  notification {
    comparison_operator        = "GREATER_THAN"
    threshold                  = 100
    threshold_type             = "PERCENTAGE"
    notification_type          = "FORECASTED"
    subscriber_email_addresses = [var.budget_alert_email]
  }
}

# GitHub Actions OIDC: no long-lived access keys in CI. CLAUDE.md D1 task 2.

# No thumbprint_list: AWS validates this specific issuer against its own trust
# store rather than a pinned certificate, and a pinned thumbprint is a live
# outage waiting for GitHub's next cert rotation.
resource "aws_iam_openid_connect_provider" "github_actions" {
  url            = "https://token.actions.githubusercontent.com"
  client_id_list = ["sts.amazonaws.com"]
}

data "aws_iam_policy_document" "github_actions_trust" {
  statement {
    effect  = "Allow"
    actions = ["sts:AssumeRoleWithWebIdentity"]

    principals {
      type        = "Federated"
      identifiers = [aws_iam_openid_connect_provider.github_actions.arn]
    }

    condition {
      test     = "StringEquals"
      variable = "token.actions.githubusercontent.com:aud"
      values   = ["sts.amazonaws.com"]
    }

    condition {
      test     = "StringLike"
      variable = "token.actions.githubusercontent.com:sub"
      values   = ["repo:${var.github_org}/${var.github_repo}:*"]
    }
  }
}

resource "aws_iam_role" "github_actions" {
  name               = "esm2-protein-search-github-actions"
  assume_role_policy = data.aws_iam_policy_document.github_actions_trust.json
}

# Least privilege for what exists as of D1: the state bucket only. D2 and D6
# attach additional scoped policies (ECR push, S3 artifact access, ECS deploy)
# as those resources come into being. Do not widen this here.
data "aws_iam_policy_document" "github_actions_state_access" {
  statement {
    effect    = "Allow"
    actions   = ["s3:ListBucket"]
    resources = [aws_s3_bucket.tfstate.arn]
  }

  # DeleteObject is what releases a use_lockfile lock. Without it a failed
  # plan leaves a stale .tflock that blocks every subsequent run.
  statement {
    effect    = "Allow"
    actions   = ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"]
    resources = ["${aws_s3_bucket.tfstate.arn}/*"]
  }
}

resource "aws_iam_role_policy" "github_actions_state_access" {
  name   = "terraform-state-access"
  role   = aws_iam_role.github_actions.id
  policy = data.aws_iam_policy_document.github_actions_state_access.json
}
