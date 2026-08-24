terraform {
  required_version = "~> 1.15"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.0"
    }
  }

  # Intentionally no backend block. This module creates the S3 bucket and
  # DynamoDB table that every other root module uses as remote state, so it
  # cannot depend on them itself. State for this module stays local
  # (infra/bootstrap/terraform.tfstate, gitignored) and is applied by hand,
  # rarely, by whoever holds admin credentials. See README.md.
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project   = "esm2-protein-search"
      ManagedBy = "terraform"
      Module    = "bootstrap"
    }
  }
}
