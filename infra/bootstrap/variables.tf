variable "aws_region" {
  description = "Single region for the whole project. See CLAUDE.md open question Q8."
  type        = string
  default     = "us-west-2"
}

variable "state_bucket_name" {
  description = "S3 bucket for Terraform remote state. Must be globally unique."
  type        = string
  default     = "esm2-protein-search-tfstate"
}

variable "state_bucket_force_destroy" {
  description = "Allow `terraform destroy` to delete the state bucket even if it holds objects. Leave false; flip to true only during teardown, per docs/aws_teardown.md."
  type        = bool
  default     = false
}

variable "budget_limit_usd" {
  description = "Monthly budget ceiling. CLAUDE.md section 6 sets a 50 USD total project ceiling, not per month; this is set to the same figure as a monthly cap so a runaway resource cannot silently exceed the project ceiling within a single month."
  type        = number
  default     = 50
}

variable "budget_alert_thresholds_usd" {
  description = "Absolute USD thresholds (not percentages) at which the budget emails the alert address. CLAUDE.md section 6.4."
  type        = list(number)
  default     = [10, 25, 40]
}

variable "budget_alert_email" {
  description = "Email address for budget alerts."
  type        = string
  default     = "kenneth.sarip@berkeley.edu"
}

variable "github_org" {
  description = "GitHub org/user that owns the repo, for the OIDC trust condition."
  type        = string
}

variable "github_repo" {
  description = "GitHub repo name, for the OIDC trust condition."
  type        = string
}
