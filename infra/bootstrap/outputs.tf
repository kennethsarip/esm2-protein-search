output "state_bucket_name" {
  value = aws_s3_bucket.tfstate.id
}

output "aws_region" {
  value = var.aws_region
}

output "github_actions_role_arn" {
  description = "Pass to GitHub Actions as the role to assume via OIDC. CLAUDE.md D6."
  value       = aws_iam_role.github_actions.arn
}

output "budget_name" {
  value = aws_budgets_budget.project_ceiling.name
}
